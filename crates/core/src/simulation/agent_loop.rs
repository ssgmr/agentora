//! Agent 决策循环
//!
//! 每个 Agent 独立 task，在同一个 task 内顺序完成：读取状态 → 决策 → 应用动作 → 推送 delta

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use std::collections::HashMap;

use crate::{World, AgentId, Action, ActionType, ResourceType};
use crate::decision::{DecisionPipeline, infer_state_mode};
use crate::rule_engine::WorldState;
use crate::world::vision::scan_vision;
use crate::memory::MemoryEvent;
use super::AgentDelta;

/// 叙事事件（推送至前端）
#[derive(Debug, Clone)]
pub struct NarrativeEvent {
    pub tick: u64,
    pub agent_id: String,
    pub agent_name: String,
    pub event_type: String,
    pub description: String,
    pub color_code: String,
}

/// Agent 同步决策+执行循环
/// 每个 Agent 独立 task，在同一个 task 内顺序完成：读取状态 → LLM 决策 → 应用动作 → 推送 delta
pub async fn run_agent_loop(
    world: Arc<Mutex<World>>,
    agent_id: AgentId,
    pipeline: Arc<DecisionPipeline>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    is_npc: bool,
    interval_secs: u32,
    vision_radius: u32,
    is_paused: Arc<AtomicBool>,
) {
    tracing::info!("[AgentLoop] Agent {:?} 启动 (is_npc={}, interval={}s, vision_radius={})", agent_id, is_npc, interval_secs, vision_radius);

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs as u64));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        // 暂停检查：跳过决策
        if is_paused.load(Ordering::SeqCst) {
            tracing::trace!("[AgentLoop] Agent {:?} 暂停中，跳过决策", agent_id);
            continue;
        }

        // 检查 Agent 是否存活
        let should_continue = {
            let w = world.lock().await;
            match w.agents.get(&agent_id) {
                Some(agent) => agent.is_alive,
                None => false,
            }
        };

        if !should_continue {
            tracing::warn!("[AgentLoop] Agent {:?} 已死亡或不存在，退出循环", agent_id);
            break;
        }

        // 构建 WorldState（锁内纯计算 + 锁外 I/O）
        let (agent_clone, world_state) = {
            let w = world.lock().await;

            let vision = scan_vision(&w, &agent_id, vision_radius);

            let agent = match w.agents.get(&agent_id) {
                Some(a) => a.clone(),
                None => break,
            };

            tracing::debug!("[AgentLoop] Agent {:?} vision: {} terrain, {} resources, {} agents, {} structures, {} legacies",
                agent_id, vision.terrain_at.len(), vision.resources_at.len(), vision.nearby_agents.len(), vision.nearby_structures.len(), vision.nearby_legacies.len());

            let ws = WorldState {
                map_size: 256,
                agent_position: agent.position,
                agent_inventory: agent.inventory.iter().map(|(k, v)| {
                    let resource = match k.as_str() {
                        "iron" => ResourceType::Iron,
                        "food" => ResourceType::Food,
                        "wood" => ResourceType::Wood,
                        "water" => ResourceType::Water,
                        "stone" => ResourceType::Stone,
                        _ => ResourceType::Food,
                    };
                    (resource, *v)
                }).collect(),
                agent_satiety: agent.satiety,
                agent_hydration: agent.hydration,
                terrain_at: vision.terrain_at,
                self_id: agent_id.clone(),
                existing_agents: w.agents.keys().cloned().collect(),
                resources_at: vision.resources_at,
                nearby_agents: vision.nearby_agents,
                nearby_structures: vision.nearby_structures,
                nearby_legacies: vision.nearby_legacies,
                active_pressures: w.pressure_pool.iter().map(|p| p.description.clone()).collect(),
                last_move_direction: agent.last_position.and_then(|last_pos| {
                    crate::world::vision::calculate_direction(&last_pos, &agent.position)
                }),
                temp_preferences: agent.temp_preferences.iter()
                    .map(|p| (p.key.clone(), p.boost, p.remaining_ticks))
                    .collect(),
                agent_personality: Some(agent.personality.clone()),
            };

            (agent, ws)
        };

        // 锁外 I/O：获取记忆摘要
        let memory_summary_opt = {
            let spark_type = infer_state_mode(&world_state);
            let summary = agent_clone.memory.get_summary(spark_type);
            if summary.is_empty() { None } else { Some(summary) }
        };

        tracing::debug!("[AgentLoop] Agent {:?} ({}) 开始决策{}", agent_id.as_str(), agent_clone.name,
            if is_npc { " (NPC 规则决策)" } else { "" });

        let (action, validation_failure): (Option<Action>, Option<String>) = if is_npc {
            // NPC：规则引擎生存兜底（不调用 LLM）
            use crate::rule_engine::RuleEngine;
            let engine = RuleEngine::new();
            if let Some(candidate) = engine.survival_fallback(&world_state) {
                (Some(Action {
                    reasoning: candidate.reasoning,
                    action_type: candidate.action_type,
                    target: candidate.target,
                    params: candidate.params.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
                    build_type: None,
                    direction: None,
                }), None)
            } else {
                (Some(Action {
                    reasoning: "NPC 无明确目标，等待".to_string(),
                    action_type: ActionType::Wait,
                    target: None,
                    params: HashMap::new(),
                    build_type: None,
                    direction: None,
                }), None)
            }
        } else {
            // Player Agent：LLM 决策
            let _ = agent_clone.last_action_type.as_deref(); // 保留占位，待后续使用
            let action_feedback = agent_clone.last_action_result.as_deref();
            let start = std::time::Instant::now();
            let result = pipeline.execute(&agent_clone.id, &world_state, memory_summary_opt.as_deref(), action_feedback).await;
            let elapsed = start.elapsed().as_secs_f32();

            if result.error_info.is_some() {
                // 校验失败：不执行动作，记录反馈让 LLM 下回合修正
                let vf = result.validation_failure.clone();
                if let Some(ref msg) = vf {
                    tracing::warn!("[AgentLoop] Agent {:?} ({}) 决策被拒绝 (耗时 {:.1}s): {}",
                        agent_id.as_str(), agent_clone.name, elapsed, msg);
                    eprintln!("[决策被拒绝] {} (耗时 {:.1}s): {}", agent_clone.name, elapsed, msg);
                }
                (None, vf)
            } else {
                let candidate = result.selected_action.expect("决策成功但 selected_action 为 None");
                tracing::info!("[AgentLoop] Agent {:?} ({}) 决策完成 (耗时 {:.1}s): {:?}",
                    agent_id.as_str(), agent_clone.name, elapsed, candidate.action_type);
                eprintln!("[{}] {} (耗时 {:.1}s): {:?}", agent_clone.name, "决策完成", elapsed, candidate.action_type);
                eprintln!("[{}] reasoning: {}", agent_clone.name, candidate.reasoning);

                (Some(Action {
                    reasoning: candidate.reasoning,
                    action_type: candidate.action_type,
                    target: candidate.target,
                    params: candidate.params.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
                    build_type: None,
                    direction: None,
                }), None)
            }
        };

        // 应用动作并发送 delta/narrative（同一个锁内完成，确保位置一致性）
        if let Some(action) = action {
            let events = {
                let mut w = world.lock().await;
                // 不在这里推进tick，tick由独立的tick_loop统一推进
                w.apply_action(&agent_id, &action);

                // 记录到 Agent 记忆系统
                if let Some(_agent) = w.agents.get(&agent_id) {
                    let action_type_str = format!("{:?}", action.action_type);
                    let (emotion_tags, importance) = match action.action_type {
                        ActionType::MoveToward { .. } => (vec!["purposeful".to_string()], 0.3),
                        ActionType::Gather { .. } => (vec!["satisfied".to_string()], 0.4),
                        ActionType::Wait => (vec!["resting".to_string()], 0.1),
                        ActionType::Eat => (vec!["satisfied".to_string()], 0.3),
                        ActionType::Drink => (vec!["refreshed".to_string()], 0.3),
                        ActionType::Attack { .. } => (vec!["aggressive".to_string(), "angry".to_string()], 0.8),
                        ActionType::Talk { .. } => (vec!["social".to_string()], 0.5),
                        ActionType::Build { .. } => (vec!["creative".to_string()], 0.6),
                        ActionType::Explore { .. } => (vec!["curious".to_string()], 0.5),
                        ActionType::TradeOffer { .. } | ActionType::TradeAccept { .. } => (vec!["cooperative".to_string()], 0.6),
                        ActionType::AllyPropose { .. } | ActionType::AllyAccept { .. } => (vec!["trust".to_string(), "bonding".to_string()], 0.7),
                        ActionType::InteractLegacy { .. } => (vec!["reverent".to_string()], 0.7),
                        _ => (vec!["unknown".to_string()], 0.3),
                    };

                    let event = MemoryEvent {
                        tick: w.tick as u32,
                        event_type: action_type_str,
                        content: action.reasoning.clone(),
                        emotion_tags,
                        importance,
                    };

                    if let Some(agent_mut) = w.agents.get_mut(&agent_id) {
                        agent_mut.memory.record(&event);
                    }
                }

                // 提取叙事事件
                let events: Vec<NarrativeEvent> = w.tick_events.drain(..).map(|e| NarrativeEvent {
                    tick: e.tick,
                    agent_id: e.agent_id,
                    agent_name: e.agent_name,
                    event_type: e.event_type,
                    description: e.description,
                    color_code: e.color_code,
                }).collect();

                // 构建 delta 事件
                let delta: Option<AgentDelta> = match w.agents.get(&agent_id) {
                    Some(agent) if agent.is_alive => {
                        Some(AgentDelta::AgentMoved {
                            id: agent.id.as_str().to_string(),
                            name: agent.name.clone(),
                            position: (agent.position.x, agent.position.y),
                            health: agent.health,
                            max_health: agent.max_health,
                            is_alive: true,
                            age: agent.age,
                        })
                    }
                    Some(agent) => {
                        Some(AgentDelta::AgentDied {
                            id: agent.id.as_str().to_string(),
                            name: agent.name.clone(),
                            position: (agent.position.x, agent.position.y),
                            age: agent.age,
                        })
                    }
                    None => None,
                };

                // Tier 2: 基于动作类型生成额外 delta
                let mut extra_deltas: Vec<AgentDelta> = Vec::new();
                match &action.action_type {
                    ActionType::Build { structure } => {
                        if let Some(agent) = w.agents.get(&agent_id) {
                            extra_deltas.push(AgentDelta::StructureCreated {
                                x: agent.position.x,
                                y: agent.position.y,
                                structure_type: format!("{:?}", structure),
                                owner_id: agent_id.as_str().to_string(),
                            });
                        }
                    }
                    ActionType::Gather { resource } => {
                        if let Some(agent) = w.agents.get(&agent_id) {
                            if let Some(node) = w.resources.get(&agent.position) {
                                extra_deltas.push(AgentDelta::ResourceChanged {
                                    x: agent.position.x,
                                    y: agent.position.y,
                                    resource_type: resource.as_str().to_string(),
                                    amount: node.current_amount,
                                });
                            }
                        }
                    }
                    ActionType::TradeAccept { .. } => {
                        if let Some(event) = events.iter().find(|e| e.event_type == "trade") {
                            extra_deltas.push(AgentDelta::TradeCompleted {
                                from_id: agent_id.as_str().to_string(),
                                to_id: "unknown".to_string(),
                                items: event.description.clone(),
                            });
                        }
                    }
                    ActionType::AllyAccept { .. } => {
                        if events.iter().any(|e| e.event_type == "ally") {
                            extra_deltas.push(AgentDelta::AllianceFormed {
                                id1: agent_id.as_str().to_string(),
                                id2: "unknown".to_string(),
                            });
                        }
                    }
                    _ => {}
                }

                // 发送 delta
                if let Some(delta) = delta {
                    if let Err(e) = delta_tx.send(delta) {
                        tracing::error!("[AgentLoop] delta 发送失败: {:?}", e);
                    }
                    for extra in extra_deltas {
                        if let Err(e) = delta_tx.send(extra) {
                            tracing::error!("[AgentLoop] extra delta 发送失败: {:?}", e);
                        }
                    }
                }

                events
            };

            // 发送叙事事件（锁外）
            for event in events {
                tracing::info!("[Narrative] tick={} {}: {}", event.tick, event.event_type, event.description);
                let _ = narrative_tx.send(event);
            }
        } else if !is_npc {
            // Player Agent 决策被拒绝，写入 last_action_result 供下次决策使用
            if let Some(ref vf) = validation_failure {
                let mut w = world.lock().await;
                if let Some(agent) = w.agents.get_mut(&agent_id) {
                    agent.last_action_result = Some(format!("[错误] 上次决策被拒绝：{}", vf));
                }
                tracing::info!("[AgentLoop] Agent {:?} LLM 校验失败反馈已记录: {}", agent_id, vf);
            }
        }

        interval.tick().await;
    }
}