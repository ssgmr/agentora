//! 记忆记录器
//!
//! 从 Agent 动作提取记忆事件并记录到 Agent 的记忆系统。
//! 从 agent_loop.rs 迁移，实现职责单一化。

use crate::world::World;
use crate::types::{AgentId, ActionType, Action};
use crate::memory::MemoryEvent;

/// 记忆记录器
pub struct MemoryRecorder;

impl MemoryRecorder {
    /// 记录动作到 Agent 记忆系统
    ///
    /// # 参数
    /// - `world`: 世界状态可变引用
    /// - `agent_id`: Agent ID
    /// - `action`: 执行的动作
    ///
    /// # 返回
    /// 记录是否成功
    pub fn record(world: &mut World, agent_id: &AgentId, action: &Action) -> bool {
        // 验证 Agent 存在
        if !world.agents.contains_key(agent_id) {
            tracing::warn!("[MemoryRecorder] Agent {:?} 不存在", agent_id);
            return false;
        }

        let action_type_str = format!("{:?}", action.action_type);
        let (emotion_tags, importance) = Self::get_emotion_and_importance(&action.action_type);

        let event = MemoryEvent {
            tick: world.tick as u32,
            event_type: action_type_str,
            content: action.reasoning.clone(),
            emotion_tags,
            importance,
        };

        if let Some(agent_mut) = world.agents.get_mut(agent_id) {
            agent_mut.memory.record(&event);
            true
        } else {
            false
        }
    }

    /// 根据动作类型获取情感标签和重要性评分
    ///
    /// # 参数
    /// - `action_type`: 动作类型
    ///
    /// # 返回
    /// (情感标签列表, 重要性评分)
    fn get_emotion_and_importance(action_type: &ActionType) -> (Vec<String>, f32) {
        match action_type {
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
        }
    }
}