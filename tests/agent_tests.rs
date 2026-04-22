//! 单元测试 - Agent 模块
//!
//! 临时偏好系统 | 交易逻辑 | 战斗逻辑

use agentora_core::agent::Agent;
use agentora_core::types::{AgentId, Position, ResourceType};
use std::collections::HashMap;

// ===== 临时偏好系统 =====

#[test]
fn test_inject_preference_new() {
    let mut agent = Agent::new(AgentId::default(), "test".into(), Position::new(0, 0));
    assert!(agent.temp_preferences.is_empty());

    agent.inject_preference("food_bias", 0.3, 10);

    assert_eq!(agent.temp_preferences.len(), 1);
    let pref = &agent.temp_preferences[0];
    assert_eq!(pref.key, "food_bias");
    assert!((pref.boost - 0.3).abs() < f32::EPSILON);
    assert_eq!(pref.remaining_ticks, 10);
}

#[test]
fn test_inject_preference_stack_same_key() {
    let mut agent = Agent::new(AgentId::default(), "test".into(), Position::new(0, 0));
    agent.inject_preference("food_bias", 0.3, 10);
    agent.inject_preference("food_bias", 0.2, 5);

    assert_eq!(agent.temp_preferences.len(), 1);
    let pref = &agent.temp_preferences[0];
    assert!((pref.boost - 0.5).abs() < f32::EPSILON);
    assert_eq!(pref.remaining_ticks, 10); // 取较大值
}

#[test]
fn test_tick_preferences_expire() {
    let mut agent = Agent::new(AgentId::default(), "test".into(), Position::new(0, 0));
    agent.inject_preference("food_bias", 0.3, 2);

    agent.tick_preferences();
    assert_eq!(agent.temp_preferences.len(), 1);
    assert_eq!(agent.temp_preferences[0].remaining_ticks, 1);

    agent.tick_preferences();
    assert!(agent.temp_preferences.is_empty());
}

// ===== 交易逻辑 =====

#[test]
fn test_freeze_resources_success() {
    let mut agent = Agent::new(AgentId::default(), "test".into(), Position::new(0, 0));
    // 初始已有 food=3
    let mut offer = HashMap::new();
    offer.insert(ResourceType::Food, 2);

    let success = agent.freeze_resources(offer, "trade-123");
    assert!(success);
    assert_eq!(agent.inventory.get("food").copied().unwrap_or(0), 1);
    assert_eq!(agent.frozen_inventory.get("food").copied().unwrap_or(0), 2);
    assert_eq!(agent.pending_trade_id, Some("trade-123".to_string()));
}

#[test]
fn test_freeze_resources_insufficient() {
    let mut agent = Agent::new(AgentId::default(), "test".into(), Position::new(0, 0));
    // 初始已有 food=3
    let mut offer = HashMap::new();
    offer.insert(ResourceType::Food, 5);

    let success = agent.freeze_resources(offer, "trade-123");
    assert!(!success);
    assert_eq!(agent.inventory.get("food").copied().unwrap_or(0), 3);
    assert!(agent.frozen_inventory.is_empty());
}

#[test]
fn test_cancel_trade_returns_resources() {
    let mut agent = Agent::new(AgentId::default(), "test".into(), Position::new(0, 0));
    let mut offer = HashMap::new();
    offer.insert(ResourceType::Food, 2);

    agent.freeze_resources(offer.clone(), "trade-123");
    assert_eq!(agent.inventory.get("food").copied().unwrap_or(0), 1);

    agent.cancel_trade(offer);
    assert_eq!(agent.inventory.get("food").copied().unwrap_or(0), 3);
    assert!(agent.frozen_inventory.is_empty());
    assert!(agent.pending_trade_id.is_none());
}

#[test]
fn test_complete_trade_send() {
    let mut proposer = Agent::new(AgentId::default(), "proposer".into(), Position::new(0, 0));
    let mut offer = HashMap::new();
    offer.insert(ResourceType::Food, 2);
    let mut want = HashMap::new();
    want.insert(ResourceType::Wood, 5);

    proposer.freeze_resources(offer.clone(), "trade-123");
    proposer.complete_trade_send(offer, want);

    // offer 从 frozen 实际扣减
    assert!(proposer.frozen_inventory.is_empty());
    // want 加到 inventory
    assert_eq!(proposer.inventory.get("wood").copied().unwrap_or(0), 5);
    assert!(proposer.pending_trade_id.is_none());
}

#[test]
fn test_give_resources_success() {
    let mut agent = Agent::new(AgentId::default(), "test".into(), Position::new(0, 0));
    agent.gather(ResourceType::Food, 10);

    let mut want = HashMap::new();
    want.insert(ResourceType::Food, 5);

    let success = agent.give_resources(want);
    assert!(success);
    assert_eq!(agent.inventory.get("food").copied().unwrap_or(0), 8); // 初始3 + 10 - 5
}

#[test]
fn test_receive_resources() {
    let mut agent = Agent::new(AgentId::default(), "test".into(), Position::new(0, 0));

    let mut offer = HashMap::new();
    offer.insert(ResourceType::Wood, 5);

    agent.receive_resources(offer);
    assert_eq!(agent.inventory.get("wood").copied().unwrap_or(0), 5);
}

// ===== 战斗逻辑 =====

#[test]
fn test_receive_attack_deals_damage() {
    let mut target = Agent::new(AgentId::default(), "target".into(), Position::new(0, 0));
    target.health = 100;
    let attacker_id = AgentId::new("attacker");

    target.receive_attack(25, &attacker_id);

    assert_eq!(target.health, 75);
}

#[test]
fn test_receive_attack_kills_target() {
    let mut target = Agent::new(AgentId::default(), "target".into(), Position::new(0, 0));
    target.health = 10;
    let attacker_id = AgentId::new("attacker");

    target.receive_attack(15, &attacker_id);

    assert_eq!(target.health, 0);
}

#[test]
fn test_receive_attack_sets_enemy_relation() {
    let mut target = Agent::new(AgentId::default(), "target".into(), Position::new(0, 0));
    let attacker_id = AgentId::new("attacker");

    target.receive_attack(5, &attacker_id);

    let rel = target.relations.get(&attacker_id).unwrap();
    assert_eq!(rel.trust, 0.0);
    assert!(rel.relation_type == agentora_core::agent::RelationType::Enemy);
}

#[test]
fn test_initiate_attack_sets_enemy_relation() {
    let mut attacker = Agent::new(AgentId::default(), "attacker".into(), Position::new(0, 0));
    let target_id = AgentId::new("target");

    attacker.initiate_attack(&target_id);

    let rel = attacker.relations.get(&target_id).unwrap();
    assert_eq!(rel.trust, 0.0);
    assert!(rel.relation_type == agentora_core::agent::RelationType::Enemy);
}

#[test]
fn test_attack_no_negative_health() {
    let mut target = Agent::new(AgentId::default(), "target".into(), Position::new(0, 0));
    target.health = 5;
    let attacker_id = AgentId::new("attacker");

    target.receive_attack(100, &attacker_id);

    assert_eq!(target.health, 0); // saturating_sub 不会溢出到负数
}