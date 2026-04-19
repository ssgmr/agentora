//! 动作处理器：每个 ActionType 的独立 handler
//!
//! 设计原则：Handler 只返回 ActionResult（携带执行详情），不设置 last_action_result。
//! 反馈生成统一在 apply_action 中处理，确保格式一致。

use crate::agent::{Relation, RelationType};
use crate::types::{
    Action, ActionType, AgentId, Direction, Position, ResourceType, StructureType
};
use crate::world::{ActionResult, World, PendingTrade, TradeStatus};
use crate::world::resource::ResourceNode;
use crate::world::structure::Structure;
use crate::snapshot::NarrativeEvent;
use crate::narrative::{NarrativeBuilder, EventType, action_type_display};
use std::collections::HashMap;

impl World {
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

    // ===== MoveToward =====
    pub fn handle_move_toward(&mut self, agent_id: &AgentId, target: Position) -> ActionResult {
        let (agent_name, current_pos) = {
            let agent = self.agents.get(agent_id).unwrap();
            (agent.name.clone(), agent.position)
        };
        let builder = NarrativeBuilder::new(agent_name.clone());

        // 如果已在目标位置
        if current_pos == target {
            self.record_event(agent_id, &agent_name, EventType::MoveToward.as_str(),
                &builder.already_at_position(target), "#888888");
            return ActionResult::AlreadyAtPosition(
                format!("你已经在 ({},{})，不需要再移动。请选择其他动作（如采集附近资源、探索其他方向等）", target.x, target.y));
        }

        // 校验：目标必须与当前位置相邻
        let dist = current_pos.manhattan_distance(&target);
        if dist != 1 {
            return ActionResult::Blocked(
                format!("目标 ({},{}) 不相邻（距离 {} 格），每次只能移动 1 格", target.x, target.y, dist));
        }

        // 边界检查
        if target.x >= self.map.size().0 || target.y >= self.map.size().1 {
            return ActionResult::OutOfBounds;
        }

        // Fence 碰撞检查
        if let Some(fence) = self.structures.get(&target) {
            if fence.structure_type == StructureType::Fence {
                if let Some(ref owner_id) = fence.owner_id {
                    let is_enemy = self.agents.get(agent_id)
                        .and_then(|a| a.relations.get(owner_id))
                        .map(|r| r.relation_type == RelationType::Enemy)
                        .unwrap_or(false);
                    if is_enemy {
                        return ActionResult::Blocked("被围栏阻挡，无法通过敌对领地".into());
                    }
                }
            }
        }

        // 执行移动
        let agent = self.agents.get_mut(agent_id).unwrap();
        agent.last_position = Some(agent.position);
        agent.position = target;

        // 使用 NarrativeBuilder 生成描述
        self.record_event(agent_id, &agent_name, EventType::MoveToward.as_str(),
            &builder.move_toward(current_pos, target), EventType::MoveToward.color_code());

        // 返回成功详情（包含起终点，供反馈生成使用）
        ActionResult::SuccessWithDetail(format!("move:{},{}→({},{})", current_pos.x, current_pos.y, target.x, target.y))
    }

    // ===== Gather =====
    pub fn handle_gather(&mut self, agent_id: &AgentId, resource: ResourceType) -> ActionResult {
        let pos = self.agents.get(agent_id).unwrap().position;
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());
        let effective_limit = self.effective_inventory_limit_for(pos);
        let resource_key = resource.as_str().to_string();

        // 检查当前位置是否有资源节点
        // 先获取节点信息，然后释放借用
        let node_info = self.resources.get(&pos).map(|node| {
            (node.resource_type, node.is_depleted, node.current_amount)
        });

        if node_info.is_none() {
            return ActionResult::Blocked("脚下无资源节点".into());
        }

        let (node_type, is_depleted, current_amount) = node_info.unwrap();

        if node_type != resource {
            return ActionResult::Blocked(format!("脚下是 {:?}，不是 {:?}，无法采集", node_type, resource));
        }
        if is_depleted {
            return ActionResult::Blocked("资源节点已枯竭".into());
        }

        // 检查压力乘数
        let multiplier = self.pressure_multiplier.get(resource.as_str()).copied().unwrap_or(1.0);

        // 采集资源（每次固定 2 个）
        let gather_amount = 2u32;
        let actual_gather = if current_amount >= gather_amount { gather_amount } else { current_amount };
        if actual_gather == 0 {
            return ActionResult::Blocked("资源不足，无法采集".into());
        }

        let gathered = if multiplier < 1.0 {
            (actual_gather as f32 * multiplier).ceil() as u32
        } else {
            actual_gather
        };
        let gathered = gathered.max(1);

        // 检查库存上限（只读查询）
        let current_inv = self.agents.get(agent_id)
            .and_then(|a| a.inventory.get(&resource_key).copied())
            .unwrap_or(0);
        let limit = effective_limit as u32;

        if current_inv + gathered > limit {
            return ActionResult::Blocked(format!("{} 已满（当前 x{}，上限 {}）", resource_key, current_inv, limit));
        }

        // 执行更新：先更新节点，再更新库存
        // 更新节点（单独借用）
        let remain_amount = {
            let node = self.resources.get_mut(&pos).unwrap();
            node.current_amount = node.current_amount.saturating_sub(actual_gather);
            node.current_amount
        };

        // 更新库存
        let agent = self.agents.get_mut(agent_id).unwrap();
        agent.inventory.insert(resource_key.clone(), current_inv + gathered);

        // 记录事件（使用 NarrativeBuilder）
        self.record_event(agent_id, &agent_name, EventType::Gather.as_str(),
            &builder.gather(resource, gathered), EventType::Gather.color_code());

        ActionResult::SuccessWithDetail(format!("gather:{}x{},remain:{}",
            resource_key, gathered, remain_amount))
    }

    // ===== Wait =====
    pub fn handle_wait(&mut self, agent_id: &AgentId) -> ActionResult {
        let agent = self.agents.get_mut(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());

        self.record_event(agent_id, &agent_name, EventType::Wait.as_str(),
            &builder.wait(), EventType::Wait.color_code());
        ActionResult::SuccessWithDetail("wait".into())
    }

    // ===== Eat =====
    pub fn handle_eat(&mut self, agent_id: &AgentId) -> ActionResult {
        let agent = self.agents.get_mut(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());

        if agent.inventory.get("food").copied().unwrap_or(0) > 0 {
            *agent.inventory.get_mut("food").unwrap() -= 1;
            if agent.inventory["food"] == 0 {
                agent.inventory.remove("food");
            }
            agent.satiety = (agent.satiety + 30).min(100);
            let new_satiety = agent.satiety;

            self.record_event(agent_id, &agent_name, EventType::Eat.as_str(),
                &builder.eat(30), EventType::Eat.color_code());
            ActionResult::SuccessWithDetail(format!("eat:satiety={}/100", new_satiety))
        } else {
            ActionResult::Blocked("背包中没有食物".into())
        }
    }

    // ===== Drink =====
    pub fn handle_drink(&mut self, agent_id: &AgentId) -> ActionResult {
        let agent = self.agents.get_mut(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());

        if agent.inventory.get("water").copied().unwrap_or(0) > 0 {
            *agent.inventory.get_mut("water").unwrap() -= 1;
            if agent.inventory["water"] == 0 {
                agent.inventory.remove("water");
            }
            agent.hydration = (agent.hydration + 25).min(100);
            let new_hydration = agent.hydration;

            self.record_event(agent_id, &agent_name, EventType::Drink.as_str(),
                &builder.drink(25), EventType::Drink.color_code());
            ActionResult::SuccessWithDetail(format!("drink:hydration={}/100", new_hydration))
        } else {
            ActionResult::Blocked("背包中没有水源".into())
        }
    }

    // ===== Build =====
    pub fn handle_build(&mut self, agent_id: &AgentId, structure: StructureType) -> ActionResult {
        let agent = self.agents.get_mut(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());
        let pos = agent.position;

        if self.structures.contains_key(&pos) {
            return ActionResult::Blocked("目标位置已有建筑".into());
        }

        let cost = structure.resource_cost();
        for (resource, amount) in &cost {
            let key = resource.as_str();
            let current = *agent.inventory.get(key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(
                    format!("资源不足，需要 {} x{}，实际有 {}", key, amount, current));
            }
        }

        for (resource, amount) in &cost {
            agent.consume(*resource, *amount);
        }

        let structure_obj = Structure::new(pos, structure, Some(agent_id.clone()), self.tick);
        self.structures.insert(pos, structure_obj);

        self.record_event(agent_id, &agent_name, EventType::Build.as_str(),
            &builder.build(structure, pos), EventType::Build.color_code());
        ActionResult::SuccessWithDetail(format!("build:{:?}at({},{})", structure, pos.x, pos.y))
    }

    // ===== Attack =====
    pub fn handle_attack(&mut self, agent_id: &AgentId, target_id: AgentId) -> ActionResult {
        let (agent_name, target_name) = {
            let agent = self.agents.get(agent_id).unwrap();
            let target_name = self.agents.get(&target_id).map(|a| a.name.clone()).unwrap_or_default();
            (agent.name.clone(), target_name)
        };
        let builder = NarrativeBuilder::new(agent_name.clone());

        if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked("目标不存在或已死亡".into());
        }

        let damage = 10;
        let target_alive;
        {
            let target = self.agents.get_mut(&target_id).unwrap();
            target.health = target.health.saturating_sub(damage);
            target_alive = target.health > 0;

            if let Some(rel) = target.relations.get_mut(agent_id) {
                rel.relation_type = RelationType::Enemy;
                rel.trust = 0.0;
            } else {
                target.relations.insert(agent_id.clone(), Relation {
                    trust: 0.0,
                    relation_type: RelationType::Enemy,
                    last_interaction_tick: 0,
                });
            }
        }

        {
            let attacker = self.agents.get_mut(agent_id).unwrap();
            if let Some(rel) = attacker.relations.get_mut(&target_id) {
                rel.relation_type = RelationType::Enemy;
                rel.trust = 0.0;
            } else {
                attacker.relations.insert(target_id.clone(), Relation {
                    trust: 0.0,
                    relation_type: RelationType::Enemy,
                    last_interaction_tick: 0,
                });
            }
        }

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

    // ===== Talk =====
    pub fn handle_talk(&mut self, agent_id: &AgentId, message: String) -> ActionResult {
        use crate::memory::MemoryEvent;

        let agent = self.agents.get(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());
        let agent_pos = agent.position;

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
        for target_id in &nearby_agents {
            let target_name = self.agents.get(target_id).map(|a| a.name.clone()).unwrap_or_default();
            affected_names.push(target_name.clone());

            self.agents.get_mut(agent_id).unwrap().increase_trust(target_id, 2.0);
            self.agents.get_mut(target_id).unwrap().increase_trust(agent_id, 1.0);

            let target = self.agents.get_mut(target_id).unwrap();
            target.memory.record(&MemoryEvent {
                tick: self.tick as u32,
                event_type: "social".to_string(),
                content: format!("与 {} 交流：「{}」", agent_name, message),
                emotion_tags: vec!["positive".to_string()],
                importance: 0.5,
            });
        }

        let initiator = self.agents.get_mut(agent_id).unwrap();
        initiator.memory.record(&MemoryEvent {
            tick: self.tick as u32,
            event_type: "social".to_string(),
            content: format!("与 {} 交流：「{}」", affected_names.join("、"), message),
            emotion_tags: vec!["positive".to_string()],
            importance: 0.5,
        });

        // 使用 NarrativeBuilder 生成描述
        let event_msg = builder.talk_to(&affected_names, &message);

        self.record_event(agent_id, &agent_name, EventType::Talk.as_str(), &event_msg, EventType::Talk.color_code());
        ActionResult::SuccessWithDetail(format!("talk:{}", affected_names.join(",")))
    }

    // ===== Explore =====
    pub fn handle_explore(&mut self, agent_id: &AgentId) -> ActionResult {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());

        let steps = rng.gen_range(1..=3);
        let dir_idx = rng.gen_range(0..4);
        let directions = [Direction::North, Direction::South, Direction::East, Direction::West];
        let dir = directions[dir_idx];
        let (dx, dy) = dir.delta();
        let direction_name = dir.as_chinese();

        let old_pos = self.agents.get(agent_id).unwrap().position;
        let agent = self.agents.get_mut(agent_id).unwrap();
        for _ in 0..steps {
            let new_x = agent.position.x as i32 + dx;
            let new_y = agent.position.y as i32 + dy;
            if new_x >= 0 && new_y >= 0 {
                let new_pos = Position::new(new_x as u32, new_y as u32);
                if self.map.is_valid(new_pos) && self.map.get_terrain(new_pos).is_passable() {
                    agent.position = new_pos;
                }
            }
        }
        let new_pos = agent.position;

        self.record_event(agent_id, &agent_name, EventType::Explore.as_str(),
            &format!("{} 向{}探索，移动了 {} 步 ({},{})→({},{})", agent_name, direction_name, steps, old_pos.x, old_pos.y, new_pos.x, new_pos.y),
            EventType::Explore.color_code());
        ActionResult::SuccessWithDetail(format!("explore:{}steps,{},{}→({},{})",
            steps, old_pos.x, old_pos.y, new_pos.x, new_pos.y))
    }

    // ===== TradeOffer =====
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

        let pending = PendingTrade {
            proposer_id: agent_id.clone(),
            acceptor_id: target_id,
            offer_resources: offer.iter().map(|(r, a)| (r.as_str().to_string(), *a)).collect(),
            want_resources: want.iter().map(|(r, a)| (r.as_str().to_string(), *a)).collect(),
            status: TradeStatus::Pending,
            tick_created: self.tick,
        };
        self.pending_trades.push(pending);

        self.record_event(agent_id, &agent_name, EventType::TradeOffer.as_str(),
            &builder.trade_offer(&target_name), EventType::TradeOffer.color_code());
        ActionResult::SuccessWithDetail(format!("trade_offer:{}", target_name))
    }

    // ===== TradeAccept =====
    pub fn handle_trade_accept(&mut self, agent_id: &AgentId) -> ActionResult {
        let trade_idx = self.pending_trades.iter().position(|t| {
            t.acceptor_id == *agent_id && t.status == TradeStatus::Pending
        });

        if trade_idx.is_none() {
            return ActionResult::Blocked("没有待处理的交易可接受".into());
        }

        let trade_idx = trade_idx.unwrap();
        let proposer_id = self.pending_trades[trade_idx].proposer_id.clone();

        let proposer = self.agents.get(&proposer_id).unwrap();
        let acceptor = self.agents.get(agent_id).unwrap();

        let offer_resources: HashMap<String, u32> = self.pending_trades[trade_idx].offer_resources.clone();
        let want_resources: HashMap<String, u32> = self.pending_trades[trade_idx].want_resources.clone();

        for (resource_key, amount) in &offer_resources {
            let current = *proposer.inventory.get(resource_key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(format!("发起方资源不足，无法提供 {} x{}", resource_key, amount));
            }
        }

        for (resource_key, amount) in &want_resources {
            let current = *acceptor.inventory.get(resource_key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(format!("接受方资源不足，无法提供 {} x{}", resource_key, amount));
            }
        }

        let proposer_id_clone = proposer_id.clone();
        let proposer = self.agents.get_mut(&proposer_id_clone).unwrap();
        for (resource_key, amount) in &offer_resources {
            let resource = str_to_resource(resource_key).unwrap_or(ResourceType::Food);
            proposer.consume(resource, *amount);
        }
        for (resource_key, amount) in &want_resources {
            let resource = str_to_resource(resource_key).unwrap_or(ResourceType::Food);
            proposer.gather(resource, *amount);
        }

        let acceptor = self.agents.get_mut(agent_id).unwrap();
        for (resource_key, amount) in &want_resources {
            let resource = str_to_resource(resource_key).unwrap_or(ResourceType::Food);
            acceptor.consume(resource, *amount);
        }
        for (resource_key, amount) in &offer_resources {
            let resource = str_to_resource(resource_key).unwrap_or(ResourceType::Food);
            acceptor.gather(resource, *amount);
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

    // ===== TradeReject =====
    pub fn handle_trade_reject(&mut self, agent_id: &AgentId) -> ActionResult {
        let trade_idx = self.pending_trades.iter().position(|t| {
            t.acceptor_id == *agent_id && t.status == TradeStatus::Pending
        });

        if trade_idx.is_none() {
            return ActionResult::Blocked("没有待处理的交易可拒绝".into());
        }

        let trade_idx = trade_idx.unwrap();
        let proposer_id = self.pending_trades[trade_idx].proposer_id.clone();
        self.pending_trades.remove(trade_idx);

        let proposer_name = self.agents.get(&proposer_id).map(|a| a.name.clone()).unwrap_or_default();
        let acceptor_name = self.agents.get(agent_id).map(|a| a.name.clone()).unwrap_or_default();
        let builder = NarrativeBuilder::new(acceptor_name.clone());

        self.record_event(agent_id, &acceptor_name, EventType::TradeReject.as_str(),
            &builder.trade_rejected(&proposer_name), EventType::TradeReject.color_code());
        ActionResult::SuccessWithDetail(format!("trade_reject:{}", proposer_name))
    }

    // ===== AllyPropose =====
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

    // ===== AllyAccept =====
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

    // ===== AllyReject =====
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

    // ===== InteractLegacy =====
    pub fn handle_legacy_interaction(&mut self, agent_id: &AgentId, legacy_id: &str, interaction: &crate::types::LegacyInteraction) -> ActionResult {
        let agent_pos = self.agents.get(agent_id).unwrap().position;

        let legacy_index = self.legacies.iter().position(|l| l.id == legacy_id);
        if legacy_index.is_none() {
            return ActionResult::InvalidAgent;
        }

        if self.legacies[legacy_index.unwrap()].position != agent_pos {
            return ActionResult::Blocked("不在遗产位置，无法交互".into());
        }

        match interaction {
            crate::types::LegacyInteraction::Worship => {
                self.total_legacy_interacts += 1;
                ActionResult::SuccessWithDetail("legacy:worship".into())
            }
            crate::types::LegacyInteraction::Explore => {
                self.total_legacy_interacts += 1;
                ActionResult::SuccessWithDetail("legacy:explore".into())
            }
            crate::types::LegacyInteraction::Pickup => {
                let legacy = &mut self.legacies[legacy_index.unwrap()];
                if legacy.items.is_empty() {
                    return ActionResult::Blocked("遗产无物品可拾取".into());
                }

                let mut items_to_transfer = Vec::new();
                for (item_name, amount) in &legacy.items {
                    if *amount > 0 {
                        items_to_transfer.push((item_name.clone(), *amount));
                        break;
                    }
                }

                if items_to_transfer.is_empty() {
                    return ActionResult::Blocked("拾取失败".into());
                }

                let (item_name, amount) = items_to_transfer[0].clone();
                let agent = self.agents.get_mut(agent_id).unwrap();
                let current = *agent.inventory.get(&item_name).unwrap_or(&0);
                agent.inventory.insert(item_name.clone(), current + amount);

                let legacy = &mut self.legacies[legacy_index.unwrap()];
                legacy.items.insert(item_name.clone(), amount - 1);

                self.total_legacy_interacts += 1;
                ActionResult::SuccessWithDetail(format!("legacy:pickup {}x{}", item_name, amount))
            }
        }
    }
}

fn str_to_resource(s: &str) -> Option<ResourceType> {
    match s {
        "iron" | "Iron" | "铁矿" => Some(ResourceType::Iron),
        "food" | "Food" | "食物" => Some(ResourceType::Food),
        "wood" | "Wood" | "木材" => Some(ResourceType::Wood),
        "water" | "Water" | "水源" => Some(ResourceType::Water),
        "stone" | "Stone" | "石材" => Some(ResourceType::Stone),
        _ => None,
    }
}