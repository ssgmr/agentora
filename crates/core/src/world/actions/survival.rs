//! 生存相关动作处理器
//!
//! Gather、Eat、Drink、Build、Wait

use crate::types::{AgentId, ResourceType, StructureType};
use crate::world::{ActionResult, World};
use crate::world::structure::Structure;
use crate::narrative::{NarrativeBuilder, EventType};

impl World {
    /// Gather：采集当前位置的资源
    pub fn handle_gather(&mut self, agent_id: &AgentId, resource: ResourceType) -> ActionResult {
        let pos = self.agents.get(agent_id).unwrap().position;
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());
        let effective_limit = self.effective_inventory_limit_for(pos);
        let resource_key = resource.as_str().to_string();

        // 检查当前位置是否有资源节点
        let node_info = self.resources.get(&pos).map(|node| {
            (node.resource_type, node.is_depleted, node.current_amount)
        });

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

        // 采集资源（每次固定 1 个）
        let gather_amount = 1u32;
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

        // 检查库存上限
        let current_inv = self.agents.get(agent_id)
            .and_then(|a| a.inventory.get(&resource_key).copied())
            .unwrap_or(0);
        let limit = effective_limit as u32;

        if current_inv + gathered > limit {
            return ActionResult::Blocked(
                format!("{} 此种资源已满（当前 x{}，上限 {}）。无法采集",
                    resource_key, current_inv, limit));
        }

        // 执行更新
        let remain_amount = {
            let node = self.resources.get_mut(&pos).unwrap();
            node.current_amount = node.current_amount.saturating_sub(actual_gather);
            node.current_amount
        };

        // 更新库存
        let new_inv = current_inv + gathered;
        let agent = self.agents.get_mut(agent_id).unwrap();
        agent.inventory.insert(resource_key.clone(), new_inv);

        // 记录事件
        self.record_event(agent_id, &agent_name, EventType::Gather.as_str(),
            &builder.gather(resource, gathered), EventType::Gather.color_code());

        ActionResult::SuccessWithDetail(format!("gather:{}x{},node_remain:{},inv:{}→{}",
            resource_key, gathered, remain_amount, current_inv, new_inv))
    }

    /// Wait：原地等待
    pub fn handle_wait(&mut self, agent_id: &AgentId) -> ActionResult {
        let agent = self.agents.get_mut(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());

        self.record_event(agent_id, &agent_name, EventType::Wait.as_str(),
            &builder.wait(), EventType::Wait.color_code());
        ActionResult::SuccessWithDetail("wait".into())
    }

    /// Eat：消耗食物恢复饱食度
    pub fn handle_eat(&mut self, agent_id: &AgentId) -> ActionResult {
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());

        let agent = self.agents.get_mut(agent_id).unwrap();
        let (success, delta, before, after, remain) = agent.eat_food();

        if success {
            self.record_event(agent_id, &agent_name, EventType::Eat.as_str(),
                &builder.eat(delta), EventType::Eat.color_code());
            ActionResult::SuccessWithDetail(format!("eat:satiety+{}({}→{}),food_remain={}",
                delta, before, after, remain))
        } else {
            let inventory_str: Vec<String> = agent.inventory.iter()
                .map(|(r, n)| format!("{} x{}", r, n))
                .collect();
            ActionResult::Blocked(
                format!("背包中没有food。当前背包：{}",
                    if inventory_str.is_empty() { "空".to_string() } else { inventory_str.join(", ") }))
        }
    }

    /// Drink：消耗水恢复水分度
    pub fn handle_drink(&mut self, agent_id: &AgentId) -> ActionResult {
        let agent_name = self.agents.get(agent_id).unwrap().name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());

        let agent = self.agents.get_mut(agent_id).unwrap();
        let (success, delta, before, after, remain) = agent.drink_water();

        if success {
            self.record_event(agent_id, &agent_name, EventType::Drink.as_str(),
                &builder.drink(delta), EventType::Drink.color_code());
            ActionResult::SuccessWithDetail(format!("drink:hydration+{}({}→{}),water_remain={}",
                delta, before, after, remain))
        } else {
            let inventory_str: Vec<String> = agent.inventory.iter()
                .map(|(r, n)| format!("{} x{}", r, n))
                .collect();
            ActionResult::Blocked(
                format!("背包中没有water。当前背包：{}",
                    if inventory_str.is_empty() { "空".to_string() } else { inventory_str.join(", ") }))
        }
    }

    /// Build：消耗资源建造建筑
    pub fn handle_build(&mut self, agent_id: &AgentId, structure: StructureType) -> ActionResult {
        let agent = self.agents.get_mut(agent_id).unwrap();
        let agent_name = agent.name.clone();
        let builder = NarrativeBuilder::new(agent_name.clone());
        let pos = agent.position;

        if self.structures.contains_key(&pos) {
            return ActionResult::Blocked("目标位置已有建筑，无法在此建造".into());
        }

        let cost = structure.resource_cost();
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
}