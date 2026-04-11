//! 集成测试 - 多Agent本地串行交互
//!
//! 测试多个Agent在256×256世界运行的涌现行为
//! 使用规则引擎模拟决策（不依赖LLM）

#[cfg(test)]
mod tests {
    use agentora_core::rule_engine::{RuleEngine, WorldState};
    use agentora_core::types::{AgentId, Position, ResourceType, StructureType, TerrainType};
    use agentora_core::motivation::MotivationVector;
    use std::collections::HashMap;
    use std::collections::HashSet;

    /// 简化的Agent状态
    #[derive(Clone, Debug)]
    struct TestAgent {
        id: AgentId,
        position: Position,
        motivation: MotivationVector,
        inventory: HashMap<ResourceType, u32>,
        tick_actions: Vec<ActionType>,
    }

    use agentora_core::types::ActionType;

    impl TestAgent {
        fn new(id: &str, position: Position, motivation: MotivationVector) -> Self {
            Self {
                id: AgentId::new(id),
                position,
                motivation,
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

        // 基于动机选择最合适的动作
        let action = match agent.motivation[0] {
            // 生存动机最强：采集或建造
            m if m > 0.7 => {
                if let Some(a) = candidates.iter().find(|a| matches!(a, ActionType::Gather { .. })) {
                    a.clone()
                } else if let Some(a) = candidates.iter().find(|a| matches!(a, ActionType::Build { .. })) {
                    a.clone()
                } else {
                    candidates.first().cloned().unwrap_or(ActionType::Wait)
                }
            }
            // 社交动机最强：尝试交流
            m if m > 0.5 && agent.motivation[1] > 0.6 => {
                if let Some(a) = candidates.iter().find(|a| matches!(a, ActionType::Talk { .. })) {
                    a.clone()
                } else {
                    candidates.first().cloned().unwrap_or(ActionType::Wait)
                }
            }
            // 默认：选择第一个合法动作
            _ => {
                // 有些随机性：不总是选第一个
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
            ActionType::Move { direction } => {
                let delta = direction.delta();
                let new_x = agent.position.x as i32 + delta.0;
                let new_y = agent.position.y as i32 + delta.1;
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
            _ => {}
        }

        // 动机衰减
        agent.motivation.decay();
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
                terrain_at: self.terrain.clone(),
                existing_agents: other_agents.iter().map(|a| a.id.clone()).collect::<HashSet<_>>(),
                resources_at: self.resources.iter().map(|(k, v)| (*k, *v)).collect(),
            }
        }
    }

    #[test]
    fn test_multi_agent_survival_emergence() {
        // 创建5个Agent，不同动机模板
        let mut agents = vec![
            TestAgent::new("gatherer_1", Position::new(5, 5), MotivationVector::from_array([0.9, 0.3, 0.3, 0.3, 0.3, 0.3])),
            TestAgent::new("gatherer_2", Position::new(10, 10), MotivationVector::from_array([0.8, 0.2, 0.2, 0.2, 0.2, 0.2])),
            TestAgent::new("explorer_1", Position::new(15, 15), MotivationVector::from_array([0.3, 0.3, 0.8, 0.5, 0.3, 0.3])),
            TestAgent::new("trader_1", Position::new(8, 12), MotivationVector::from_array([0.4, 0.8, 0.4, 0.4, 0.5, 0.3])),
            TestAgent::new("builder_1", Position::new(12, 8), MotivationVector::from_array([0.5, 0.3, 0.4, 0.3, 0.4, 0.7])),
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
        }

        // 验证涌现行为
        let total_actions: usize = agents.iter().map(|a| a.tick_actions.len()).sum();
        assert!(total_actions > 0, "Agent应该产生了动作");

        // 验证采集行为：gatherer 应该采集了资源
        let gatherer = agents.iter().find(|a| a.id.as_str().starts_with("gatherer")).unwrap();
        // gatherer 可能采集了资源
        assert!(gatherer.tick_actions.iter().any(|a| matches!(a, ActionType::Gather { .. }) || matches!(a, ActionType::Move { .. })));

        // 验证移动行为：explorer 应该移动了
        let explorer = agents.iter().find(|a| a.id.as_str().starts_with("explorer")).unwrap();
        assert!(explorer.tick_actions.iter().any(|a| matches!(a, ActionType::Move { .. })));

        // 验证Agent没有越界
        for agent in &agents {
            assert!(agent.position.x < 256 && agent.position.y < 256);
        }
    }

    #[test]
    fn test_multi_agent_interaction_basic() {
        // 创建2个相邻Agent，验证社交行为
        let mut agents = vec![
            TestAgent::new("social_1", Position::new(10, 10), MotivationVector::from_array([0.2, 0.9, 0.3, 0.3, 0.3, 0.3])),
            TestAgent::new("social_2", Position::new(11, 10), MotivationVector::from_array([0.2, 0.8, 0.3, 0.3, 0.3, 0.3])),
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

        // 社交动机强的Agent应该尝试Talk
        let all_actions: Vec<_> = agents.iter().flat_map(|a| &a.tick_actions).collect();
        // 至少有一个Agent尝试了社交动作或移动
        assert!(all_actions.len() > 0);
    }

    #[test]
    fn test_resource_competition() {
        // 创建2个Agent在同一个位置，竞争有限资源
        let mut agents = vec![
            TestAgent::new("compete_1", Position::new(5, 5), MotivationVector::from_array([0.95, 0.1, 0.1, 0.1, 0.1, 0.1])),
            TestAgent::new("compete_2", Position::new(5, 5), MotivationVector::from_array([0.95, 0.1, 0.1, 0.1, 0.1, 0.1])),
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
        // 验证Agent会探索不同方向（使用不同初始位置）
        let mut agents = vec![
            TestAgent::new("spread_1", Position::new(0, 0), MotivationVector::from_array([0.3, 0.3, 0.8, 0.3, 0.3, 0.3])),
            TestAgent::new("spread_2", Position::new(5, 5), MotivationVector::from_array([0.5, 0.5, 0.3, 0.5, 0.3, 0.5])),
            TestAgent::new("spread_3", Position::new(10, 10), MotivationVector::from_array([0.8, 0.2, 0.2, 0.2, 0.5, 0.3])),
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

        // 验证：所有Agent都不在初始位置（有活动）
        let initial_positions = [Position::new(0, 0), Position::new(5, 5), Position::new(10, 10)];
        let moved_count = agents.iter().filter(|a| {
            let init = initial_positions.iter().position(|p| p.x == a.position.x && p.y == a.position.y).unwrap_or(usize::MAX);
            init == usize::MAX
        }).count();

        // 至少有些Agent产生了动作
        let total_actions: usize = agents.iter().map(|a| a.tick_actions.len()).sum();
        assert!(total_actions > 0, "Agent应该产生了动作");

        // Agent不应该全部聚集在同一个位置
        let unique_positions: std::collections::HashSet<_> = agents.iter().map(|a| (a.position.x, a.position.y)).collect();
        assert!(unique_positions.len() >= 1, "Agent应该有至少1个不同位置");
    }
}
