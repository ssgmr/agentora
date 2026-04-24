//! 社交相关动作处理器
//!
//! Talk、Attack、TradeOffer/Accept/Reject、AllyPropose/Accept/Reject

use crate::agent::RelationType;
use crate::types::{AgentId, ResourceType};
use crate::world::{ActionResult, World, PendingTrade, TradeStatus};
use crate::narrative::{NarrativeBuilder, EventType};
use std::collections::HashMap;

impl World {
    /// Talk：与附近Agent对话
    pub fn handle_talk(&mut self, agent_id: &AgentId, message: String) -> ActionResult {
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());
        let agent_pos = self.agents.get(agent_id).unwrap().position;

        // World查找附近Agent
        const VISION_RANGE: i32 = 3;
        let nearby_agents: Vec<AgentId> = self.agents.iter()
            .filter(|(id, other)| {
                *id != agent_id && {
                    let dx = (other.position.x as i32 - agent_pos.x as i32).abs();
                    let dy = (other.position.y as i32 - agent_pos.y as i32).abs();
                    dx <= VISION_RANGE && dy <= VISION_RANGE
                }
            })
            .map(|(id, _)| id.clone())
            .collect();

        if nearby_agents.is_empty() {
            self.record_event(agent_id, &agent_name, EventType::Talk.as_str(),
                &builder.talk_self(&message), EventType::Talk.color_code());
            return ActionResult::SuccessWithDetail("talk:self".into());
        }

        let mut affected_names = Vec::new();
        let tick = self.tick as u32;

        for target_id in &nearby_agents {
            let target_name = self.agents.get(target_id).map(|a| a.name.clone()).unwrap_or_default();
            affected_names.push(target_name.clone());

            let target = self.agents.get_mut(target_id).unwrap();
            target.receive_talk(agent_id, &agent_name, &message, tick);
        }

        let initiator = self.agents.get_mut(agent_id).unwrap();
        initiator.talk_with(&nearby_agents, &message, tick);

        let event_msg = builder.talk_to(&affected_names, &message);

        self.record_event(agent_id, &agent_name, EventType::Talk.as_str(), &event_msg, EventType::Talk.color_code());
        ActionResult::SuccessWithDetail(format!("talk:{}", affected_names.join(",")))
    }

    /// Attack：攻击相邻格Agent
    pub fn handle_attack(&mut self, agent_id: &AgentId, target_id: AgentId) -> ActionResult {
        let agent_pos = self.agents.get(agent_id).unwrap().position;
        let (agent_name, target_name) = {
            let agent = self.agents.get(agent_id).unwrap();
            let target_name = self.agents.get(&target_id).map(|a| a.name.clone()).unwrap_or_default();
            (agent.name.clone(), target_name)
        };
        let builder = NarrativeBuilder::new(agent_name.clone());

        if !self.agents.contains_key(&target_id) {
            return ActionResult::Blocked(
                format!("Attack失败：目标Agent {} 不存在", target_id.as_str()));
        }

        if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked(
                format!("Attack失败：目标Agent {} 已死亡", target_name));
        }

        let target_pos = self.agents.get(&target_id).unwrap().position;
        let distance = agent_pos.manhattan_distance(&target_pos);
        if distance > 1 {
            return ActionResult::Blocked(
                format!("Attack失败：目标Agent {} 距离过远（距离{}格）。Attack只能对相邻格Agent执行",
                    target_name, distance));
        }

        let is_ally = self.agents.get(agent_id)
            .and_then(|a| a.relations.get(&target_id))
            .map(|r| r.relation_type == RelationType::Ally)
            .unwrap_or(false);
        if is_ally {
            return ActionResult::Blocked(
                format!("Attack失败：不能攻击盟友Agent {}。若要攻击，需先解除盟约", target_name));
        }

        let damage = 10;

        {
            let target = self.agents.get_mut(&target_id).unwrap();
            target.receive_attack(damage, agent_id);
        }
        {
            let attacker = self.agents.get_mut(agent_id).unwrap();
            attacker.initiate_attack(&target_id);
        }

        let target_alive = self.agents.get(&target_id).map(|a| a.health > 0).unwrap_or(false);
        self.total_attacks += 1;

        if !target_alive {
            self.record_event(agent_id, &agent_name, EventType::Attack.as_str(),
                &builder.attack_defeated(&target_name), EventType::Attack.color_code());
            ActionResult::SuccessWithDetail(format!("attack:{}defeated,damage={}", target_name, damage))
        } else {
            self.record_event(agent_id, &agent_name, EventType::Attack.as_str(),
                &builder.attack_hit(&target_name, damage), EventType::Attack.color_code());
            ActionResult::SuccessWithDetail(format!("attack:{}hit,damage={}", target_name, damage))
        }
    }

    /// TradeOffer：发起交易
    pub fn handle_trade_offer(&mut self, agent_id: &AgentId, offer: HashMap<ResourceType, u32>, want: HashMap<ResourceType, u32>, target_id: AgentId) -> ActionResult {
        let agent = self.agents.get(agent_id).unwrap();
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let current = *agent.inventory.get(key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(format!("资源不足，无法提供 {} x{}", key, amount));
            }
        }

        if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked("交易目标不存在或已死亡".into());
        }

        let agent_name = agent.name.clone();
        let target_name = self.agents.get(&target_id).map(|a| a.name.clone()).unwrap_or_default();
        let builder = NarrativeBuilder::new(agent_name.clone());

        let trade_id = uuid::Uuid::new_v4().to_string();
        let pending = PendingTrade {
            trade_id: trade_id.clone(),
            proposer_id: agent_id.clone(),
            acceptor_id: target_id,
            offer_resources: offer.iter().map(|(r, a)| (r.as_str().to_string(), *a)).collect(),
            want_resources: want.iter().map(|(r, a)| (r.as_str().to_string(), *a)).collect(),
            status: TradeStatus::Pending,
            tick_created: self.tick,
        };

        let proposer = self.agents.get_mut(agent_id).unwrap();
        proposer.freeze_resources(offer.clone(), &trade_id);

        self.pending_trades.push(pending);

        self.record_event(agent_id, &agent_name, EventType::TradeOffer.as_str(),
            &builder.trade_offer(&target_name), EventType::TradeOffer.color_code());
        ActionResult::SuccessWithDetail(format!("trade_offer:{}", target_name))
    }

    /// TradeAccept：接受交易
    pub fn handle_trade_accept(&mut self, agent_id: &AgentId) -> ActionResult {
        let trade_idx = self.pending_trades.iter().position(|t| {
            t.acceptor_id == *agent_id && t.status == TradeStatus::Pending
        });

        if trade_idx.is_none() {
            return ActionResult::Blocked("没有待处理的交易可接受".into());
        }

        let trade_idx = trade_idx.unwrap();
        let trade = self.pending_trades[trade_idx].clone();
        let proposer_id = trade.proposer_id.clone();

        let offer_resources: HashMap<ResourceType, u32> = trade.offer_resources.iter()
            .filter_map(|(k, v)| Some((str_to_resource(k)?, *v)))
            .collect();
        let want_resources: HashMap<ResourceType, u32> = trade.want_resources.iter()
            .filter_map(|(k, v)| Some((str_to_resource(k)?, *v)))
            .collect();

        let acceptor = self.agents.get(agent_id).unwrap();
        for (resource, amount) in &want_resources {
            let key = resource.as_str();
            let current = *acceptor.inventory.get(key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(format!("接受方资源不足，无法提供 {} x{}", key, amount));
            }
        }

        {
            let acceptor = self.agents.get_mut(agent_id).unwrap();
            acceptor.give_resources(want_resources.clone());
            acceptor.receive_resources(offer_resources.clone());
        }
        {
            let proposer = self.agents.get_mut(&proposer_id).unwrap();
            proposer.complete_trade_send(offer_resources.clone(), want_resources.clone());
        }

        self.pending_trades.remove(trade_idx);
        self.total_trades += 1;

        let proposer_name = self.agents.get(&proposer_id).map(|a| a.name.clone()).unwrap_or_default();
        let acceptor_name = self.agents.get(agent_id).map(|a| a.name.clone()).unwrap_or_default();
        let builder = NarrativeBuilder::new(acceptor_name.clone());

        self.record_event(agent_id, &acceptor_name, EventType::TradeAccept.as_str(),
            &builder.trade_completed(&proposer_name), EventType::TradeAccept.color_code());
        ActionResult::SuccessWithDetail(format!("trade_accept:{} ↔ {}", proposer_name, acceptor_name))
    }

    /// TradeReject：拒绝交易
    pub fn handle_trade_reject(&mut self, agent_id: &AgentId) -> ActionResult {
        let trade_idx = self.pending_trades.iter().position(|t| {
            t.acceptor_id == *agent_id && t.status == TradeStatus::Pending
        });

        if trade_idx.is_none() {
            return ActionResult::Blocked("没有待处理的交易可拒绝".into());
        }

        let trade_idx = trade_idx.unwrap();
        let trade = self.pending_trades[trade_idx].clone();
        let proposer_id = trade.proposer_id.clone();

        let offer_resources: HashMap<ResourceType, u32> = trade.offer_resources.iter()
            .filter_map(|(k, v)| Some((str_to_resource(k)?, *v)))
            .collect();

        let proposer = self.agents.get_mut(&proposer_id).unwrap();
        proposer.cancel_trade(offer_resources);

        self.pending_trades.remove(trade_idx);

        let proposer_name = self.agents.get(&proposer_id).map(|a| a.name.clone()).unwrap_or_default();
        let acceptor_name = self.agents.get(agent_id).map(|a| a.name.clone()).unwrap_or_default();
        let builder = NarrativeBuilder::new(acceptor_name.clone());

        self.record_event(agent_id, &acceptor_name, EventType::TradeReject.as_str(),
            &builder.trade_rejected(&proposer_name), EventType::TradeReject.color_code());
        ActionResult::SuccessWithDetail(format!("trade_reject:{}", proposer_name))
    }

    /// AllyPropose：提议结盟
    pub fn handle_ally_propose(&mut self, agent_id: &AgentId, target_id: AgentId) -> ActionResult {
        if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked("结盟目标不存在或已死亡".into());
        }

        let agent = self.agents.get(agent_id).unwrap();
        let can_propose = agent.relations.get(&target_id)
            .map(|r| r.trust > 0.5)
            .unwrap_or(false);

        if !can_propose {
            return ActionResult::Blocked("信任值不足，无法提议结盟".into());
        }

        let agent_name = agent.name.clone();
        let target_name = self.agents.get(&target_id).map(|a| a.name.clone()).unwrap_or_default();
        let builder = NarrativeBuilder::new(agent_name.clone());

        self.record_event(agent_id, &agent_name, EventType::AllyPropose.as_str(),
            &builder.ally_propose(&target_name), EventType::AllyPropose.color_code());
        ActionResult::SuccessWithDetail(format!("ally_propose:{}", target_name))
    }

    /// AllyAccept：接受结盟
    pub fn handle_ally_accept(&mut self, agent_id: &AgentId, ally_id: AgentId) -> ActionResult {
        if !self.agents.get(&ally_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked("结盟目标不存在或已死亡".into());
        }

        self.agents.get_mut(agent_id).unwrap().accept_alliance(ally_id.clone());
        self.agents.get_mut(&ally_id).unwrap().accept_alliance(agent_id.clone());

        let acceptor_name = self.agents.get(agent_id).map(|a| a.name.clone()).unwrap_or_default();
        let proposer_name = self.agents.get(&ally_id).map(|a| a.name.clone()).unwrap_or_default();
        let builder = NarrativeBuilder::new(acceptor_name.clone());

        self.record_event(agent_id, &acceptor_name, EventType::AllyAccept.as_str(),
            &builder.ally_formed(&proposer_name), EventType::AllyAccept.color_code());
        ActionResult::SuccessWithDetail(format!("ally_accept:{} ↔ {}", acceptor_name, proposer_name))
    }

    /// AllyReject：拒绝结盟
    pub fn handle_ally_reject(&mut self, agent_id: &AgentId, ally_id: AgentId) -> ActionResult {
        if !self.agents.contains_key(&ally_id) {
            return ActionResult::Blocked("结盟目标不存在".into());
        }

        self.agents.get_mut(agent_id).unwrap().reject_alliance(ally_id.clone());

        let acceptor_name = self.agents.get(agent_id).map(|a| a.name.clone()).unwrap_or_default();
        let proposer_name = self.agents.get(&ally_id).map(|a| a.name.clone()).unwrap_or_default();
        let builder = NarrativeBuilder::new(acceptor_name.clone());

        self.record_event(agent_id, &acceptor_name, EventType::AllyReject.as_str(),
            &builder.ally_rejected(&proposer_name), EventType::AllyReject.color_code());
        ActionResult::SuccessWithDetail(format!("ally_reject:{}", proposer_name))
    }
}

/// 字符串转资源类型（辅助函数）
fn str_to_resource(s: &str) -> Option<ResourceType> {
    std::str::FromStr::from_str(s).ok()
}