//! 单元测试 - 规则说明书、个性配置、动作反馈、规则引擎详细反馈

use agentora_core::prompt::{RulesManual, PromptBuilder};
use agentora_core::types::{PersonalitySeed, PersonalityTemplate};
use agentora_core::rule_engine::{RuleEngine, WorldState};
use agentora_core::decision::ActionCandidate;
use agentora_core::types::{ActionType, Position, ResourceType, AgentId};
use std::collections::HashMap;

// ===== 5.1 单元测试 - RulesManual =====

#[test]
fn test_rules_manual_default_values() {
    let manual = RulesManual::default();
    assert_eq!(manual.survival.satiety_decay_per_tick, 1);
    assert_eq!(manual.survival.hydration_decay_per_tick, 1);
    assert_eq!(manual.recovery.eat_satiety_gain, 30);
    assert_eq!(manual.recovery.drink_hydration_gain, 25);
    assert_eq!(manual.gather.gather_amount, 2);
    assert_eq!(manual.inventory.default_stack_limit, 20);
    assert_eq!(manual.inventory.warehouse_stack_limit, 40);
    assert_eq!(manual.structure.camp_heal_per_tick, 2);
    assert_eq!(manual.structure.camp_range, 1);
    assert_eq!(manual.pressure.drought_water_reduction, 0.5);
    assert_eq!(manual.pressure.abundance_food_multiplier, 2.0);
    assert_eq!(manual.pressure.plague_hp_loss, 20);
}

#[test]
fn test_rules_section_core_always_present() {
    let manual = RulesManual::default();
    let section = manual.build_rules_section(100, 100, &[], &[]);

    assert!(section.contains("世界规则数值表"));
    assert!(section.contains("饱食度每tick下降"));
    assert!(section.contains("水分度每tick下降"));
    assert!(section.contains("Eat：消耗"));
    assert!(section.contains("Drink：消耗"));
    assert!(section.contains("Gather：每次采集"));
    assert!(section.contains("背包每种资源上限"));
    assert!(section.contains("Build消耗"));
    assert!(section.contains("Camp效果"));
}

#[test]
fn test_rules_section_survival_urgency() {
    let manual = RulesManual::default();

    // 饱食度低时应注入紧迫提示
    let section = manual.build_rules_section(30, 100, &[], &[]);
    assert!(section.contains("饱食度偏低"));

    // 水分度低时应注入紧迫提示
    let section = manual.build_rules_section(100, 40, &[], &[]);
    assert!(section.contains("水分度偏低"));

    // 都不低时不应注入
    let section = manual.build_rules_section(80, 80, &[], &[]);
    assert!(!section.contains("生存紧迫提示"));
}

#[test]
fn test_rules_section_building_effects() {
    let manual = RulesManual::default();

    // 附近有Camp时应注入Camp效果
    let section = manual.build_rules_section(100, 100, &["Camp at (5,5)"], &[]);
    assert!(section.contains("Camp"));
    assert!(section.contains("恢复"));

    // 附近有Warehouse时应注入Warehouse效果
    let section = manual.build_rules_section(100, 100, &["Warehouse at (3,3)"], &[]);
    assert!(section.contains("Warehouse"));
    assert!(section.contains("库存上限"));

    // 没有建筑时不应注入建筑效果
    let section = manual.build_rules_section(100, 100, &[], &[]);
    assert!(!section.contains("建筑效果"));
}

#[test]
fn test_rules_section_pressure_events() {
    let manual = RulesManual::default();

    // 干旱事件
    let section = manual.build_rules_section(100, 100, &[], &["干旱"]);
    assert!(section.contains("干旱"));
    assert!(section.contains("水资源产出"));

    // 丰饶事件
    let section = manual.build_rules_section(100, 100, &[], &["丰饶"]);
    assert!(section.contains("丰饶"));
    assert!(section.contains("食物产出"));

    // 瘟疫事件
    let section = manual.build_rules_section(100, 100, &[], &["瘟疫"]);
    assert!(section.contains("瘟疫"));
    assert!(section.contains("HP"));

    // 无压力事件
    let section = manual.build_rules_section(100, 100, &[], &[]);
    assert!(!section.contains("压力事件"));
}

// ===== 5.2 单元测试 - PersonalitySeed =====

#[test]
fn test_personality_from_template() {
    let template = PersonalityTemplate {
        openness: 0.7,
        agreeableness: 0.3,
        neuroticism: 0.5,
        description: "一个勇敢的探索者".to_string(),
    };
    let personality = PersonalitySeed::from_template(&template);
    assert_eq!(personality.openness, 0.7);
    assert_eq!(personality.agreeableness, 0.3);
    assert_eq!(personality.neuroticism, 0.5);
    assert_eq!(personality.description, "一个勇敢的探索者");
}

#[test]
fn test_personality_section_with_description() {
    let builder = PromptBuilder::new();
    let template = PersonalityTemplate {
        openness: 0.8,
        agreeableness: 0.3,
        neuroticism: 0.4,
        description: "一个好奇的探索者，喜欢发现新事物".to_string(),
    };
    let personality = PersonalitySeed::from_template(&template);

    let section = builder.build_personality_section("Agent_A", &personality);
    assert!(section.contains("Agent_A"));
    assert!(section.contains("一个好奇的探索者"));
}

#[test]
fn test_personality_section_empty_description_fallback() {
    let builder = PromptBuilder::new();
    let personality = PersonalitySeed {
        openness: 0.5,
        agreeableness: 0.5,
        neuroticism: 0.5,
        description: String::new(),
    };

    let section = builder.build_personality_section("Agent_B", &personality);
    assert!(section.contains("Agent_B"));
    assert!(section.contains("自主决策"));
}

#[test]
fn test_decision_prompt_includes_personality() {
    let builder = PromptBuilder::new();
    let template = PersonalityTemplate {
        openness: 0.3,
        agreeableness: 0.4,
        neuroticism: 0.7,
        description: "一个谨慎的生存者，注重自身安全".to_string(),
    };
    let personality = PersonalitySeed::from_template(&template);

    let prompt = builder.build_decision_prompt(
        "Survivor_A",
        "周围有树木和石头",
        "曾经采集过资源",
        Some("优先采集食物"),
        None,
        10,
        Some(&personality),
        80,
        80,
        &[],
        &[],
    );

    assert!(prompt.contains("Survivor_A"));
    assert!(prompt.contains("谨慎的生存者"));
    assert!(prompt.contains("安全"));
}

// ===== 5.3 单元测试 - ActionFeedback =====

#[test]
fn test_action_feedback_format_move_toward() {
    use agentora_core::world::ActionResult;

    let result = ActionResult::SuccessWithDetail("move:10,10→(11,10)".into());
    match result {
        ActionResult::SuccessWithDetail(msg) => {
            assert!(msg.contains("move:"));
            assert!(msg.contains("→"));
        }
        _ => panic!("Expected SuccessWithDetail"),
    }
}

#[test]
fn test_action_feedback_format_gather_success() {
    use agentora_core::world::ActionResult;

    let result = ActionResult::SuccessWithDetail("gather:woodx2,node_remain:48,inv:3→5".into());
    match result {
        ActionResult::SuccessWithDetail(msg) => {
            assert!(msg.contains("gather:"));
            assert!(msg.contains("wood"));
            assert!(msg.contains("node_remain"));
            assert!(msg.contains("inv:"));
        }
        _ => panic!("Expected SuccessWithDetail"),
    }
}

#[test]
fn test_action_feedback_format_eat_success() {
    use agentora_core::world::ActionResult;

    let result = ActionResult::SuccessWithDetail("eat:satiety+30(45→75),food_remain=2".into());
    match result {
        ActionResult::SuccessWithDetail(msg) => {
            assert!(msg.contains("satiety+"));
            assert!(msg.contains("food_remain"));
        }
        _ => panic!("Expected SuccessWithDetail"),
    }
}

#[test]
fn test_action_feedback_format_blocked() {
    use agentora_core::world::ActionResult;

    let result: ActionResult = ActionResult::Blocked("资源不足。需要 wood x5 + stone x2，背包中只有 wood x2".into());
    match result {
        ActionResult::Blocked(msg) => {
            assert!(msg.contains("资源不足"));
            assert!(msg.contains("需要"));
            assert!(msg.contains("背包中只有"));
        }
        _ => panic!("Expected Blocked"),
    }
}

#[test]
fn test_action_feedback_format_attack_distance() {
    use agentora_core::world::ActionResult;

    let result: ActionResult = ActionResult::Blocked("Attack失败：目标Agent 张三 距离过远（距离3格）。Attack只能对相邻格Agent执行".into());
    match result {
        ActionResult::Blocked(msg) => {
            assert!(msg.contains("距离过远"));
            assert!(msg.contains("距离3格"));
            assert!(msg.contains("相邻格"));
        }
        _ => panic!("Expected Blocked"),
    }
}

#[test]
fn test_action_feedback_format_attack_ally() {
    use agentora_core::world::ActionResult;

    let result: ActionResult = ActionResult::Blocked("Attack失败：不能攻击盟友Agent 李四。若要攻击，需先解除盟约".into());
    match result {
        ActionResult::Blocked(msg) => {
            assert!(msg.contains("不能攻击盟友"));
            assert!(msg.contains("李四"));
        }
        _ => panic!("Expected Blocked"),
    }
}

#[test]
fn test_action_feedback_format_build_insufficient() {
    use agentora_core::world::ActionResult;

    let result: ActionResult = ActionResult::Blocked("资源不足。需要 wood x5 + stone x2，背包中只有 wood x2 + stone x0".into());
    match result {
        ActionResult::Blocked(msg) => {
            assert!(msg.contains("wood x5"));
            assert!(msg.contains("stone x2"));
            assert!(msg.contains("wood x2"));
        }
        _ => panic!("Expected Blocked"),
    }
}

// ===== 5.4 单元测试 - 规则引擎详细反馈 =====

#[test]
fn test_validate_action_build_insufficient_returns_resource_diff() {
    let mut world_state = WorldState::default();
    world_state.agent_inventory.insert(ResourceType::Wood, 2);
    world_state.agent_inventory.insert(ResourceType::Stone, 0);

    let candidate = ActionCandidate {
        reasoning: "建造营地".to_string(),
        action_type: ActionType::Build { structure: agentora_core::types::StructureType::Camp },
        target: None,
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    let (valid, reason) = engine.validate_action(&candidate, &world_state);
    assert!(!valid);
    let reason = reason.unwrap();
    assert!(reason.contains("需要"));
    assert!(reason.contains("wood"));
    assert!(reason.contains("stone"));
    assert!(reason.contains("背包只有"));
}

#[test]
fn test_validate_action_eat_no_food_returns_inventory_state() {
    let mut world_state = WorldState::default();
    world_state.agent_inventory.insert(ResourceType::Wood, 3);
    world_state.agent_inventory.insert(ResourceType::Stone, 2);

    let candidate = ActionCandidate {
        reasoning: "进食".to_string(),
        action_type: ActionType::Eat,
        target: None,
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    let (valid, reason) = engine.validate_action(&candidate, &world_state);
    assert!(!valid);
    let reason = reason.unwrap();
    assert!(reason.contains("没有food"));
    assert!(reason.contains("当前背包"));
    assert!(reason.contains("wood"));
    assert!(reason.contains("stone"));
}

#[test]
fn test_validate_action_drink_no_water_returns_inventory_state() {
    let world_state = WorldState::default();

    let candidate = ActionCandidate {
        reasoning: "饮水".to_string(),
        action_type: ActionType::Drink,
        target: None,
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    let (valid, reason) = engine.validate_action(&candidate, &world_state);
    assert!(!valid);
    let reason = reason.unwrap();
    assert!(reason.contains("没有water"));
    assert!(reason.contains("当前背包"));
}

#[test]
fn test_validate_action_gather_no_resource_returns_position() {
    let mut world_state = WorldState::default();
    world_state.agent_position = Position::new(120, 115);

    let candidate = ActionCandidate {
        reasoning: "采集食物".to_string(),
        action_type: ActionType::Gather { resource: ResourceType::Food },
        target: None,
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    let (valid, reason) = engine.validate_action(&candidate, &world_state);
    assert!(!valid);
    let reason = reason.unwrap();
    assert!(reason.contains("120"));
    assert!(reason.contains("115"));
    assert!(reason.contains("没有"));
    assert!(reason.contains("Food"));
}

#[test]
fn test_validate_action_gather_wrong_resource_returns_details() {
    let mut world_state = WorldState::default();
    world_state.agent_position = Position::new(5, 5);
    world_state.resources_at.insert(Position::new(5, 5), (ResourceType::Wood, 50));

    let candidate = ActionCandidate {
        reasoning: "采集食物".to_string(),
        action_type: ActionType::Gather { resource: ResourceType::Food },
        target: None,
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    let (valid, reason) = engine.validate_action(&candidate, &world_state);
    assert!(!valid);
    let reason = reason.unwrap();
    assert!(reason.contains("Wood"));
    assert!(reason.contains("Food"));
    assert!(reason.contains("不是"));
}

#[test]
fn test_validate_action_attack_distance_returns_distance() {
    let mut world_state = WorldState::default();
    world_state.agent_position = Position::new(10, 10);
    let target_id = AgentId::new("enemy");
    world_state.existing_agents.insert(target_id.clone());
    world_state.nearby_agents.push(agentora_core::vision::NearbyAgentInfo {
        id: target_id.clone(),
        name: "敌人".to_string(),
        position: Position::new(13, 10), // 距离3格
        distance: 3,
        relation_type: agentora_core::agent::RelationType::Enemy,
        trust: 0.0,
    });

    let candidate = ActionCandidate {
        reasoning: "攻击".to_string(),
        action_type: ActionType::Attack { target_id },
        target: Some("敌人".to_string()),
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    let (valid, reason) = engine.validate_action(&candidate, &world_state);
    assert!(!valid);
    let reason = reason.unwrap();
    assert!(reason.contains("距离过远"));
    assert!(reason.contains("3格"));
}

#[test]
fn test_validate_action_trade_insufficient_returns_detail() {
    let mut world_state = WorldState::default();
    world_state.existing_agents.insert(AgentId::new("trader"));
    world_state.agent_inventory.insert(ResourceType::Wood, 3);

    let mut offer = HashMap::new();
    offer.insert(ResourceType::Wood, 10);

    let candidate = ActionCandidate {
        reasoning: "交易".to_string(),
        action_type: ActionType::TradeOffer { offer, want: HashMap::new(), target_id: AgentId::new("trader") },
        target: Some("trader".to_string()),
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    let (valid, reason) = engine.validate_action(&candidate, &world_state);
    assert!(!valid);
    let reason = reason.unwrap();
    assert!(reason.contains("资源不足"));
    assert!(reason.contains("wood"));
    assert!(reason.contains("x10"));
    assert!(reason.contains("x3"));
}

#[test]
fn test_validate_action_valid_returns_none_reason() {
    let candidate = ActionCandidate {
        reasoning: "等待".to_string(),
        action_type: ActionType::Wait,
        target: None,
        params: HashMap::new(),
    };

    let engine = RuleEngine::new();
    let world_state = WorldState::default();
    let (valid, reason) = engine.validate_action(&candidate, &world_state);
    assert!(valid);
    assert!(reason.is_none());
}
