//! Delta 发射器
//!
//! 从 Agent 动作结果构建 Delta 事件并发送到 delta channel。
//! 使用统一的 AgentState 和简化后的 Delta（AgentStateChanged + WorldEvent）。

use crate::world::World;
use crate::types::{AgentId, ActionType, Action};
use crate::snapshot::{AgentState, NarrativeEvent};
use super::delta::{Delta, ChangeHint, WorldEvent};
use std::sync::mpsc::Sender;

/// Delta 发射器
pub struct DeltaEmitter;

impl DeltaEmitter {
    /// 构建并发送 Agent 状态 delta
    ///
    /// 使用统一的 AgentState 结构
    pub fn emit_agent_state(
        delta_tx: &Sender<Delta>,
        world: &World,
        agent_id: &AgentId,
        change_hint: ChangeHint,
        reasoning: Option<&str>,
    ) -> usize {
        let delta = match world.agents.get(agent_id) {
            Some(agent) if agent.is_alive => {
                // 使用 Agent::to_state() 构建统一状态
                let state = AgentState {
                    id: agent.id.as_str().to_string(),
                    name: agent.name.clone(),
                    position: (agent.position.x, agent.position.y),
                    health: agent.health,
                    max_health: agent.max_health,
                    satiety: agent.satiety,
                    hydration: agent.hydration,
                    age: agent.age,
                    level: agent.level,
                    is_alive: true,
                    inventory_summary: agent.inventory.clone(),
                    current_action: agent.last_action_type.clone().unwrap_or_default(),
                    action_result: agent.last_action_result.clone().unwrap_or_default(),
                    reasoning: reasoning.map(|s| s.to_string()),
                };
                Some(state.to_delta(change_hint))
            }
            Some(agent) => {
                // 死亡状态
                let state = AgentState {
                    id: agent.id.as_str().to_string(),
                    name: agent.name.clone(),
                    position: (agent.position.x, agent.position.y),
                    health: 0,
                    max_health: agent.max_health,
                    satiety: 0,
                    hydration: 0,
                    age: agent.age,
                    level: agent.level,
                    is_alive: false,
                    inventory_summary: std::collections::HashMap::new(),
                    current_action: String::new(),
                    action_result: String::new(),
                    reasoning: None,
                };
                Some(state.to_delta(ChangeHint::Died))
            }
            None => None,
        };

        let mut sent_count = 0;
        if let Some(d) = delta {
            if let Err(e) = delta_tx.send(d) {
                tracing::error!("[DeltaEmitter] delta 发送失败: {:?}", e);
            } else {
                sent_count += 1;
            }
        }
        sent_count
    }

    /// 根据动作类型生成 WorldEvent delta
    pub fn emit_action_deltas(
        delta_tx: &Sender<Delta>,
        world: &World,
        agent_id: &AgentId,
        action: &Action,
        events: &[NarrativeEvent],
    ) -> usize {
        let mut world_events: Vec<WorldEvent> = Vec::new();

        match &action.action_type {
            ActionType::Build { structure } => {
                if let Some(agent) = world.agents.get(agent_id) {
                    world_events.push(WorldEvent::StructureCreated {
                        pos: (agent.position.x, agent.position.y),
                        structure_type: format!("{:?}", structure),
                        owner_id: agent_id.as_str().to_string(),
                    });
                }
            }
            ActionType::Gather { resource } => {
                if let Some(agent) = world.agents.get(agent_id) {
                    if let Some(node) = world.resources.get(&agent.position) {
                        world_events.push(WorldEvent::ResourceChanged {
                            pos: (agent.position.x, agent.position.y),
                            resource_type: resource.as_str().to_string(),
                            amount: node.current_amount,
                        });
                    }
                }
            }
            ActionType::TradeAccept { .. } => {
                if let Some(event) = events.iter().find(|e| e.event_type == "trade") {
                    world_events.push(WorldEvent::TradeCompleted {
                        from_id: agent_id.as_str().to_string(),
                        to_id: "unknown".to_string(),
                        items: event.description.clone(),
                    });
                }
            }
            ActionType::AllyAccept { .. } => {
                if events.iter().any(|e| e.event_type == "ally") {
                    world_events.push(WorldEvent::AllianceFormed {
                        id1: agent_id.as_str().to_string(),
                        id2: "unknown".to_string(),
                    });
                }
            }
            _ => {}
        }

        let mut sent_count = 0;
        for world_event in world_events {
            let delta = Delta::WorldEvent(world_event);
            if let Err(e) = delta_tx.send(delta) {
                tracing::error!("[DeltaEmitter] world event delta 发送失败: {:?}", e);
            } else {
                sent_count += 1;
            }
        }
        sent_count
    }

    /// 发送叙事事件作为 WorldEvent
    pub fn emit_narratives(
        delta_tx: &Sender<Delta>,
        events: &[NarrativeEvent],
    ) -> usize {
        let mut sent_count = 0;
        for event in events {
            let delta = Delta::WorldEvent(WorldEvent::AgentNarrative {
                narrative: event.clone(),
            });

            if let Err(e) = delta_tx.send(delta) {
                tracing::error!("[DeltaEmitter] narrative delta 发送失败: {:?}", e);
            } else {
                sent_count += 1;
            }
        }
        sent_count
    }

    /// 发送所有 delta（状态 + 动作相关 + 叙事）
    pub fn emit_all(
        delta_tx: &Sender<Delta>,
        world: &World,
        agent_id: &AgentId,
        action: &Action,
        events: &[NarrativeEvent],
    ) -> usize {
        // 根据动作类型判定 change_hint
        let change_hint = match &action.action_type {
            ActionType::MoveToward { .. } => ChangeHint::Moved,
            ActionType::Eat | ActionType::Drink => ChangeHint::ActionExecuted,
            ActionType::Wait => ChangeHint::ActionExecuted,
            _ => ChangeHint::ActionExecuted,
        };

        let reasoning = if action.reasoning.is_empty() {
            None
        } else {
            Some(action.reasoning.as_str())
        };
        let state_count = Self::emit_agent_state(delta_tx, world, agent_id, change_hint, reasoning);
        let action_count = Self::emit_action_deltas(delta_tx, world, agent_id, action, events);
        let narrative_count = Self::emit_narratives(delta_tx, events);
        state_count + action_count + narrative_count
    }
}