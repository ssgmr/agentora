//! 动作处理器：每个 ActionType 的独立 handler
//!
//! 设计原则：Handler 只返回 ActionResult（携带执行详情），不设置 last_action_result。
//! 反馈生成统一在 apply_action 中处理，确保格式一致。

use crate::agent::RelationType;
use crate::types::{
    ActionType, AgentId, Direction, Position, ResourceType, StructureType, Action
};
use crate::world::{ActionResult, World, PendingTrade, TradeStatus};
use crate::world::structure::Structure;
use crate::narrative::{NarrativeBuilder, EventType, action_type_display};
use std::collections::HashMap;
use std::str::FromStr;

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
            ActionType::Explore { .. } => self.handle_explore(agent_id),
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

        // 校验：目标必须与当前位置相邻（World职责）
        let dist = current_pos.manhattan_distance(&target);
        if dist != 1 {
            return ActionResult::Blocked(
                format!("目标 ({},{}) 不相邻（距离 {} 格），每次只能移动 1 格", target.x, target.y, dist));
        }

        // 边界检查（World职责）
        if target.x >= self.map.size().0 || target.y >= self.map.size().1 {
            return ActionResult::OutOfBounds;
        }

        // Fence 碰撞检查（World职责）
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

        // 执行移动：调用 Agent 方法
        let agent = self.agents.get_mut(agent_id).unwrap();
        let (_, old_pos, new_pos) = agent.move_to(target);

        // 使用 NarrativeBuilder 生成描述
        self.record_event(agent_id, &agent_name, EventType::MoveToward.as_str(),
            &builder.move_toward(old_pos, new_pos), EventType::MoveToward.color_code());

        // 返回成功详情
        ActionResult::SuccessWithDetail(format!("move:{},{}→({},{})", old_pos.x, old_pos.y, new_pos.x, new_pos.y))
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

        // 任务 3.4：Gather 失败反馈包含位置信息
        if node_info.is_none() {
            return ActionResult::Blocked(
                format!("当前位置({}, {}) 没有{}资源节点。请先 MoveToward 到资源位置",
                    pos.x, pos.y, resource_key));
        }

        let (node_type, is_depleted, current_amount) = node_info.unwrap();

        if node_type != resource {
            return ActionResult::Blocked(
                format!("当前位置({}, {}) 是 {:?} 资源节点，不是 {:?}。请移动到正确的资源位置",
                    pos.x, pos.y, node_type, resource));
        }
        if is_depleted {
            return ActionResult::Blocked(
                format!("当前位置({}, {}) 的 {:?} 资源节点已枯竭。请寻找其他资源节点",
                    pos.x, pos.y, resource));
        }

        // 检查压力乘数
        let multiplier = self.pressure_multiplier.get(resource.as_str()).copied().unwrap_or(1.0);

        // 采集资源（每次固定 2 个）
        let gather_amount = 2u32;
        let actual_gather = if current_amount >= gather_amount { gather_amount } else { current_amount };
        if actual_gather == 0 {
            return ActionResult::Blocked(
                format!("当前位置({}, {}) 的 {:?} 资源存量不足（剩余0）。请寻找其他资源节点",
                    pos.x, pos.y, resource));
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
            return ActionResult::Blocked(
                format!("{} 已满（当前 x{}，上限 {}）。请先消耗或存储",
                    resource_key, current_inv, limit));
        }

        // 执行更新：先更新节点，再更新库存
        // 更新节点（单独借用）
        let remain_amount = {
            let node = self.resources.get_mut(&pos).unwrap();
            node.current_amount = node.current_amount.saturating_sub(actual_gather);
            node.current_amount
        };

        // 更新库存（任务 3.6：记录背包变化）
        let new_inv = current_inv + gathered;
        let agent = self.agents.get_mut(agent_id).unwrap();
        agent.inventory.insert(resource_key.clone(), new_inv);

        // 记录事件（使用 NarrativeBuilder）
        self.record_event(agent_id, &agent_name, EventType::Gather.as_str(),
            &builder.gather(resource, gathered), EventType::Gather.color_code());

        // 任务 3.6：Gather 成功反馈包含资源剩余和背包变化
        ActionResult::SuccessWithDetail(format!("gather:{}x{},node_remain:{},inv:{}→{}",
            resource_key, gathered, remain_amount, current_inv, new_inv))
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
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());

        // 调用 Agent 方法
        let agent = self.agents.get_mut(agent_id).unwrap();
        let (success, delta, before, after, remain) = agent.eat_food();

        if success {
            self.record_event(agent_id, &agent_name, EventType::Eat.as_str(),
                &builder.eat(delta), EventType::Eat.color_code());
            ActionResult::SuccessWithDetail(format!("eat:satiety+{}({}→{}),food_remain={}",
                delta, before, after, remain))
        } else {
            // 失败：获取背包状态
            let inventory_str: Vec<String> = agent.inventory.iter()
                .map(|(r, n)| format!("{} x{}", r, n))
                .collect();
            ActionResult::Blocked(
                format!("背包中没有food。当前背包：{}",
                    if inventory_str.is_empty() { "空".to_string() } else { inventory_str.join(", ") }))
        }
    }

    // ===== Drink =====
    pub fn handle_drink(&mut self, agent_id: &AgentId) -> ActionResult {
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());

        // 调用 Agent 方法
        let agent = self.agents.get_mut(agent_id).unwrap();
        let (success, delta, before, after, remain) = agent.drink_water();

        if success {
            self.record_event(agent_id, &agent_name, EventType::Drink.as_str(),
                &builder.drink(delta), EventType::Drink.color_code());
            ActionResult::SuccessWithDetail(format!("drink:hydration+{}({}→{}),water_remain={}",
                delta, before, after, remain))
        } else {
            // 失败：获取背包状态
            let inventory_str: Vec<String> = agent.inventory.iter()
                .map(|(r, n)| format!("{} x{}", r, n))
                .collect();
            ActionResult::Blocked(
                format!("背包中没有water。当前背包：{}",
                    if inventory_str.is_empty() { "空".to_string() } else { inventory_str.join(", ") }))
        }
    }

    // ===== Build =====
    pub fn handle_build(&mut self, agent_id: &AgentId, structure: StructureType) -> ActionResult {
        let agent = self.agents.get_mut(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());
        let pos = agent.position;

        if self.structures.contains_key(&pos) {
            return ActionResult::Blocked("目标位置已有建筑，无法在此建造".into());
        }

        let cost = structure.resource_cost();
        // 检查所有资源是否足够
        let insufficient: Vec<(String, u32, u32)> = cost.iter()
            .filter_map(|(resource, required)| {
                let key = resource.as_str();
                let current = *agent.inventory.get(key).unwrap_or(&0);
                if current < *required {
                    Some((key.to_string(), *required, current))
                } else {
                    None
                }
            })
            .collect();

        if !insufficient.is_empty() {
            // 任务 3.2：生成详细的资源不足反馈
            let required_str: Vec<String> = cost.iter()
                .map(|(r, n)| format!("{} x{}", r.as_str(), n))
                .collect();
            let inventory_str: Vec<String> = agent.inventory.iter()
                .map(|(r, n)| format!("{} x{}", r, n))
                .collect();
            return ActionResult::Blocked(
                format!("资源不足。需要 {}，背包中只有 {}",
                    required_str.join(" + "),
                    if inventory_str.is_empty() { "空背包".to_string() } else { inventory_str.join(" + ") }));
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
        let agent_pos = self.agents.get(agent_id).unwrap().position;
        let (agent_name, target_name) = {
            let agent = self.agents.get(agent_id).unwrap();
            let target_name = self.agents.get(&target_id).map(|a| a.name.clone()).unwrap_or_default();
            (agent.name.clone(), target_name)
        };
        let builder = NarrativeBuilder::new(agent_name.clone());

        // 目标不存在检查（World职责）
        if !self.agents.contains_key(&target_id) {
            return ActionResult::Blocked(
                format!("Attack失败：目标Agent {} 不存在", target_id.as_str()));
        }

        // 目标死亡检查（World职责）
        if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked(
                format!("Attack失败：目标Agent {} 已死亡", target_name));
        }

        // 距离限制检查（World职责）
        let target_pos = self.agents.get(&target_id).unwrap().position;
        let distance = agent_pos.manhattan_distance(&target_pos);
        if distance > 1 {
            return ActionResult::Blocked(
                format!("Attack失败：目标Agent {} 距离过远（距离{}格）。Attack只能对相邻格Agent执行",
                    target_name, distance));
        }

        // 盟友关系检查（World职责）
        let is_ally = self.agents.get(agent_id)
            .and_then(|a| a.relations.get(&target_id))
            .map(|r| r.relation_type == RelationType::Ally)
            .unwrap_or(false);
        if is_ally {
            return ActionResult::Blocked(
                format!("Attack失败：不能攻击盟友Agent {}。若要攻击，需先解除盟约", target_name));
        }

        // World计算damage
        let damage = 10;

        // 分段借用，调用Agent方法
        {
            let target = self.agents.get_mut(&target_id).unwrap();
            target.receive_attack(damage, agent_id);
        }
        {
            let attacker = self.agents.get_mut(agent_id).unwrap();
            attacker.initiate_attack(&target_id);
        }

        // 检查目标存活状态
        let target_alive = self.agents.get(&target_id).map(|a| a.health > 0).unwrap_or(false);

        // World维护统计
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

        // 循环调用每个 nearby 的 receive_talk()
        for target_id in &nearby_agents {
            let target_name = self.agents.get(target_id).map(|a| a.name.clone()).unwrap_or_default();
            affected_names.push(target_name.clone());

            let target = self.agents.get_mut(target_id).unwrap();
            target.receive_talk(agent_id, &agent_name, &message, tick);
        }

        // 调用发起方的 talk_with()
        let initiator = self.agents.get_mut(agent_id).unwrap();
        initiator.talk_with(&nearby_agents, &message, tick);

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

        // World随机方向计算（单步）
        let dir_idx = rng.gen_range(0..4);
        let directions = [Direction::North, Direction::South, Direction::East, Direction::West];
        let dir = directions[dir_idx];
        let (dx, dy) = dir.delta();
        let direction_name = dir.as_chinese();

        let old_pos = self.agents.get(agent_id).unwrap().position;
        let new_x = old_pos.x as i32 + dx;
        let new_y = old_pos.y as i32 + dy;

        // 边界和地形校验（World职责）
        if new_x < 0 || new_y < 0 {
            self.record_event(agent_id, &agent_name, EventType::Explore.as_str(),
                &format!("{} 向{}探索，但边界阻挡", agent_name, direction_name), EventType::Explore.color_code());
            return ActionResult::Blocked("探索被边界阻挡".into());
        }

        let target = Position::new(new_x as u32, new_y as u32);
        if !self.map.is_valid(target) || !self.map.get_terrain(target).is_passable() {
            self.record_event(agent_id, &agent_name, EventType::Explore.as_str(),
                &format!("{} 向{}探索，但地形阻挡", agent_name, direction_name), EventType::Explore.color_code());
            return ActionResult::Blocked("探索被地形阻挡".into());
        }

        // 调用Agent方法执行移动
        let agent = self.agents.get_mut(agent_id).unwrap();
        let (_, _, new_pos) = agent.move_to(target);

        self.record_event(agent_id, &agent_name, EventType::Explore.as_str(),
            &format!("{} 向{}探索 ({},{})→({},{})", agent_name, direction_name, old_pos.x, old_pos.y, new_pos.x, new_pos.y),
            EventType::Explore.color_code());
        ActionResult::SuccessWithDetail(format!("explore:{},({},{})→({},{})", direction_name, old_pos.x, old_pos.y, new_pos.x, new_pos.y))
    }

    // ===== TradeOffer =====
    pub fn handle_trade_offer(&mut self, agent_id: &AgentId, offer: HashMap<ResourceType, u32>, want: HashMap<ResourceType, u32>, target_id: AgentId) -> ActionResult {
        // World校验发起方资源足够
        let agent = self.agents.get(agent_id).unwrap();
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let current = *agent.inventory.get(key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(format!("资源不足，无法提供 {} x{}", key, amount));
            }
        }

        // 目标存在性校验（World职责）
        if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked("交易目标不存在或已死亡".into());
        }

        let agent_name = agent.name.clone();
        let target_name = self.agents.get(&target_id).map(|a| a.name.clone()).unwrap_or_default();
        let builder = NarrativeBuilder::new(agent_name.clone());

        // World创建PendingTrade（包含trade_id）
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

        // 调用proposer.freeze_resources(offer, trade_id)
        let proposer = self.agents.get_mut(agent_id).unwrap();
        proposer.freeze_resources(offer.clone(), &trade_id);

        // 添加到pending_trades队列
        self.pending_trades.push(pending);

        self.record_event(agent_id, &agent_name, EventType::TradeOffer.as_str(),
            &builder.trade_offer(&target_name), EventType::TradeOffer.color_code());
        ActionResult::SuccessWithDetail(format!("trade_offer:{}", target_name))
    }

    // ===== TradeAccept =====
    pub fn handle_trade_accept(&mut self, agent_id: &AgentId) -> ActionResult {
        // World查找pending_trade
        let trade_idx = self.pending_trades.iter().position(|t| {
            t.acceptor_id == *agent_id && t.status == TradeStatus::Pending
        });

        if trade_idx.is_none() {
            return ActionResult::Blocked("没有待处理的交易可接受".into());
        }

        let trade_idx = trade_idx.unwrap();
        let trade = self.pending_trades[trade_idx].clone();
        let proposer_id = trade.proposer_id.clone();

        // 获取资源Map（String格式）
        let offer_resources: HashMap<ResourceType, u32> = trade.offer_resources.iter()
            .filter_map(|(k, v)| Some((str_to_resource(k)?, *v)))
            .collect();
        let want_resources: HashMap<ResourceType, u32> = trade.want_resources.iter()
            .filter_map(|(k, v)| Some((str_to_resource(k)?, *v)))
            .collect();

        // World校验双方资源足够（acceptor需要want，proposer的offer已在frozen中）
        let acceptor = self.agents.get(agent_id).unwrap();
        for (resource, amount) in &want_resources {
            let key = resource.as_str();
            let current = *acceptor.inventory.get(key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(format!("接受方资源不足，无法提供 {} x{}", key, amount));
            }
        }

        // 分段借用，调用Agent方法
        {
            let acceptor = self.agents.get_mut(agent_id).unwrap();
            acceptor.give_resources(want_resources.clone());
            acceptor.receive_resources(offer_resources.clone());
        }
        {
            let proposer = self.agents.get_mut(&proposer_id).unwrap();
            proposer.complete_trade_send(offer_resources.clone(), want_resources.clone());
        }

        // 移除pending_trade，更新统计
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
        // World查找pending_trade
        let trade_idx = self.pending_trades.iter().position(|t| {
            t.acceptor_id == *agent_id && t.status == TradeStatus::Pending
        });

        if trade_idx.is_none() {
            return ActionResult::Blocked("没有待处理的交易可拒绝".into());
        }

        let trade_idx = trade_idx.unwrap();
        let trade = self.pending_trades[trade_idx].clone();
        let proposer_id = trade.proposer_id.clone();

        // 获取资源Map（用于cancel_trade）
        let offer_resources: HashMap<ResourceType, u32> = trade.offer_resources.iter()
            .filter_map(|(k, v)| Some((str_to_resource(k)?, *v)))
            .collect();

        // 调用proposer.cancel_trade(offer)
        let proposer = self.agents.get_mut(&proposer_id).unwrap();
        proposer.cancel_trade(offer_resources);

        // 移除pending_trade
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
    ResourceType::from_str(s).ok()
}