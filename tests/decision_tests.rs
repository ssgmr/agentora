//! 单元测试 - 决策管道
//!
//! 测试硬约束过滤、规则校验、加权选择

use agentora_core::decision::{ActionCandidate, CandidateSource, DecisionPipeline, Spark, SparkType};
use agentora_core::rule_engine::RuleEngine;
use agentora_core::types::{ActionType, Direction, Position, TerrainType};
use agentora_core::motivation::MotivationVector;
use std::collections::HashMap;

#[test]
fn test_filter_move_valid() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.terrain_at.insert(Position::new(11, 10), TerrainType::Plains);

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    assert!(filtered.iter().any(|a| matches!(a, ActionType::Move { direction: Direction::East })));
}

#[test]
fn test_filter_move_blocked() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.terrain_at.insert(Position::new(11, 10), TerrainType::Mountain);

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    // 向东移动被阻挡
    assert!(!filtered.iter().any(|a| matches!(a, ActionType::Move { direction: Direction::East })));
}

#[test]
fn test_filter_move_boundary() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(0, 10);  // 左边界

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    // 向西移动会越界
    assert!(!filtered.iter().any(|a| matches!(a, ActionType::Move { direction: Direction::West })));
}

#[test]
fn test_fallback_decision() {
    let motivation = MotivationVector::from_array([0.8, 0.5, 0.5, 0.5, 0.5, 0.5]);
    let world_state = agentora_core::rule_engine::WorldState::default();

    let engine = RuleEngine::new();
    let action = engine.fallback_action(&motivation, &world_state);

    // 应返回安全动作（等待）
    assert_eq!(action.action_type, ActionType::Wait);
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
    use agentora_core::types::ResourceType;
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
    use agentora_core::types::AgentId;
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.existing_agents.insert(AgentId::new("other_agent"));

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    assert!(filtered.iter().any(|a| matches!(a, ActionType::Talk { .. })));
    assert!(filtered.iter().any(|a| matches!(a, ActionType::Attack { .. })));
}

#[test]
fn test_filter_terrain_unpassable() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.terrain_at.insert(Position::new(10, 11), TerrainType::Water);

    let engine = RuleEngine::new();
    let filtered = engine.filter_hard_constraints(&world_state);

    // 向南移动被水阻挡
    assert!(!filtered.iter().any(|a| matches!(a, ActionType::Move { direction: Direction::South })));
}

#[test]
fn test_validate_action_valid_move() {
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    world_state.terrain_at.insert(Position::new(11, 10), TerrainType::Plains);

    let candidate = ActionCandidate {
        reasoning: "向东移动".to_string(),
        action_type: ActionType::Move { direction: Direction::East },
        target: None,
        params: HashMap::new(),
        motivation_delta: [0.1, 0.0, 0.0, 0.0, 0.0, 0.0],
        source: CandidateSource::Llm,
    };

    let engine = RuleEngine::new();
    assert!(engine.validate_action(&candidate, &world_state));
}

#[test]
fn test_validate_action_invalid_attack_target() {
    let world_state = agentora_core::rule_engine::WorldState::default();
    let candidate = ActionCandidate {
        reasoning: "攻击不存在的目标".to_string(),
        action_type: ActionType::Attack { target_id: agentora_core::types::AgentId::new("ghost") },
        target: Some("ghost".to_string()),
        params: HashMap::new(),
        motivation_delta: [0.0, 0.0, 0.0, 0.0, 0.5, 0.0],
        source: CandidateSource::Llm,
    };

    let engine = RuleEngine::new();
    assert!(!engine.validate_action(&candidate, &world_state));
}

#[test]
fn test_validate_action_trade_insufficient_resources() {
    use agentora_core::types::ResourceType;
    let world_state = agentora_core::rule_engine::WorldState::default();
    // 没有资源却要交易
    let mut offer = HashMap::new();
    offer.insert(ResourceType::Wood, 10);

    let candidate = ActionCandidate {
        reasoning: "交易".to_string(),
        action_type: ActionType::TradeOffer { offer, want: HashMap::new() },
        target: Some("trader".to_string()),
        params: HashMap::new(),
        motivation_delta: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        source: CandidateSource::Llm,
    };

    let engine = RuleEngine::new();
    assert!(!engine.validate_action(&candidate, &world_state));
}

#[test]
fn test_validate_action_trade_valid() {
    use agentora_core::types::ResourceType;
    let mut world_state = agentora_core::rule_engine::WorldState::default();
    let mut offer = HashMap::new();
    offer.insert(ResourceType::Wood, 5);

    // 背包有足够资源
    world_state.agent_inventory.insert(ResourceType::Wood, 10);

    let candidate = ActionCandidate {
        reasoning: "交易".to_string(),
        action_type: ActionType::TradeOffer { offer, want: HashMap::new() },
        target: Some("trader".to_string()),
        params: HashMap::new(),
        motivation_delta: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        source: CandidateSource::Llm,
    };

    let engine = RuleEngine::new();
    assert!(engine.validate_action(&candidate, &world_state));
}

#[test]
fn test_validate_action_wait_always_valid() {
    let candidate = ActionCandidate {
        reasoning: "等待".to_string(),
        action_type: ActionType::Wait,
        target: None,
        params: HashMap::new(),
        motivation_delta: [0.0; 6],
        source: CandidateSource::RuleEngine,
    };

    let engine = RuleEngine::new();
    let world_state = agentora_core::rule_engine::WorldState::default();
    assert!(engine.validate_action(&candidate, &world_state));
}

#[test]
fn test_motivation_weighted_select_unique_candidate() {
    // 唯一候选直接选择
    let candidate = ActionCandidate {
        reasoning: "唯一候选".to_string(),
        action_type: ActionType::Move { direction: Direction::East },
        target: None,
        params: HashMap::new(),
        motivation_delta: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0],
        source: CandidateSource::Llm,
    };

    let motivation = MotivationVector::from_array([0.8, 0.5, 0.5, 0.5, 0.5, 0.5]);
    let pipeline = DecisionPipeline::new();

    let selected = pipeline.select_unique_or_motivated(&[candidate], &motivation);
    assert_eq!(selected.action_type, ActionType::Move { direction: Direction::East });
}

#[test]
fn test_motivation_weighted_select_prefers_aligned() {
    // 多候选时，选择与动机最对齐的
    let candidates = vec![
        ActionCandidate {
            reasoning: "采集食物".to_string(),
            action_type: ActionType::Gather { resource: agentora_core::types::ResourceType::Food },
            target: None,
            params: HashMap::new(),
            motivation_delta: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // 强烈对齐生存
            source: CandidateSource::Llm,
        },
        ActionCandidate {
            reasoning: "社交".to_string(),
            action_type: ActionType::Talk { message: "hello".to_string() },
            target: Some("other".to_string()),
            params: HashMap::new(),
            motivation_delta: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0], // 对齐社交
            source: CandidateSource::Llm,
        },
    ];

    // Agent 生存动机最强
    let motivation = MotivationVector::from_array([0.9, 0.1, 0.1, 0.1, 0.1, 0.1]);
    let pipeline = DecisionPipeline::new();

    let selected = pipeline.select_unique_or_motivated(&candidates, &motivation);
    // 应该选择采集食物（生存动机得分最高）
    assert!(matches!(selected.action_type, ActionType::Gather { .. }));
}

#[test]
fn test_motivation_weighted_select_social_preference() {
    // 社交动机最强时选择社交
    let candidates = vec![
        ActionCandidate {
            reasoning: "采集食物".to_string(),
            action_type: ActionType::Gather { resource: agentora_core::types::ResourceType::Food },
            target: None,
            params: HashMap::new(),
            motivation_delta: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            source: CandidateSource::Llm,
        },
        ActionCandidate {
            reasoning: "社交".to_string(),
            action_type: ActionType::Talk { message: "hello".to_string() },
            target: Some("other".to_string()),
            params: HashMap::new(),
            motivation_delta: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            source: CandidateSource::Llm,
        },
    ];

    // Agent 社交动机最强
    let motivation = MotivationVector::from_array([0.1, 0.9, 0.1, 0.1, 0.1, 0.1]);
    let pipeline = DecisionPipeline::new();

    let selected = pipeline.select_unique_or_motivated(&candidates, &motivation);
    assert!(matches!(selected.action_type, ActionType::Talk { .. }));
}

#[test]
fn test_dot_product_calculation() {
    let pipeline = DecisionPipeline::new();
    let motivation = MotivationVector::from_array([1.0, 0.5, 0.0, 0.0, 0.0, 0.0]);
    let delta = [0.5, 1.0, 0.0, 0.0, 0.0, 0.0];

    let score = pipeline.compute_dot_product(&delta, &motivation);
    // 0.5*1.0 + 1.0*0.5 = 1.0
    assert!((score - 1.0).abs() < 0.001);
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
    use agentora_core::motivation::MotivationVector;
    use agentora_core::decision::Spark;

    let builder = PromptBuilder::new();
    let motivation = MotivationVector::from_array([0.5; 6]);
    let spark = Spark {
        spark_type: SparkType::Explore,
        description: "探索周围世界".to_string(),
        gap_value: 0.3,
    };

    // 正常大小的输入不应截断
    let prompt = builder.build_decision_prompt(
        "test_agent",
        &motivation,
        &spark,
        "周围有树木和石头",
        "曾经采集过资源",
        Some("优先采集食物"),
    );

    let estimated = PromptBuilder::estimate_tokens(&prompt);
    assert!(estimated <= builder.get_max_tokens());
}

#[test]
fn test_prompt_memory_truncation() {
    use agentora_core::prompt::PromptBuilder;
    use agentora_core::motivation::MotivationVector;
    use agentora_core::decision::Spark;

    let builder = PromptBuilder::new();
    let motivation = MotivationVector::from_array([0.5; 6]);
    let spark = Spark {
        spark_type: SparkType::Explore,
        description: "探索周围世界".to_string(),
        gap_value: 0.3,
    };

    // 超长记忆应该被截断
    let long_memory = "Agent 过去做了很多很多事情。".repeat(200);
    let prompt = builder.build_decision_prompt(
        "test_agent",
        &motivation,
        &spark,
        "感知摘要",
        &long_memory,
        Some("策略提示"),
    );

    let estimated = PromptBuilder::estimate_tokens(&prompt);
    assert!(estimated <= builder.get_max_tokens() + 50); // 允许少量误差
}