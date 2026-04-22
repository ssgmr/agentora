//! Delta 发射器
//!
//! 从 Agent 动作结果构建 AgentDelta 事件并发送到 delta channel。
//! 从 agent_loop.rs 迁移，实现职责单一化。

use crate::world::World;
use crate::types::{AgentId, ActionType, Action};
use super::delta::AgentDelta;
use super::agent_loop::NarrativeEvent;
use std::sync::mpsc::Sender;

/// Delta 发射器
pub struct DeltaEmitter;

impl DeltaEmitter {
    /// 构建并发送 Agent 移动/状态 delta
    ///
    /// # 参数
    /// - `delta_tx`: delta 发送通道
    /// - `world`: 世界状态引用
    /// - `agent_id`: Agent ID
    ///
    /// # 返回
    /// 发送的 delta 数量
    pub fn emit_agent_state(
        delta_tx: &Sender<AgentDelta>,
        world: &World,
        agent_id: &AgentId,
    ) -> usize {
        let delta = match world.agents.get(agent_id) {
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

    /// 根据动作类型生成额外 delta
    ///
    /// # 参数
    /// - `delta_tx`: delta 发送通道
    /// - `world`: 世界状态引用
    /// - `agent_id`: Agent ID
    /// - `action`: 执行的动作
    /// - `events`: 当前 tick 的叙事事件（用于提取交易/结盟信息）
    ///
    /// # 返回
    /// 发送的额外 delta 数量
    pub fn emit_action_deltas(
        delta_tx: &Sender<AgentDelta>,
        world: &World,
        agent_id: &AgentId,
        action: &Action,
        events: &[NarrativeEvent],
    ) -> usize {
        let mut extra_deltas: Vec<AgentDelta> = Vec::new();

        match &action.action_type {
            ActionType::Build { structure } => {
                if let Some(agent) = world.agents.get(agent_id) {
                    extra_deltas.push(AgentDelta::StructureCreated {
                        x: agent.position.x,
                        y: agent.position.y,
                        structure_type: format!("{:?}", structure),
                        owner_id: agent_id.as_str().to_string(),
                    });
                }
            }
            ActionType::Gather { resource } => {
                if let Some(agent) = world.agents.get(agent_id) {
                    if let Some(node) = world.resources.get(&agent.position) {
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

        let mut sent_count = 0;
        for extra in extra_deltas {
            if let Err(e) = delta_tx.send(extra) {
                tracing::error!("[DeltaEmitter] extra delta 发送失败: {:?}", e);
            } else {
                sent_count += 1;
            }
        }
        sent_count
    }

    /// 发送所有 delta（状态 + 动作相关）
    ///
    /// 组合 emit_agent_state 和 emit_action_deltas
    pub fn emit_all(
        delta_tx: &Sender<AgentDelta>,
        world: &World,
        agent_id: &AgentId,
        action: &Action,
        events: &[NarrativeEvent],
    ) -> usize {
        let state_count = Self::emit_agent_state(delta_tx, world, agent_id);
        let action_count = Self::emit_action_deltas(delta_tx, world, agent_id, action, events);
        state_count + action_count
    }
}