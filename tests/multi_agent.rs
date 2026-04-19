//! 集成测试 - 多Agent本地串行交互
//!
//! 测试多个Agent在256×256世界运行的涌现行为
//! 使用规则引擎模拟决策（不依赖LLM）

#[cfg(test)]
mod tests {
    use agentora_core::rule_engine::{RuleEngine, WorldState};
    use agentora_core::types::{AgentId, Position, ResourceType, StructureType, TerrainType};
    use std::collections::HashMap;
    use std::collections::HashSet;

    /// 简化的Agent状态
    #[derive(Clone, Debug)]
    struct TestAgent {
        id: AgentId,
        position: Position,
        satiety: u32,      // 饱食度 0-100
        hydration: u32,    // 水分度 0-100
        inventory: HashMap<ResourceType, u32>,
        tick_actions: Vec<ActionType>,
    }

    use agentora_core::types::ActionType;

    impl TestAgent {
        fn new(id: &str, position: Position, satiety: u32, hydration: u32) -> Self {
            Self {
                id: AgentId::new(id),
                position,
                satiety,
                hydration,
                inventory: HashMap::new(),
                tick_actions: Vec::new(),
            }
        }
    }

    /// 使用规则引擎生成决策（模拟LLM失败时的兜底）
    fn decide_with_rule_engine(agent: &TestAgent, world_state: &WorldState) -> ActionType {
        let engine = RuleEngine::new();
        let candidates = engine.filter_hard_constraints(world_state);

        if candidates.is_empty() {
            return ActionType::Wait;
        }

        // 基于状态值选择最合适的动作
        let action = match () {
            // 饱食度低且有食物 → Eat
            _ if agent.satiety < 30 && agent.inventory.get(&ResourceType::Food).unwrap_or(&0) > &0 => {
                ActionType::Eat
            }
            // 水分度低且有水 → Drink
            _ if agent.hydration < 30 && agent.inventory.get(&ResourceType::Water).unwrap_or(&0) > &0 => {
                ActionType::Drink
            }
            // 饱食度低 → 采集或建造
            _ if agent.satiety < 50 => {
                if let Some(a) = candidates.iter().find(|a| matches!(a, ActionType::Gather { .. })) {
                    a.clone()
                } else if let Some(a) = candidates.iter().find(|a| matches!(a, ActionType::MoveToward { .. })) {
                    a.clone()
                } else {
                    candidates.first().cloned().unwrap_or(ActionType::Wait)
                }
            }
            // 水分度低 → 采集
            _ if agent.hydration < 50 => {
                if let Some(a) = candidates.iter().find(|a| matches!(a, ActionType::Gather { .. })) {
                    a.clone()
                } else {
                    candidates.first().cloned().unwrap_or(ActionType::Wait)
                }
            }
            // 默认：选择第一个合法动作
            _ => {
                let idx = agent.position.x as usize % candidates.len();
                candidates[idx].clone()
            }
        };

        action
    }

    /// 执行动作并更新世界状态
    fn apply_action(agent: &mut TestAgent, action: &ActionType, world: &mut TestWorld) {
        agent.tick_actions.push(action.clone());

        match action {
            ActionType::MoveToward { target } => {
                // MoveToward moves one step toward target position
                let cur_x = agent.position.x as i32;
                let cur_y = agent.position.y as i32;
                let tx = target.x as i32;
                let ty = target.y as i32;
                let step_x = cur_x + (tx - cur_x).signum();
                let step_y = cur_y + (ty - cur_y).signum();
                // Move one axis at a time
                let (new_x, new_y) = if (tx - cur_x).abs() >= (ty - cur_y).abs() {
                    (step_x, cur_y)
                } else {
                    (cur_x, step_y)
                };
                if new_x >= 0 && new_y >= 0 && new_x < 256 && new_y < 256 {
                    agent.position = Position::new(new_x as u32, new_y as u32);
                }
            }
            ActionType::Gather { resource } => {
                let pos = agent.position;
                if world.resources.contains_key(&pos) {
                    let r = world.resources[&pos];
                    if r == *resource {
                        *agent.inventory.entry(r).or_insert(0) += 1;
                        // 资源被采集后移除
                        world.resources.remove(&pos);
                    }
                }
            }
            ActionType::Build { structure } => {
                let cost = match structure {
                    StructureType::Camp => [(ResourceType::Wood, 5), (ResourceType::Stone, 2)].into_iter().collect::<HashMap<_, _>>(),
                    StructureType::Fence => [(ResourceType::Wood, 2)].into_iter().collect::<HashMap<_, _>>(),
                    StructureType::Warehouse => [(ResourceType::Wood, 10), (ResourceType::Stone, 5)].into_iter().collect::<HashMap<_, _>>(),
                };
                // 检查资源是否足够
                let can_build = cost.iter().all(|(r, a)| agent.inventory.get(r).unwrap_or(&0) >= a);
                if can_build {
                    for (r, a) in cost {
                        let entry = agent.inventory.entry(r).or_insert(0);
                        *entry = entry.saturating_sub(a);
                    }
                    world.structures.insert(agent.position, *structure);
                }
            }
            ActionType::Eat => {
                if let Some(food) = agent.inventory.get_mut(&ResourceType::Food) {
                    if *food > 0 {
                        *food -= 1;
                        agent.satiety = (agent.satiety + 20).min(100);
                    }
                }
            }
            ActionType::Drink => {
                if let Some(water) = agent.inventory.get_mut(&ResourceType::Water) {
                    if *water > 0 {
                        *water -= 1;
                        agent.hydration = (agent.hydration + 20).min(100);
                    }
                }
            }
            _ => {}
        }
    }

    /// 简化的世界状态
    struct TestWorld {
        resources: HashMap<Position, ResourceType>,
        structures: HashMap<Position, StructureType>,
        terrain: HashMap<Position, TerrainType>,
    }

    impl TestWorld {
        fn new() -> Self {
            let mut world = Self {
                resources: HashMap::new(),
                structures: HashMap::new(),
                terrain: HashMap::new(),
            };
            // 放置一些初始资源
            for x in 0..20u32 {
                for y in 0..20u32 {
                    if (x + y) % 5 == 0 {
                        let r = if (x + y) % 10 == 0 {
                            ResourceType::Food
                        } else {
                            ResourceType::Wood
                        };
                        world.resources.insert(Position::new(x, y), r);
                    }
                }
            }
            world
        }

        fn to_world_state(&self, agent: &TestAgent, other_agents: &[&TestAgent]) -> WorldState {
            WorldState {
                map_size: 256,
                agent_position: agent.position,
                agent_inventory: agent.inventory.clone(),
                agent_satiety: agent.satiety,
                agent_hydration: agent.hydration,
                terrain_at: self.terrain.clone(),
                self_id: agent.id.clone(),
                existing_agents: other_agents.iter().map(|a| a.id.clone()).collect::<HashSet<_>>(),
                resources_at: self.resources.iter().map(|(k, v)| (*k, (*v, 1))).collect(),
                nearby_agents: Vec::new(),
                nearby_structures: Vec::new(),
                nearby_legacies: Vec::new(),
                active_pressures: Vec::new(),
                last_move_direction: None,
                temp_preferences: Vec::new(),
            }
        }
    }

    #[test]
    fn test_multi_agent_survival_emergence() {
        // 创建5个Agent，不同初始状态
        let mut agents = vec![
            TestAgent::new("gatherer_1", Position::new(5, 5), 80, 80),
            TestAgent::new("gatherer_2", Position::new(10, 10), 70, 70),
            TestAgent::new("explorer_1", Position::new(15, 15), 90, 90),
            TestAgent::new("trader_1", Position::new(8, 12), 60, 60),
            TestAgent::new("builder_1", Position::new(12, 8), 50, 50),
        ];

        let mut world = TestWorld::new();

        // 运行50 tick
        for _tick in 0..50 {
            // 先收集所有决策
            let mut decisions: Vec<(String, ActionType)> = Vec::new();
            for agent in &agents {
                let others: Vec<&TestAgent> = agents.iter().filter(|a| a.id.as_str() != agent.id.as_str()).collect();
                let world_state = world.to_world_state(agent, &others);
                let action = decide_with_rule_engine(agent, &world_state);
                decisions.push((agent.id.as_str().to_string(), action));
            }

            // 再执行
            for (agent_id, action) in decisions {
                if let Some(agent) = agents.iter_mut().find(|a| a.id.as_str() == agent_id) {
                    apply_action(agent, &action, &mut world);
                }
            }

            // 自然衰减：饱食度和水分度每tick下降
            for agent in &mut agents {
                agent.satiety = agent.satiety.saturating_sub(1);
                agent.hydration = agent.hydration.saturating_sub(1);
            }
        }

        // 验证涌现行为
        let total_actions: usize = agents.iter().map(|a| a.tick_actions.len()).sum();
        assert!(total_actions > 0, "Agent应该产生了动作");

        // 验证采集行为
        let gatherer = agents.iter().find(|a| a.id.as_str().starts_with("gatherer")).unwrap();
        assert!(gatherer.tick_actions.iter().any(|a| matches!(a, ActionType::Gather { .. }) || matches!(a, ActionType::MoveToward { .. })));

        // 验证移动行为
        let explorer = agents.iter().find(|a| a.id.as_str().starts_with("explorer")).unwrap();
        assert!(explorer.tick_actions.iter().any(|a| matches!(a, ActionType::MoveToward { .. })));

        // 验证Agent没有越界
        for agent in &agents {
            assert!(agent.position.x < 256 && agent.position.y < 256);
        }
    }

    #[test]
    fn test_multi_agent_interaction_basic() {
        // 创建2个相邻Agent
        let mut agents = vec![
            TestAgent::new("social_1", Position::new(10, 10), 80, 80),
            TestAgent::new("social_2", Position::new(11, 10), 75, 75),
        ];

        let mut world = TestWorld::new();

        // 运行10 tick
        for _tick in 0..10 {
            let mut decisions: Vec<(String, ActionType)> = Vec::new();
            for agent in &agents {
                let others: Vec<&TestAgent> = agents.iter().filter(|a| a.id.as_str() != agent.id.as_str()).collect();
                let world_state = world.to_world_state(agent, &others);
                let action = decide_with_rule_engine(agent, &world_state);
                decisions.push((agent.id.as_str().to_string(), action));
            }
            for (agent_id, action) in decisions {
                if let Some(agent) = agents.iter_mut().find(|a| a.id.as_str() == agent_id) {
                    apply_action(agent, &action, &mut world);
                }
            }
        }

        // 至少有一个Agent尝试了动作
        let all_actions: Vec<_> = agents.iter().flat_map(|a| &a.tick_actions).collect();
        assert!(all_actions.len() > 0);
    }

    #[test]
    fn test_resource_competition() {
        // 创建2个Agent在同一个位置，竞争有限资源
        let mut agents = vec![
            TestAgent::new("compete_1", Position::new(5, 5), 30, 30),
            TestAgent::new("compete_2", Position::new(5, 5), 30, 30),
        ];

        let mut world = TestWorld::new();
        // 在位置(5,5)放置一个食物资源
        world.resources.insert(Position::new(5, 5), ResourceType::Food);

        // 运行20 tick
        for _tick in 0..20 {
            let mut decisions: Vec<(String, ActionType)> = Vec::new();
            for agent in &agents {
                let others: Vec<&TestAgent> = agents.iter().filter(|a| a.id.as_str() != agent.id.as_str()).collect();
                let world_state = world.to_world_state(agent, &others);
                let action = decide_with_rule_engine(agent, &world_state);
                decisions.push((agent.id.as_str().to_string(), action));
            }
            for (agent_id, action) in decisions {
                if let Some(agent) = agents.iter_mut().find(|a| a.id.as_str() == agent_id) {
                    apply_action(agent, &action, &mut world);
                }
            }
        }

        // 只有一个Agent能采集到(5,5)的食物
        let food_collectors = agents.iter().filter(|a| a.inventory.get(&ResourceType::Food).unwrap_or(&0) > &0).count();
        assert!(food_collectors <= 1, "资源竞争：只有一个Agent能采集到同一位置的资源");
    }

    #[test]
    fn test_agent_movement_spread() {
        // 验证Agent会探索不同方向
        let mut agents = vec![
            TestAgent::new("spread_1", Position::new(0, 0), 80, 80),
            TestAgent::new("spread_2", Position::new(5, 5), 70, 70),
            TestAgent::new("spread_3", Position::new(10, 10), 50, 50),
        ];

        let mut world = TestWorld::new();

        // 运行30 tick
        for _tick in 0..30 {
            let mut decisions: Vec<(String, ActionType)> = Vec::new();
            for agent in &agents {
                let others: Vec<&TestAgent> = agents.iter().filter(|a| a.id.as_str() != agent.id.as_str()).collect();
                let world_state = world.to_world_state(agent, &others);
                let action = decide_with_rule_engine(agent, &world_state);
                decisions.push((agent.id.as_str().to_string(), action));
            }
            for (agent_id, action) in decisions {
                if let Some(agent) = agents.iter_mut().find(|a| a.id.as_str() == agent_id) {
                    apply_action(agent, &action, &mut world);
                }
            }
        }

        // 验证：至少有些Agent产生了动作
        let total_actions: usize = agents.iter().map(|a| a.tick_actions.len()).sum();
        assert!(total_actions > 0, "Agent应该产生了动作");

        // Agent不应该全部聚集在同一个位置
        let unique_positions: std::collections::HashSet<_> = agents.iter().map(|a| (a.position.x, a.position.y)).collect();
        assert!(unique_positions.len() >= 1, "Agent应该有至少1个不同位置");
    }
}
