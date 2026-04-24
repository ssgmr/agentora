//! Agent 决策循环
//!
//! 每个 Agent 独立 task，在同一个 task 内顺序完成：读取状态 → 决策 → 应用动作 → 推送 delta
//!
//! ## 6 阶段流水线
//!
//! 1. **WorldState 构建** — 使用 WorldStateBuilder 自动构建 WorldState
//! 2. **感知摘要构建** — 使用 PerceptionBuilder 构建感知摘要
//! 3. **决策阶段** — LLM 或规则引擎决策
//! 4. **应用阶段** — 执行动作并更新 World
//! 5. **Delta 发送** — 使用 DeltaEmitter 发送状态变更
//! 6. **叙事发送** — 使用 NarrativeEmitter 发送叙事事件

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use std::collections::HashMap;

use crate::{World, AgentId, Action, ActionType};
use crate::decision::{DecisionPipeline, infer_state_mode, PerceptionBuilder};
use crate::simulation::{WorldStateBuilder, Delta, DeltaEmitter, NarrativeEmitter, MemoryRecorder};
use crate::snapshot::NarrativeEvent;

impl Default for super::delta::DeltaEnvelope {
    fn default() -> Self {
        use super::delta::ChangeHint;
        use crate::snapshot::AgentState;
        Self {
            delta: Delta::AgentStateChanged {
                agent_id: String::new(),
                state: AgentState {
                    id: String::new(),
                    name: String::new(),
                    position: (0, 0),
                    health: 0,
                    max_health: 0,
                    satiety: 0,
                    hydration: 0,
                    age: 0,
                    level: 0,
                    is_alive: true,
                    inventory_summary: std::collections::HashMap::new(),
                    current_action: String::new(),
                    action_result: String::new(),
                    reasoning: None,
                },
                change_hint: ChangeHint::Spawned,
            },
            source_peer_id: None,
            tick: 0,
        }
    }
}

/// Agent 同步决策+执行循环
/// 每个 Agent 独立 task，在同一个 task 内顺序完成：读取状态 → LLM 决策 → 应用动作 → 推送 delta
pub async fn run_agent_loop(
    world: Arc<Mutex<World>>,
    agent_id: AgentId,
    pipeline: Arc<DecisionPipeline>,
    delta_tx: Sender<Delta>,
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
        // 暂停检查：跳过决策
        if is_paused.load(Ordering::SeqCst) {
            tracing::trace!("[AgentLoop] Agent {:?} 暂停中，跳过决策", agent_id);
            continue;
        }

        // ===== 阶段 1: WorldState 构建 =====
        let (agent_clone, world_state, memory_summary_opt) = {
            let w = world.lock().await;

            // 检查 Agent 是否存活
            let agent = match w.agents.get(&agent_id) {
                Some(a) if a.is_alive => a,
                _ => {
                    tracing::warn!("[AgentLoop] Agent {:?} 已死亡或不存在，退出循环", agent_id);
                    return;
                }
            };

            // 使用 WorldStateBuilder 自动构建 WorldState
            let ws = match WorldStateBuilder::build(&w, &agent_id, vision_radius) {
                Some(state) => state,
                None => return, // Agent 已死亡或不存在
            };

            // 在持有锁时读取记忆摘要（此时 MemorySystem 的 DB 连接可用）
            let spark_type = infer_state_mode(&ws);
            let summary = agent.memory.get_summary(spark_type);
            let mem_summary = if summary.is_empty() { None } else { Some(summary) };

            tracing::debug!("[AgentLoop] Agent {:?} vision: {} terrain, {} resources, {} agents, {} structures, {} legacies",
                agent_id, ws.terrain_at.len(), ws.resources_at.len(), ws.nearby_agents.len(), ws.nearby_structures.len(), ws.nearby_legacies.len());

            (agent.clone(), ws, mem_summary)
        };

        // ===== 阶段 2: 感知摘要构建（锁外 I/O） =====
        let perception_summary = PerceptionBuilder::build_perception_summary(&world_state);

        // ===== 阶段 3: 决策阶段 =====
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
                    params: candidate.params.into_iter().map(|(k, v): (_, _)| (k, v.to_string())).collect(),
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
            let action_feedback = agent_clone.last_action_result.as_deref();
            let start = std::time::Instant::now();
            let result = pipeline.execute(&agent_clone.id, &world_state, &perception_summary, memory_summary_opt.as_deref(), action_feedback).await;
            let elapsed = start.elapsed().as_secs_f32();

            if result.error_info.is_some() {
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
                    params: candidate.params.into_iter().map(|(k, v): (_, _)| (k, v.to_string())).collect(),
                    build_type: None,
                    direction: None,
                }), None)
            }
        };

        // ===== 阶段 4-6: 应用动作 + 发送 Delta + 发送叙事 =====
        if let Some(action) = action {
            let events = {
                let mut w = world.lock().await;

                // 阶段 4: 应用动作
                w.apply_action(&agent_id, &action);

                // 阶段 5: 记录记忆（使用 MemoryRecorder）
                MemoryRecorder::record(&mut w, &agent_id, &action);

                // 阶段 6a: 提取叙事事件（使用 NarrativeEmitter）
                let events = NarrativeEmitter::extract(&w);
                w.tick_events.clear();

                // 阶段 6b: 发送 Delta（使用 DeltaEmitter）
                DeltaEmitter::emit_all(&delta_tx, &w, &agent_id, &action, &events);

                events
            };

            // 阶段 6c: 发送叙事事件（锁外）
            NarrativeEmitter::send_events(&narrative_tx, events);
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