//! 集成测试 - 遗产系统
//!
//! 测试 Agent 死亡→遗迹→回响→他人交互的完整闭环

use agentora_core::{
    Agent, AgentId, Position, World, WorldSeed,
    ActionType, Action,
    legacy::{Legacy, LegacyEvent},
    types::LegacyInteraction,
    motivation::MotivationVector,
};
use std::collections::HashMap;

/// 单 Agent 死亡测试
#[test]
fn test_agent_death_creates_legacy() {
    // 6.1 单 Agent 死亡测试
    let seed = WorldSeed::default();
    let mut world = World::new(&seed);

    // 创建一个即将死亡的 Agent（年龄=max_age）
    let agent_id = AgentId::new("test-agent-001");
    let position = Position::new(10, 10);
    let mut agent = Agent::new(agent_id.clone(), "Test Agent".to_string(), position);
    agent.age = agent.max_age; // 设置为最大年龄，下一次 tick 应该死亡
    agent.is_alive = true;

    world.agents.insert(agent_id.clone(), agent);

    println!("=== 单 Agent 死亡测试 ===");
    println!("Agent 初始状态：age={}, is_alive={}",
             world.agents.get(&agent_id).unwrap().age,
             world.agents.get(&agent_id).unwrap().is_alive);
    println!("初始遗产数量：{}", world.legacies.len());

    // 推进 tick，触发死亡检测
    world.advance_tick();

    // 验证 Agent 已死亡
    let agent = world.agents.get(&agent_id).unwrap();
    assert!(!agent.is_alive, "Agent 应该已死亡");

    // 验证产生了遗产
    assert_eq!(world.legacies.len(), 1, "应该产生 1 个遗产");

    let legacy = &world.legacies[0];
    assert_eq!(legacy.original_agent_id, agent_id);
    assert_eq!(legacy.position, position);
    assert_eq!(legacy.created_tick, 1);

    println!("✅ Agent 死亡后产生遗产：id={}, position=({}, {})",
             legacy.id, legacy.position.x, legacy.position.y);
    println!("✅ 遗产物品：{:?}", legacy.items);
    if let Some(echo) = &legacy.echo_log {
        println!("✅ 回响日志：{}", echo.summary);
    }
}

/// 多 Agent 遗产交互测试
#[test]
fn test_multi_agent_legacy_interaction() {
    // 6.2 多 Agent 遗产交互测试
    let seed = WorldSeed::default();
    let mut world = World::new(&seed);

    // 创建两个 Agent
    let agent1_id = AgentId::new("agent-001");
    let agent2_id = AgentId::new("agent-002");
    let position = Position::new(10, 10);

    let mut agent1 = Agent::new(agent1_id.clone(), "Agent One".to_string(), position);
    agent1.age = agent1.max_age;
    world.agents.insert(agent1_id.clone(), agent1);

    let mut agent2 = Agent::new(agent2_id.clone(), "Agent Two".to_string(), Position::new(10, 10));
    world.agents.insert(agent2_id.clone(), agent2);

    // 推进 tick，让 agent1 死亡产生遗产
    world.advance_tick();

    assert_eq!(world.legacies.len(), 1, "应该有 1 个遗产");
    let legacy_id = world.legacies[0].id.clone();

    println!("=== 多 Agent 遗产交互测试 ===");
    println!("遗产 ID: {}", legacy_id);

    // 测试祭拜动作
    let worship_action = Action {
        reasoning: "祭拜逝去的 Agent".to_string(),
        action_type: ActionType::InteractLegacy {
            legacy_id: legacy_id.clone(),
            interaction: LegacyInteraction::Worship
        },
        target: Some(legacy_id.clone()),
        params: HashMap::new(),
        motivation_delta: [0.0; 6],
    };

    let result = world.apply_action(&agent2_id, &worship_action);
    assert!(matches!(result, agentora_core::world::ActionResult::Success),
            "祭拜动作应该成功");

    let agent2 = world.agents.get(&agent2_id).unwrap();
    println!("祭拜后动机：认知={:.2}, 传承={:.2}",
             agent2.motivation.get(2), agent2.motivation.get(5));
    assert!(agent2.motivation.get(2) > 0.5, "认知动机应该增加");
    assert!(agent2.motivation.get(5) > 0.5, "传承动机应该增加");

    println!("✅ 祭拜动作成功，动机得到激励");

    // 测试探索动作
    let explore_action = Action {
        reasoning: "探索遗迹获取回响".to_string(),
        action_type: ActionType::InteractLegacy {
            legacy_id: legacy_id.clone(),
            interaction: LegacyInteraction::Explore
        },
        target: Some(legacy_id.clone()),
        params: HashMap::new(),
        motivation_delta: [0.0; 6],
    };

    let result = world.apply_action(&agent2_id, &explore_action);
    assert!(matches!(result, agentora_core::world::ActionResult::Success),
            "探索动作应该成功");

    let agent2 = world.agents.get(&agent2_id).unwrap();
    println!("探索后认知动机：{:.2}", agent2.motivation.get(2));
    assert!(agent2.motivation.get(2) > 0.55, "探索后认知动机应该继续增加");

    println!("✅ 探索动作成功，认知动机得到激励");
}

/// 遗产广播正确性验证
#[test]
fn test_legacy_broadcast_format() {
    // 6.3 验证遗产广播正确性
    let legacy = Legacy {
        id: "test-legacy-001".to_string(),
        position: Position::new(50, 50),
        legacy_type: agentora_core::legacy::LegacyType::Grave,
        original_agent_id: AgentId::new("dead-agent"),
        original_agent_name: "Dead Agent".to_string(),
        items: HashMap::new(),
        echo_log: None,
        created_tick: 100,
        decay_tick: 150,
    };

    let event = LegacyEvent::from_legacy(&legacy);

    println!("=== 遗产广播格式验证 ===");
    println!("遗产 ID: {}", event.legacy_id);
    println!("原始 Agent: {} ({})", event.original_agent_name, event.original_agent_id.as_str());
    println!("位置：({}, {})", event.position.x, event.position.y);
    println!("创建 tick: {}", event.created_tick);

    // 验证序列化
    let json = serde_json::to_string(&event).unwrap();
    println!("序列化 JSON: {}", json);

    // 验证反序列化
    let parsed: LegacyEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.legacy_id, event.legacy_id);
    assert_eq!(parsed.original_agent_id, event.original_agent_id);

    println!("✅ 遗产事件序列化/反序列化成功");
}

/// 动机反馈效果验证
#[test]
fn test_motivation_feedback_from_legacy() {
    // 6.4 验证动机的反馈效果
    let seed = WorldSeed::default();
    let mut world = World::new(&seed);

    let agent_id = AgentId::new("test-agent");
    let position = Position::new(20, 20);
    let mut agent = Agent::new(agent_id.clone(), "Test Agent".to_string(), position);

    // 设置初始动机为中性值
    agent.motivation = agentora_core::motivation::MotivationVector::new();
    world.agents.insert(agent_id.clone(), agent);

    // 创建测试遗产
    let legacy = Legacy {
        id: "test-legacy".to_string(),
        position,
        legacy_type: agentora_core::legacy::LegacyType::Grave,
        original_agent_id: AgentId::new("other-agent"),
        original_agent_name: "Other Agent".to_string(),
        items: HashMap::new(),
        echo_log: None,
        created_tick: 0,
        decay_tick: 50,
    };
    world.legacies.push(legacy);

    println!("=== 动机反馈效果验证 ===");
    let initial_cognitive = world.agents.get(&agent_id).unwrap().motivation.get(2);
    let initial_legacy = world.agents.get(&agent_id).unwrap().motivation.get(5);
    println!("初始动机：认知={:.2}, 传承={:.2}", initial_cognitive, initial_legacy);

    // 执行祭拜动作
    let worship_action = Action {
        reasoning: "祭拜".to_string(),
        action_type: ActionType::InteractLegacy {
            legacy_id: "test-legacy".to_string(),
            interaction: LegacyInteraction::Worship
        },
        target: Some("test-legacy".to_string()),
        params: HashMap::new(),
        motivation_delta: [0.0; 6],
    };

    world.apply_action(&agent_id, &worship_action);

    let agent = world.agents.get(&agent_id).unwrap();
    let final_cognitive = agent.motivation.get(2);
    let final_legacy = agent.motivation.get(5);

    println!("祭拜后动机：认知={:.2}, 传承={:.2}", final_cognitive, final_legacy);
    println!("认知增加：{:.2}, 传承增加：{:.2}",
             final_cognitive - initial_cognitive,
             final_legacy - initial_legacy);

    // 验证动机增加
    assert!(final_cognitive > initial_cognitive, "认知动机应该增加");
    assert!(final_legacy > initial_legacy, "传承动机应该增加");

    // 验证增加量约为 0.05
    let cognitive_delta = final_cognitive - initial_cognitive;
    let legacy_delta = final_legacy - initial_legacy;
    assert!(cognitive_delta >= 0.04 && cognitive_delta <= 0.06,
            "认知动机增加应约为 0.05");
    assert!(legacy_delta >= 0.04 && legacy_delta <= 0.06,
            "传承动机增加应约为 0.05");

    println!("✅ 动机反馈效果符合预期");
}
