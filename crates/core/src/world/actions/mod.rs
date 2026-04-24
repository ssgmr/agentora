//! 动作处理器：每个 ActionType 的独立 handler
//!
//! 设计原则：Handler 只返回 ActionResult（携带执行详情），不设置 last_action_result。
//! 反馈生成统一在 apply_action 中处理，确保格式一致。

pub mod movement;
pub mod survival;
pub mod social;
pub mod legacy;

use crate::types::{ActionType, AgentId, Action};
use crate::world::{ActionResult, World};
use crate::narrative::{NarrativeBuilder, EventType, action_type_display};

impl World {
    /// Action 执行入口：路由 ActionType 到具体 handler（World 职责：协调 + 后处理）
    pub fn execute_action(&mut self, agent_id: &AgentId, action: &Action) -> ActionResult {
        match &action.action_type {
            ActionType::MoveToward { target } => self.handle_move_toward(agent_id, *target),
            ActionType::Gather { resource } => self.handle_gather(agent_id, *resource),
            ActionType::Wait => self.handle_wait(agent_id),
            ActionType::Eat => self.handle_eat(agent_id),
            ActionType::Drink => self.handle_drink(agent_id),
            ActionType::Build { structure } => self.handle_build(agent_id, *structure),
            ActionType::Attack { target_id } => self.handle_attack(agent_id, target_id.clone()),
            ActionType::Talk { message } => self.handle_talk(agent_id, message.clone()),
            ActionType::TradeOffer { offer, want, target_id } => self.handle_trade_offer(agent_id, offer.clone(), want.clone(), target_id.clone()),
            ActionType::TradeAccept { .. } => self.handle_trade_accept(agent_id),
            ActionType::TradeReject { .. } => self.handle_trade_reject(agent_id),
            ActionType::AllyPropose { target_id } => self.handle_ally_propose(agent_id, target_id.clone()),
            ActionType::AllyAccept { ally_id } => self.handle_ally_accept(agent_id, ally_id.clone()),
            ActionType::AllyReject { ally_id } => self.handle_ally_reject(agent_id, ally_id.clone()),
            ActionType::InteractLegacy { legacy_id, interaction } => {
                self.handle_legacy_interaction(agent_id, legacy_id, interaction)
            }
        }
    }

    /// 记录错误叙事（统一入口）
    pub fn record_error_narrative(&mut self, agent_id: &AgentId, action_type: &ActionType, reason: &str) {
        if let Some(agent) = self.agents.get(agent_id) {
            let agent_name = agent.name.clone();
            let builder = NarrativeBuilder::new(agent_name.clone());
            self.record_event(agent_id, &agent_name, EventType::Error.as_str(),
                &builder.error(action_type.clone(), reason), EventType::Error.color_code());
        }
    }

    /// 获取动作类型的中文名称（供 mod.rs 使用）
    pub fn action_type_name(&self, action_type: &ActionType) -> &'static str {
        action_type_display(action_type.clone())
    }
}