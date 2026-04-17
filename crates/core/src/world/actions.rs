//! 动作处理器：每个 ActionType 的独立 handler

use crate::agent::{Relation, RelationType};
use crate::types::{
    Action, ActionType, AgentId, Direction, Position, ResourceType, StructureType
};
use crate::world::{ActionResult, World, PendingTrade, TradeStatus};
use crate::world::resource::ResourceNode;
use crate::world::structure::Structure;
use crate::snapshot::NarrativeEvent;
use std::collections::HashMap;

impl World {
    /// 记录错误叙事（统一入口）
    pub fn record_error_narrative(&mut self, agent_id: &AgentId, action_type: &ActionType, reason: &str) {
        if let Some(agent) = self.agents.get(agent_id) {
            let agent_name = agent.name.clone();
            let action_name = match action_type {
                ActionType::Move { .. } => "移动",
                ActionType::MoveToward { .. } => "导航移动",
                ActionType::Gather { .. } => "采集",
                ActionType::Build { .. } => "建造",
                ActionType::Attack { .. } => "攻击",
                ActionType::Talk { .. } => "对话",
                ActionType::Explore { .. } => "探索",
                ActionType::TradeOffer { .. } => "交易提议",
                ActionType::TradeAccept { .. } => "交易接受",
                ActionType::TradeReject { .. } => "交易拒绝",
                ActionType::AllyPropose { .. } => "结盟提议",
                ActionType::AllyAccept { .. } => "结盟接受",
                ActionType::AllyReject { .. } => "结盟拒绝",
                ActionType::Wait => "休息",
                ActionType::InteractLegacy { .. } => "遗产交互",
            };
            self.record_event(agent_id, &agent_name, "error",
                &format!("{} 尝试{}失败：{}", agent_name, action_name, reason), "#FF6666");
        }
    }

    // ===== Move =====
    pub fn handle_move(&mut self, agent_id: &AgentId, direction: Direction) -> ActionResult {
        // 先提取必要信息，避免后续双重借用
        let (agent_name, old_pos) = {
            let agent = self.agents.get(agent_id).unwrap();
            (agent.name.clone(), agent.position)
        };
        let (dx, dy) = direction.delta();
        let new_x = old_pos.x as i32 + dx;
        let new_y = old_pos.y as i32 + dy;

        if new_x < 0 || new_y < 0 {
            return ActionResult::Blocked("移动超出地图边界".into());
        }

        let new_pos = Position::new(new_x as u32, new_y as u32);
        if !self.map.is_valid(new_pos) {
            return ActionResult::OutOfBounds;
        }

        let terrain = self.map.get_terrain(new_pos);
        if !terrain.is_passable() {
            return ActionResult::Blocked(format!("{:?} 地形不可通行", terrain));
        }

        // Fence 碰撞检查：目标格有 Fence 且 Agent 与所有者为 Enemy → 阻挡
        if let Some(fence) = self.structures.get(&new_pos) {
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

        // 更新位置
        self.agents.get_mut(agent_id).unwrap().position = new_pos;
        self.record_event(agent_id, &agent_name, "move",
            &format!("{} 移动至 ({},{})", agent_name, new_pos.x, new_pos.y), "#888888");
        ActionResult::Success
    }

    // ===== MoveToward =====
    /// 处理 MoveToward 动作：导航到目标位置（单步移动）
    ///
    /// 每次只移动一格，朝向目标位置的方向移动
    /// 使用 calculate_direction 计算最优方向
    pub fn handle_move_toward(&mut self, agent_id: &AgentId, target: Position) -> ActionResult {
        let (agent_name, current_pos) = {
            let agent = self.agents.get(agent_id).unwrap();
            (agent.name.clone(), agent.position)
        };

        // 如果已在目标位置，无操作
        if current_pos == target {
            self.record_event(agent_id, &agent_name, "move_toward",
                &format!("{} 已在目标位置 ({},{})", agent_name, target.x, target.y), "#888888");
            return ActionResult::Success;
        }

        // 计算方向
        let direction = match crate::vision::calculate_direction(&current_pos, &target) {
            Some(d) => d,
            None => {
                // 已在目标位置（理论上不会到达这里）
                return ActionResult::Success;
            }
        };

        // 记录当前位置，用于计算移动后与目标的距离
        let old_distance = current_pos.manhattan_distance(&target);

        // 复用现有的移动逻辑
        let result = self.handle_move(agent_id, direction);

        // 更新叙事描述
        if let ActionResult::Success = result {
            let new_pos = self.agents.get(agent_id).unwrap().position;
            let new_distance = new_pos.manhattan_distance(&target);

            self.record_event(agent_id, &agent_name, "move_toward",
                &format!("{} 向目标 ({},{}) 移动，当前距离 {} 格 → {} 格",
                    agent_name, target.x, target.y, old_distance, new_distance), "#88AA88");
        }

        result
    }

    // ===== Gather =====
    pub fn handle_gather(&mut self, agent_id: &AgentId, resource: ResourceType) -> ActionResult {
        let pos = self.agents.get(agent_id).unwrap().position;
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();

        // 计算有效库存上限（需要先获取位置）
        let effective_limit = self.effective_inventory_limit_for(pos);

        // 检查当前位置是否有资源节点
        if let Some(node) = self.resources.get_mut(&pos) {
            if node.resource_type != resource {
                return ActionResult::Blocked(
                    format!("当前位置资源类型为 {:?}，尝试采集 {:?}", node.resource_type, resource));
            }
            if node.is_depleted {
                return ActionResult::Blocked("资源节点已枯竭".into());
            }

            // 检查压力乘数（干旱等效果）
            let multiplier = self.pressure_multiplier.get(resource.as_str()).copied().unwrap_or(1.0);

            // 真实调用 ResourceNode.gather() 扣除资源
            // 每次采集获取 2-3 个资源，提高生存效率
            let gather_amount = 2u32;
            let base_gathered = node.gather(gather_amount);
            if base_gathered == 0 {
                return ActionResult::Blocked("采集失败，资源不足".into());
            }

            // 应用压力乘数计算实际采集量
            let gathered = if multiplier < 1.0 {
                (base_gathered as f32 * multiplier).ceil() as u32
            } else {
                base_gathered
            };
            let gathered = gathered.max(1); // 至少采集1个

            // Agent 库存增加（受 Warehouse 影响的动态上限）
            let agent = self.agents.get_mut(agent_id).unwrap();
            let resource_key = resource.as_str().to_string();
            let current = *agent.inventory.get(&resource_key).unwrap_or(&0);
            if current + gathered > (effective_limit as u32).min(99) {
                // 库存已满
                return ActionResult::Blocked("背包已满，无法采集更多资源".into());
            }
            agent.inventory.insert(resource_key.clone(), current + gathered);

            self.record_event(agent_id, &agent_name, "gather",
                &format!("{} 采集了 {} 个 {}", agent_name, gathered, resource_key), "#88CC44");
            ActionResult::Success
        } else {
            ActionResult::Blocked("当前位置无资源节点".into())
        }
    }

    // ===== Wait =====
    pub fn handle_wait(&mut self, agent_id: &AgentId) -> ActionResult {
        let agent = self.agents.get_mut(agent_id).unwrap();
        let agent_name = agent.name.clone();

        // Wait 动作改为饮食恢复：尝试消耗食物和水
        let mut ate = false;
        let mut drank = false;

        // 消耗 1 Food → satiety +30
        if agent.inventory.get("food").copied().unwrap_or(0) > 0 {
            agent.inventory.insert("food".to_string(), agent.inventory["food"] - 1);
            if agent.inventory["food"] == 0 {
                agent.inventory.remove("food");
            }
            agent.satiety = (agent.satiety + 30).min(100);
            ate = true;
        }

        // 消耗 1 Water → hydration +25
        if agent.inventory.get("water").copied().unwrap_or(0) > 0 {
            agent.inventory.insert("water".to_string(), agent.inventory["water"] - 1);
            if agent.inventory["water"] == 0 {
                agent.inventory.remove("water");
            }
            agent.hydration = (agent.hydration + 25).min(100);
            drank = true;
        }

        let desc = if ate && drank {
            format!("{} 进食并饮水，恢复体力", agent_name)
        } else if ate {
            format!("{} 进食恢复体力，但缺少饮水", agent_name)
        } else if drank {
            format!("{} 饮水恢复体力，但缺少食物", agent_name)
        } else {
            format!("{} 休息中，但背包没有食物和水源", agent_name)
        };

        self.record_event(agent_id, &agent_name, "wait", &desc, "#CCCCCC");
        ActionResult::Success
    }

    // ===== Build =====
    pub fn handle_build(&mut self, agent_id: &AgentId, structure: StructureType) -> ActionResult {
        let agent = self.agents.get_mut(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let pos = agent.position;

        // 校验：位置无已有建筑
        if self.structures.contains_key(&pos) {
            return ActionResult::Blocked("目标位置已有建筑".into());
        }

        // 校验：资源消耗
        let cost = structure.resource_cost();
        for (resource, amount) in &cost {
            let key = resource.as_str();
            let current = *agent.inventory.get(key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(
                    format!("资源不足，需要 {} x{}，实际有 {}", key, amount, current));
            }
        }

        // 扣除资源
        for (resource, amount) in &cost {
            agent.consume(*resource, *amount);
        }

        // 创建 Structure
        let structure_obj = Structure::new(pos, structure, Some(agent_id.clone()), self.tick);
        self.structures.insert(pos, structure_obj);

        self.record_event(agent_id, &agent_name, "build",
            &format!("{} 在 ({},{}) 建造了 {:?}", agent_name, pos.x, pos.y, structure), "#FF44AA");
        ActionResult::Success
    }

    // ===== Attack =====
    pub fn handle_attack(&mut self, agent_id: &AgentId, target_id: AgentId) -> ActionResult {
        let (agent_name, target_name) = {
            let agent = self.agents.get(agent_id).unwrap();
            let target_name = self.agents.get(&target_id).map(|a| a.name.clone()).unwrap_or_default();
            (agent.name.clone(), target_name)
        };

        // 检查目标存在且存活
        if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked("目标不存在或已死亡".into());
        }

        // 手动实现战斗逻辑（避免 HashMap 双重可变借用问题）
        let damage = 10;
        let target_alive;
        {
            let target = self.agents.get_mut(&target_id).unwrap();
            target.health = target.health.saturating_sub(damage);
            target_alive = target.health > 0;

            // 目标标记攻击者为敌人
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

        // 攻击者标记目标为敌人
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

        if !target_alive {
            self.record_event(agent_id, &agent_name, "attack",
                &format!("{} 攻击了 {} 并将其击败", agent_name, target_name), "#FF0000");
        } else {
            self.record_event(agent_id, &agent_name, "attack",
                &format!("{} 攻击了 {}，造成 {} 点伤害", agent_name, target_name, damage), "#FF4444");
        }

        // 更新攻击计数器
        self.total_attacks += 1;

        ActionResult::Success
    }

    // ===== Talk =====
    pub fn handle_talk(&mut self, agent_id: &AgentId, message: String) -> ActionResult {
        use crate::memory::MemoryEvent;

        let agent = self.agents.get(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let agent_pos = agent.position;

        // 视野范围（与 vision.rs 保持一致）
        const VISION_RANGE: i32 = 3;

        // 找到视野范围内的其他 Agent
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
            // 没有听众，自言自语
            self.record_event(agent_id, &agent_name, "talk",
                &format!("{} 自言自语：「{}」", agent_name, message), "#AAAAAA");
            return ActionResult::Success;
        }

        // 收集附近 Agent 名字并增加信任
        let mut affected_names = Vec::new();
        for target_id in &nearby_agents {
            let target_name = self.agents.get(target_id).map(|a| a.name.clone()).unwrap_or_default();
            affected_names.push(target_name.clone());

            // 双向信任增加：发起者 +2.0，接收者 +1.0
            self.agents.get_mut(agent_id).unwrap().increase_trust(target_id, 2.0);
            self.agents.get_mut(target_id).unwrap().increase_trust(agent_id, 1.0);

            // 为接收者记录记忆
            let target = self.agents.get_mut(target_id).unwrap();
            target.memory.record(&MemoryEvent {
                tick: self.tick as u32,
                event_type: "social".to_string(),
                content: format!("与 {} 交流：「{}」", agent_name, message),
                emotion_tags: vec!["positive".to_string()],
                importance: 0.5,
            });
        }

        // 为发起者记录记忆
        let initiator = self.agents.get_mut(agent_id).unwrap();
        initiator.memory.record(&MemoryEvent {
            tick: self.tick as u32,
            event_type: "social".to_string(),
            content: format!("与 {} 交流：「{}」", affected_names.join("、"), message),
            emotion_tags: vec!["positive".to_string()],
            importance: 0.5,
        });

        // 生成叙事事件
        let event_msg = if affected_names.len() == 1 {
            format!("{} 与 {} 交流：「{}」", agent_name, affected_names[0], message)
        } else {
            format!("{} 向 {} 说：「{}」", agent_name, affected_names.join("、"), message)
        };

        self.record_event(agent_id, &agent_name, "talk", &event_msg, "#FFAA44");
        ActionResult::Success
    }

    // ===== Explore =====
    pub fn handle_explore(&mut self, agent_id: &AgentId) -> ActionResult {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();

        let steps = rng.gen_range(1..=3);
        let dir_idx = rng.gen_range(0..4);
        let directions = [Direction::North, Direction::South, Direction::East, Direction::West];
        let dir = directions[dir_idx];
        let (dx, dy) = dir.delta();

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

        self.record_event(agent_id, &agent_name, "explore",
            &format!("{} 探索周边区域，移动了 {} 步", agent_name, steps), "#44AAFF");
        ActionResult::Success
    }

    // ===== TradeOffer =====
    pub fn handle_trade_offer(&mut self, agent_id: &AgentId, offer: HashMap<ResourceType, u32>, want: HashMap<ResourceType, u32>, target_id: AgentId) -> ActionResult {
        // 校验：发起方是否有足够资源
        let agent = self.agents.get(agent_id).unwrap();
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let current = *agent.inventory.get(key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(
                    format!("资源不足，无法提供 {} x{}", key, amount));
            }
        }

        // 校验：目标存在且存活
        if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked("交易目标不存在或已死亡".into());
        }

        let agent_name = agent.name.clone();
        let target_name = self.agents.get(&target_id).map(|a| a.name.clone()).unwrap_or_default();

        // 创建待处理交易
        let pending = PendingTrade {
            proposer_id: agent_id.clone(),
            acceptor_id: target_id,
            offer_resources: offer.iter().map(|(r, a)| (r.as_str().to_string(), *a)).collect(),
            want_resources: want.iter().map(|(r, a)| (r.as_str().to_string(), *a)).collect(),
            status: TradeStatus::Pending,
            tick_created: self.tick,
        };
        self.pending_trades.push(pending);

        self.record_event(agent_id, &agent_name, "trade",
            &format!("{} 向 {} 发起交易请求", agent_name, target_name), "#44FFAA");
        ActionResult::Success
    }

    // ===== TradeAccept =====
    pub fn handle_trade_accept(&mut self, agent_id: &AgentId) -> ActionResult {
        // 查找是否有指向该 Agent 的待处理交易
        let trade_idx = self.pending_trades.iter().position(|t| {
            t.acceptor_id == *agent_id && t.status == TradeStatus::Pending
        });

        if trade_idx.is_none() {
            return ActionResult::Blocked("没有待处理的交易可接受".into());
        }

        let trade_idx = trade_idx.unwrap();
        let proposer_id = self.pending_trades[trade_idx].proposer_id.clone();

        // 校验：双方都有足够资源
        let proposer = self.agents.get(&proposer_id).unwrap();
        let acceptor = self.agents.get(agent_id).unwrap();

        let offer_resources: HashMap<String, u32> = self.pending_trades[trade_idx].offer_resources.clone();
        let want_resources: HashMap<String, u32> = self.pending_trades[trade_idx].want_resources.clone();

        // 检查发起方是否有 offer 资源
        for (resource_key, amount) in &offer_resources {
            let current = *proposer.inventory.get(resource_key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(
                    format!("发起方资源不足，无法提供 {} x{}", resource_key, amount));
            }
        }

        // 检查接受方是否有 want 资源
        for (resource_key, amount) in &want_resources {
            let current = *acceptor.inventory.get(resource_key).unwrap_or(&0);
            if current < *amount {
                return ActionResult::Blocked(
                    format!("接受方资源不足，无法提供 {} x{}", resource_key, amount));
            }
        }

        // 执行交易：发起方给出 offer，获得 want
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

        // 接受方给出 want，获得 offer
        let acceptor = self.agents.get_mut(agent_id).unwrap();
        for (resource_key, amount) in &want_resources {
            let resource = str_to_resource(resource_key).unwrap_or(ResourceType::Food);
            acceptor.consume(resource, *amount);
        }
        for (resource_key, amount) in &offer_resources {
            let resource = str_to_resource(resource_key).unwrap_or(ResourceType::Food);
            acceptor.gather(resource, *amount);
        }

        // 移除待处理交易
        self.pending_trades.remove(trade_idx);

        // 更新交易计数器
        self.total_trades += 1;

        let proposer_name = self.agents.get(&proposer_id).map(|a| a.name.clone()).unwrap_or_default();
        let acceptor_name = self.agents.get(agent_id).map(|a| a.name.clone()).unwrap_or_default();

        self.record_event(agent_id, &proposer_name, "trade",
            &format!("{} 与 {} 完成了交易", proposer_name, acceptor_name), "#44FFAA");
        ActionResult::Success
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

        self.record_event(agent_id, &acceptor_name, "trade",
            &format!("{} 拒绝了 {} 的交易请求", acceptor_name, proposer_name), "#FFAA88");
        ActionResult::Success
    }

    // ===== AllyPropose =====
    pub fn handle_ally_propose(&mut self, agent_id: &AgentId, target_id: AgentId) -> ActionResult {
        // 校验：目标存在且存活
        if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked("结盟目标不存在或已死亡".into());
        }

        // 校验：信任值足够
        let agent = self.agents.get(agent_id).unwrap();
        let can_propose = if let Some(rel) = agent.relations.get(&target_id) {
            rel.trust > 0.5
        } else {
            false // 无关系记录，不能结盟
        };

        if !can_propose {
            return ActionResult::Blocked("信任值不足，无法提议结盟".into());
        }

        let agent_name = agent.name.clone();
        let target_name = self.agents.get(&target_id).map(|a| a.name.clone()).unwrap_or_default();

        self.record_event(agent_id, &agent_name, "ally",
            &format!("{} 向 {} 提议结盟", agent_name, target_name), "#AAFF44");
        ActionResult::Success
    }

    // ===== AllyAccept =====
    pub fn handle_ally_accept(&mut self, agent_id: &AgentId, ally_id: AgentId) -> ActionResult {
        // 校验：目标存在且存活
        if !self.agents.get(&ally_id).map(|a| a.is_alive).unwrap_or(false) {
            return ActionResult::Blocked("结盟目标不存在或已死亡".into());
        }

        // 接受方建立联盟关系
        let acceptor = self.agents.get_mut(agent_id).unwrap();
        acceptor.accept_alliance(ally_id.clone());

        // 发起方也建立联盟关系
        let proposer = self.agents.get_mut(&ally_id).unwrap();
        proposer.accept_alliance(agent_id.clone());

        let acceptor_name = self.agents.get(agent_id).map(|a| a.name.clone()).unwrap_or_default();
        let proposer_name = self.agents.get(&ally_id).map(|a| a.name.clone()).unwrap_or_default();

        self.record_event(agent_id, &acceptor_name, "ally",
            &format!("{} 与 {} 结成了联盟", acceptor_name, proposer_name), "#AAFF44");
        ActionResult::Success
    }

    // ===== AllyReject =====
    pub fn handle_ally_reject(&mut self, agent_id: &AgentId, ally_id: AgentId) -> ActionResult {
        // 校验：目标存在
        if !self.agents.contains_key(&ally_id) {
            return ActionResult::Blocked("结盟目标不存在".into());
        }

        // 拒绝方略微降低信任
        let acceptor = self.agents.get_mut(agent_id).unwrap();
        acceptor.reject_alliance(ally_id.clone());

        let acceptor_name = self.agents.get(agent_id).map(|a| a.name.clone()).unwrap_or_default();
        let proposer_name = self.agents.get(&ally_id).map(|a| a.name.clone()).unwrap_or_default();

        self.record_event(agent_id, &acceptor_name, "ally",
            &format!("{} 拒绝了 {} 的结盟请求", acceptor_name, proposer_name), "#FFAA88");
        ActionResult::Success
    }

    // ===== InteractLegacy =====
    pub fn handle_legacy_interaction(&mut self, agent_id: &AgentId, legacy_id: &str, interaction: &crate::types::LegacyInteraction) -> ActionResult {
        let agent_pos = self.agents.get(agent_id).unwrap().position;

        // 查找遗产
        let legacy_index = self.legacies.iter().position(|l| l.id == legacy_id);
        if legacy_index.is_none() {
            return ActionResult::InvalidAgent;
        }

        // 检查 Agent 是否在遗产位置
        if self.legacies[legacy_index.unwrap()].position != agent_pos {
            return ActionResult::Blocked("不在遗产位置，无法交互".into());
        }

        match interaction {
            crate::types::LegacyInteraction::Worship => {
                let agent = self.agents.get_mut(agent_id).unwrap();
                agent.motivation[2] = (agent.motivation[2] + 0.05).clamp(0.0, 1.0);
                agent.motivation[5] = (agent.motivation[5] + 0.05).clamp(0.0, 1.0);
                self.total_legacy_interacts += 1;
                ActionResult::Success
            }
            crate::types::LegacyInteraction::Explore => {
                let agent = self.agents.get_mut(agent_id).unwrap();
                agent.motivation[2] = (agent.motivation[2] + 0.1).clamp(0.0, 1.0);
                self.total_legacy_interacts += 1;
                ActionResult::Success
            }
            crate::types::LegacyInteraction::Pickup => {
                let legacy = &mut self.legacies[legacy_index.unwrap()];
                if legacy.items.is_empty() {
                    return ActionResult::Blocked("遗产无物品可拾取".into());
                }

                // 拾取第一个有数量的物品
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

                // 从遗产中移除物品
                let legacy = &mut self.legacies[legacy_index.unwrap()];
                legacy.items.insert(item_name, amount - 1);

                self.total_legacy_interacts += 1;
                ActionResult::Success
            }
        }
    }
}

/// 资源类型字符串转换
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
