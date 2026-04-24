//! 验收测试 - Tier 3 涌现催化剂：策略闭环验证
//!
//! 验证 "创建策略 → 检索策略 → 参考策略" 的闭环是否生效

use agentora_core::strategy::StrategyHub;
use agentora_core::strategy::create::create_strategy;
use agentora_core::strategy::retrieve::{retrieve_strategy, get_strategy_summary};
use agentora_core::decision::{SparkType, infer_state_mode};
use agentora_core::rule_engine::WorldState;
use agentora_core::types::Position;

fn uid(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4())
}

/// 验收测试 1：策略创建使用动态 SparkType，与检索时匹配
#[test]
fn test_strategy_create_retrieve_loop() {
    let hub = StrategyHub::new(&uid("test-loop"));

    // 模拟 ResourcePressure 情境下创建策略
    create_strategy(
        &hub,
        SparkType::ResourcePressure,
        50,
        "资源短缺时优先采集附近食物",
    ).unwrap();

    // 同样处于 ResourcePressure 情境下，应该能检索到
    let retrieved = retrieve_strategy(&hub, SparkType::ResourcePressure);
    assert!(retrieved.is_some(), "相同 SparkType 下应该能检索到策略");

    let strategy = retrieved.unwrap();
    assert!(strategy.content.contains("资源短缺"));
}

/// 验收测试 2：不同 SparkType 之间不应该交叉匹配
#[test]
fn test_strategy_no_cross_match() {
    let hub = StrategyHub::new(&uid("test-cross"));

    // 只创建 ResourcePressure 策略
    create_strategy(
        &hub,
        SparkType::ResourcePressure,
        50,
        "资源短缺时优先采集附近食物",
    ).unwrap();

    // SocialPressure 情境下不应该检索到 ResourcePressure 策略
    let retrieved = retrieve_strategy(&hub, SparkType::SocialPressure);
    assert!(retrieved.is_none(), "不同 SparkType 不应该匹配");

    // Explore 情境下也不应该
    let retrieved = retrieve_strategy(&hub, SparkType::Explore);
    assert!(retrieved.is_none(), "不同 SparkType 不应该匹配");
}

/// 验收测试 3：infer_state_mode 推断的一致性
#[test]
fn test_infer_state_mode_consistency() {
    // 饥饿情境 → ResourcePressure
    let mut ws_hungry = WorldState::default();
    ws_hungry.agent_satiety = 20;
    ws_hungry.agent_hydration = 80;
    assert_eq!(infer_state_mode(&ws_hungry), SparkType::ResourcePressure);

    // 社交情境 → SocialPressure
    let mut ws_social = WorldState::default();
    ws_social.agent_satiety = 80;
    ws_social.agent_hydration = 80;
    ws_social.nearby_agents.push(agentora_core::NearbyAgentInfo {
        id: agentora_core::AgentId::new("other"),
        name: "其他Agent".to_string(),
        position: Position::new(10, 11),
        distance: 1,
        relation_type: agentora_core::agent::RelationType::Neutral,
        trust: 0.3,
    });
    assert_eq!(infer_state_mode(&ws_social), SparkType::SocialPressure);

    // 无压力 → Explore
    let ws_explore = WorldState::default();
    assert_eq!(infer_state_mode(&ws_explore), SparkType::Explore);
}

/// 验收测试 4：完整闭环 — 创建 → 推断 → 检索 → 注入
#[test]
fn test_full_strategy_loop() {
    let hub = StrategyHub::new(&uid("test-full-loop"));

    // 步骤 1：Agent 在饥饿情境下做出决策并创建了策略
    let mut ws_hungry = WorldState::default();
    ws_hungry.agent_satiety = 15;
    ws_hungry.agent_hydration = 60;
    let spark_type_at_creation = infer_state_mode(&ws_hungry);
    assert_eq!(spark_type_at_creation, SparkType::ResourcePressure);

    create_strategy(
        &hub,
        spark_type_at_creation,
        50,
        "饥饿时优先采集附近食物",
    ).unwrap();

    // 步骤 2：下次 Agent 再次处于饥饿情境时
    let mut ws_hungry_again = WorldState::default();
    ws_hungry_again.agent_satiety = 25;
    ws_hungry_again.agent_hydration = 70;
    let spark_type_now = infer_state_mode(&ws_hungry_again);

    // 步骤 3：应该能检索到之前创建的策略
    let retrieved = retrieve_strategy(&hub, spark_type_now);
    assert!(retrieved.is_some(), "相同情境下应该能检索到历史策略");

    // 步骤 4：策略内容可以注入 Prompt
    let summary = get_strategy_summary(&retrieved.unwrap());
    assert!(summary.contains("饥饿"));
    assert!(summary.contains("resource_pressure"));
}

/// 验收测试 5：策略摘要格式正确
#[test]
fn test_strategy_summary_format() {
    let hub = StrategyHub::new(&uid("test-summary"));

    create_strategy(
        &hub,
        SparkType::Explore,
        120,
        "无压力时向未知方向探索",
    ).unwrap();

    let retrieved = retrieve_strategy(&hub, SparkType::Explore).unwrap();
    let summary = get_strategy_summary(&retrieved);

    assert!(summary.contains("explore"));
    assert!(summary.contains("100%")); // 新策略成功率 1.0
    assert!(summary.contains("使用1次"));
    assert!(summary.contains("无压力"));
}
