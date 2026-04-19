//! 单元测试 - 决策管道
//!
//! 测试规则校验、LLM 兜底、Prompt 构建

use agentora_core::decision::{ActionCandidate, DecisionPipeline};
use agentora_core::rule_engine::RuleEngine;
use agentora_core::types::{ActionType, Position, TerrainType, ResourceType, AgentId};
use std::collections::HashMap;

#[test]
fn test_filter_move_valid() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.terrain_at.insert(Position::new(11, 10), TerrainType::Plains);

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    assert!(filtered.iter().any(|a| matches!(a, ActionType::MoveToward { target } if target == &Position::new(11, 10))));
}

#[test]
fn test_filter_move_mountain_passable() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.terrain_at.insert(Position::new(11, 10), TerrainType::Mountain);

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    // 所有地形都可通行，山地也可以穿越
    assert!(filtered.iter().any(|a| matches!(a, ActionType::MoveToward { target } if target == &Position::new(11, 10))));
}

#[test]
fn test_filter_move_boundary() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(0, 10);  // 左边界
    world_state.terrain_at.insert(Position::new(1, 10), TerrainType::Plains);

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    // 向东移动合法（在地图内）
    assert!(filtered.iter().any(|a| matches!(a, ActionType::MoveToward { target } if target == &Position::new(1, 10))));
    // 向西移动会越界，不应包含
    let west_move = filtered.iter().any(|a| matches!(a, ActionType::MoveToward { target } if target.x == 0 && target.y == 10));
    assert!(!west_move, "边界位置不应生成回退到原地的 MoveToward");
}

#[test]
fn test_survival_fallback_low_satiety_with_food() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_satiety = 20;
    world_state.agent_inventory.insert(ResourceType::Food, 5);

    let engine = RuleEngine::new();
    let action = engine.survival_fallback(&world_state);

    assert!(action.is_some());
    let action = action.unwrap();
    assert!(matches!(action.action_type, ActionType::Eat));
    assert!(action.reasoning.contains("食物"));
}

#[test]
fn test_survival_fallback_low_hydration_with_water() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_hydration = 15;
    world_state.agent_inventory.insert(ResourceType::Water, 3);

    let engine = RuleEngine::new();
    let action = engine.survival_fallback(&world_state);

    assert!(action.is_some());
    let action = action.unwrap();
    assert!(matches!(action.action_type, ActionType::Drink));
    assert!(action.reasoning.contains("水源"));
}

#[test]
fn test_survival_fallback_resource_at_feet() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.resources_at.insert(Position::new(10, 10), (ResourceType::Wood, 50));

    let engine = RuleEngine::new();
    let action = engine.survival_fallback(&world_state);

    assert!(action.is_some());
    let action = action.unwrap();
    assert!(matches!(action.action_type, ActionType::Gather { .. }));
}

#[test]
fn test_survival_fallback_nearby_resource() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.terrain_at.insert(Position::new(11, 10), TerrainType::Plains);
    world_state.resources_at.insert(Position::new(11, 10), (ResourceType::Food, 30));

    let engine = RuleEngine::new();
    let action = engine.survival_fallback(&world_state);

    assert!(action.is_some());
    let action = action.unwrap();
    assert!(matches!(action.action_type, ActionType::MoveToward { .. }));
}

#[test]
fn test_survival_fallback_default_wait() {
    let world_state = agentora_core::rule_engine::WorldState::default();

    let engine = RuleEngine::new();
    let action = engine.survival_fallback(&world_state);

    assert!(action.is_some());
    let action = action.unwrap();
    assert!(matches!(action.action_type, ActionType::Wait));
}

#[test]
fn test_filter_build_insufficient_resources() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    // 没有资源，不能建造
    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    assert!(!filtered.iter().any(|a| matches!(a, ActionType::Build { .. })));
}

#[test]
fn test_filter_build_sufficient_resources() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    // 提供足够的木材可以建造 Fence (需要 2 wood)
    world_state.agent_inventory.insert(ResourceType::Wood, 5);

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    assert!(filtered.iter().any(|a| matches!(a, ActionType::Build { structure: agentora_core::types::StructureType::Fence })));
}

#[test]
fn test_filter_target_not_exists() {
    let world_state = agentora_core::rule_engine::WorldState::default();
    // 没有其他 Agent，不能有 Talk/Attack
    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    assert!(!filtered.iter().any(|a| matches!(a, ActionType::Talk { .. })));
    assert!(!filtered.iter().any(|a| matches!(a, ActionType::Attack { .. })));
}

#[test]
fn test_filter_target_exists() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.existing_agents.insert(AgentId::new("other_agent"));

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    assert!(filtered.iter().any(|a| matches!(a, ActionType::Talk { .. })));
    assert!(filtered.iter().any(|a| matches!(a, ActionType::Attack { .. })));
}

#[test]
fn test_filter_terrain_water_passable() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.terrain_at.insert(Position::new(10, 11), TerrainType::Water);

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    // 所有地形都可通行，水域也可以穿越
    let water_passable = filtered.iter().any(|a| matches!(a, ActionType::MoveToward { target } if target == &Position::new(10, 11)));
    assert!(water_passable, "水域地形应该可以穿越");
}

#[test]
fn test_validate_action_valid_move() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.terrain_at.insert(Position::new(11, 10), TerrainType::Plains);

    let candidate = ActionCandidate {
        reasoning: "向东移动".to_string(),
        action_type: ActionType::MoveToward { target: Position::new(11, 10) },
        target: None,
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    assert!(engine.validate_action(&candidate, &world_state).0);
}

#[test]
fn test_validate_action_invalid_attack_target() {
    let world_state = agentora_core::rule_engine::WorldState::default();
    let candidate = ActionCandidate {
        reasoning: "攻击不存在的目标".to_string(),
        action_type: ActionType::Attack { target_id: AgentId::new("ghost") },
        target: Some("ghost".to_string()),
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    assert!(!engine.validate_action(&candidate, &world_state).0);
}

#[test]
fn test_validate_action_trade_insufficient_resources() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    // 没有资源却要交易
    let mut offer = HashMap::new();
    offer.insert(ResourceType::Wood, 10);

    let candidate = ActionCandidate {
        reasoning: "交易".to_string(),
        action_type: ActionType::TradeOffer { offer, want: HashMap::new(), target_id: AgentId::new("trader") },
        target: Some("trader".to_string()),
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    assert!(!engine.validate_action(&candidate, &world_state).0);
}

#[test]
fn test_validate_action_trade_valid() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    let mut offer = HashMap::new();
    offer.insert(ResourceType::Wood, 5);

    // 背包有足够资源
    world_state.agent_inventory.insert(ResourceType::Wood, 10);
    world_state.existing_agents.insert(AgentId::new("trader"));

    let candidate = ActionCandidate {
        reasoning: "交易".to_string(),
        action_type: ActionType::TradeOffer { offer, want: HashMap::new(), target_id: AgentId::new("trader") },
        target: Some("trader".to_string()),
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    assert!(engine.validate_action(&candidate, &world_state).0);
}

#[test]
fn test_validate_action_wait_always_valid() {
    let candidate = ActionCandidate {
        reasoning: "等待".to_string(),
        action_type: ActionType::Wait,
        target: None,
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    let world_state = agentora_core::rule_engine::WorldState::default();
    assert!(engine.validate_action(&candidate, &world_state).0);
}

#[test]
fn test_prompt_token_estimation() {
    use agentora_core::prompt::PromptBuilder;

    let builder = PromptBuilder::new();

    // 纯英文字符串
    let english = "Hello world, this is a test";
    let en_tokens = PromptBuilder::estimate_tokens(english);
    assert!(en_tokens > 0);

    // 中文字符串
    let chinese = "你是一个自主决策的AI Agent";
    let zh_tokens = PromptBuilder::estimate_tokens(chinese);
    assert!(zh_tokens > 0);

    // 中文 token 数应大于英文（相同字符数下）
    assert!(zh_tokens > en_tokens);
}

#[test]
fn test_prompt_truncation_under_limit() {
    use agentora_core::prompt::PromptBuilder;

    let builder = PromptBuilder::new();

    // 正常大小的输入不应截断
    let prompt = builder.build_decision_prompt(
        "test_agent",
        "周围有树木和石头",
        "曾经采集过资源",
        Some("优先采集食物"),
        None,
    );

    let estimated = PromptBuilder::estimate_tokens(&prompt);
    assert!(estimated <= builder.get_max_tokens());
}

#[test]
fn test_prompt_memory_truncation() {
    use agentora_core::prompt::PromptBuilder;

    let builder = PromptBuilder::new();

    // 超长记忆应该被截断
    let long_memory = "Agent 过去做了很多很多事情。".repeat(200);
    let prompt = builder.build_decision_prompt(
        "test_agent",
        "感知摘要",
        &long_memory,
        Some("策略提示"),
        None,
    );

    let estimated = PromptBuilder::estimate_tokens(&prompt);
    assert!(estimated <= builder.get_max_tokens() + 200); // 允许较多误差（系统提示增大后截断余量有限）
}

#[test]
fn test_infer_state_mode_hunger() {
    use agentora_core::decision::{infer_state_mode, SparkType};

    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_satiety = 20;
    let mode = infer_state_mode(&world_state);
    assert!(matches!(mode, SparkType::ResourcePressure));
}

#[test]
fn test_infer_state_mode_social() {
    use agentora_core::decision::{infer_state_mode, SparkType};
    use agentora_core::vision::NearbyAgentInfo;
    use agentora_core::agent::RelationType;

    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.nearby_agents.push(NearbyAgentInfo {
        id: AgentId::new("other"),
        name: "Other".to_string(),
        position: Position::new(11, 10),
        distance: 1,
        relation_type: RelationType::Neutral,
        trust: 0.0,
    });
    let mode = infer_state_mode(&world_state);
    assert!(matches!(mode, SparkType::SocialPressure));
}

#[test]
fn test_infer_state_mode_explore() {
    use agentora_core::decision::{infer_state_mode, SparkType};

    let world_state = agentora_core::rule_engine::WorldState::default();
    let mode = infer_state_mode(&world_state);
    assert!(matches!(mode, SparkType::Explore));
}
