//! vision 模块单元测试：scan_vision、agent_positions 反向索引

#[cfg(test)]
mod tests {
    use agentora_core::vision::scan_vision;
    use agentora_core::world::World;
    use agentora_core::seed::WorldSeed;
    use agentora_core::types::{Position, ActionType, Action, Direction};
    use agentora_core::agent::{Relation, RelationType};
    use std::collections::HashMap;

    fn create_test_world() -> World {
        let seed = WorldSeed {
            map_size: [64, 64],
            terrain_ratio: HashMap::from([
                ("plains".to_string(), 0.8),
                ("forest".to_string(), 0.2),
            ]),
            resource_density: 0.05,
            region_size: 16,
            initial_agents: 3,
            motivation_templates: HashMap::from([
                ("gatherer".to_string(), [0.8, 0.4, 0.3, 0.2, 0.3, 0.2]),
            ]),
            spawn_strategy: "scattered".to_string(),
            seed_peers: vec![],
            pressure_config: agentora_core::seed::PressureConfig::default(),
        };
        World::new(&seed)
    }

    // 任务 6.1: scan_vision 覆盖四个象限，验证圆形扫描正确
    #[test]
    fn test_scan_vision_four_quadrants() {
        let mut world = create_test_world();

        // 获取第一个 Agent 的位置
        let agent_id = world.agents.keys().next().unwrap().clone();
        let agent_pos = world.agents.get(&agent_id).unwrap().position;

        // 在四个象限放置资源（距离 Agent 2-3 格，确保在 radius=5 内）
        let mut positions = Vec::new();

        // 东北 (+x, +y)
        let ne = Position::new((agent_pos.x + 2).min(63), (agent_pos.y + 2).min(63));
        if ne.manhattan_distance(&agent_pos) <= 5 && ne != agent_pos {
            positions.push(ne);
        }
        // 东南 (+x, -y)
        if agent_pos.y >= 2 {
            let se = Position::new((agent_pos.x + 2).min(63), agent_pos.y - 2);
            if se.manhattan_distance(&agent_pos) <= 5 && se != agent_pos {
                positions.push(se);
            }
        }
        // 西北 (-x, +y)
        if agent_pos.x >= 2 {
            let nw = Position::new(agent_pos.x - 2, (agent_pos.y + 2).min(63));
            if nw.manhattan_distance(&agent_pos) <= 5 && nw != agent_pos {
                positions.push(nw);
            }
        }
        // 西南 (-x, -y)
        if agent_pos.x >= 2 && agent_pos.y >= 2 {
            let sw = Position::new(agent_pos.x - 2, agent_pos.y - 2);
            if sw.manhattan_distance(&agent_pos) <= 5 && sw != agent_pos {
                positions.push(sw);
            }
        }

        assert!(positions.len() >= 3, "至少应有 3 个测试位置");

        for pos in &positions {
            let node = agentora_core::world::resource::ResourceNode::new(
                *pos,
                agentora_core::types::ResourceType::Food,
                100,
            );
            world.resources.insert(*pos, node);
        }

        let result = scan_vision(&world, &agent_id, 5);

        // 验证所有放置的资源都被发现
        let found_positions: Vec<_> = result.resources_at.keys().cloned().collect();
        for pos in &positions {
            assert!(found_positions.contains(pos), "资源应在视野内: ({}, {}), 距离={}",
                pos.x, pos.y, pos.manhattan_distance(&agent_pos));
        }

        println!("[test] scan_vision 发现 {} 个资源点（共 {} 个测试位置，Agent 在 {:?}）",
            result.resources_at.len(), positions.len(), agent_pos);
    }

    // 任务 6.2: scan_vision 返回的 nearby_agents 包含关系数据
    #[test]
    fn test_scan_vision_nearby_agents_relation() {
        let mut world = create_test_world();

        let agent_ids: Vec<_> = world.agents.keys().cloned().collect();
        if agent_ids.len() < 2 {
            println!("[test] 跳过：Agent 数量不足");
            return;
        }

        let agent_id = agent_ids[0].clone();
        let other_id = agent_ids[1].clone();

        // 把另一个 Agent 移到附近
        let agent = world.agents.get(&agent_id).unwrap();
        let nearby_pos = Position::new(agent.position.x + 2, agent.position.y);

        // 先记录旧位置
        let old_other_pos = world.agents.get(&other_id).unwrap().position;

        // 更新其他 Agent 的位置
        {
            let other = world.agents.get_mut(&other_id).unwrap();
            other.position = nearby_pos;
        }

        // 更新 agent_positions
        // 从旧位置移除
        if let Some(ids) = world.agent_positions.get_mut(&old_other_pos) {
            ids.retain(|id| *id != other_id);
            if ids.is_empty() {
                world.agent_positions.remove(&old_other_pos);
            }
        }
        // 加入新位置
        world.agent_positions.entry(nearby_pos).or_default().push(other_id.clone());

        // 建立盟友关系
        {
            let agent = world.agents.get_mut(&agent_id).unwrap();
            agent.relations.insert(other_id.clone(), Relation {
                trust: 50.0,
                relation_type: RelationType::Ally,
                last_interaction_tick: 0,
            });
        }

        let result = scan_vision(&world, &agent_id, 5);

        // 验证返回的 Agent 包含关系信息
        assert!(!result.nearby_agents.is_empty(), "应发现附近 Agent");
        let info = &result.nearby_agents[0];
        assert!(!info.name.is_empty(), "Agent 名称不应为空");
        assert!(info.distance <= 5, "距离应在视野半径内");
        assert_eq!(info.relation_type, RelationType::Ally, "关系类型应为 Ally");
        assert!((info.trust - 50.0).abs() < 0.01, "信任值应匹配");
        println!("[test] 发现 Agent: {}, 关系: {:?}, 信任: {}", info.name, info.relation_type, info.trust);
    }

    // 任务 6.3: scan_vision 返回的 resources_at 包含数量信息
    #[test]
    fn test_scan_vision_resources_with_amount() {
        let mut world = create_test_world();

        let agent_id = world.agents.keys().next().unwrap().clone();
        let agent_pos = world.agents.get(&agent_id).unwrap().position;

        // 在附近放置已知数量的资源
        let resource_pos = Position::new((agent_pos.x + 1).min(63), agent_pos.y);
        let expected_amount = 150u32;
        let node = agentora_core::world::resource::ResourceNode::new(
            resource_pos,
            agentora_core::types::ResourceType::Iron,
            expected_amount,
        );
        world.resources.insert(resource_pos, node);

        let result = scan_vision(&world, &agent_id, 5);

        assert!(result.resources_at.contains_key(&resource_pos), "资源应在视野内");
        let (resource_type, amount) = result.resources_at.get(&resource_pos).unwrap();
        assert_eq!(*resource_type, agentora_core::types::ResourceType::Iron);
        assert_eq!(*amount, expected_amount, "资源数量应匹配");
    }

    // 任务 6.4: agent_positions 在 Move 后保持一致
    #[test]
    fn test_agent_positions_consistent_after_move() {
        let mut world = create_test_world();

        let agent_id = world.agents.keys().next().unwrap().clone();
        let old_pos = world.agents.get(&agent_id).unwrap().position;

        // 验证初始位置索引存在
        assert!(world.agent_positions.contains_key(&old_pos), "初始位置应有索引");

        // 创建 Move 动作
        let action = Action {
            reasoning: "测试移动".to_string(),
            action_type: ActionType::Move { direction: Direction::East },
            target: None,
            params: HashMap::new(),
            motivation_delta: [0.0; 6],
        };

        world.apply_action(&agent_id, &action);

        let new_pos = world.agents.get(&agent_id).unwrap().position;

        // 验证新位置已更新
        if new_pos != old_pos {
            assert!(
                world.agent_positions.get(&new_pos).map(|ids| ids.contains(&agent_id)).unwrap_or(false),
                "新位置应包含 Agent ID"
            );
            assert!(
                world.agent_positions.get(&old_pos).map(|ids| !ids.contains(&agent_id)).unwrap_or(true),
                "旧位置不应包含 Agent ID"
            );
        }
    }

    // 任务 6.5: agent_positions 在 Explore 后保持一致
    #[test]
    fn test_agent_positions_consistent_after_explore() {
        let mut world = create_test_world();

        let agent_id = world.agents.keys().next().unwrap().clone();
        let old_pos = world.agents.get(&agent_id).unwrap().position;

        let action = Action {
            reasoning: "测试探索".to_string(),
            action_type: ActionType::Explore { target_region: 0 },
            target: None,
            params: HashMap::new(),
            motivation_delta: [0.0; 6],
        };

        world.apply_action(&agent_id, &action);

        let new_pos = world.agents.get(&agent_id).unwrap().position;

        // Explore 可能移动了位置
        if new_pos != old_pos {
            assert!(
                world.agent_positions.get(&new_pos).map(|ids| ids.contains(&agent_id)).unwrap_or(false),
                "新位置应包含 Agent ID (Explore 后)"
            );
        }
    }

    // 任务 6.6: agent_positions 在 Agent 生成/死亡后保持一致
    #[test]
    fn test_agent_positions_after_death() {
        let mut world = create_test_world();

        let agent_id = world.agents.keys().next().unwrap().clone();
        let agent_pos = world.agents.get(&agent_id).unwrap().position;

        // 设置 health = 0 但保持 is_alive = true，让 advance_tick 检测到
        {
            let agent = world.agents.get_mut(&agent_id).unwrap();
            agent.health = 0;
            // 不要设置 is_alive = false，让 check_agent_death 来处理
        }
        world.advance_tick();

        // 验证死亡 Agent 的位置记录已清理
        let agent = world.agents.get(&agent_id);
        if let Some(a) = agent {
            if !a.is_alive {
                let ids_at_pos = world.agent_positions.get(&agent_pos);
                if let Some(ids) = ids_at_pos {
                    assert!(!ids.contains(&agent_id), "死亡 Agent 不应在位置索引中");
                }
            }
        }
    }

    // 任务 6.7: insert_agent_at() 同时更新 agents 和 agent_positions
    #[test]
    fn test_insert_agent_at() {
        let mut world = create_test_world();

        let pos = Position::new(30, 30);
        let new_id = agentora_core::types::AgentId::new("test_new_agent");
        let agent = agentora_core::agent::Agent::new(
            new_id.clone(),
            "TestAgent".to_string(),
            pos,
        );

        world.insert_agent_at(new_id.clone(), agent);

        // 验证 agents 中有
        assert!(world.agents.contains_key(&new_id), "Agent 应在 agents 中");

        // 验证 agent_positions 中有
        let ids_at_pos = world.agent_positions.get(&pos).expect("位置应有索引");
        assert!(ids_at_pos.contains(&new_id), "位置索引应包含新 Agent");
    }
}
