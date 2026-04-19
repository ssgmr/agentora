//! 单元测试 - Agent 模块
//!
//! 临时偏好系统 | 交易逻辑 | 战斗逻辑

use agentora_core::agent::Agent;
use agentora_core::agent::trade::TradeOffer;
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

fn make_test_trade(offer: ResourceType, offer_amount: u32, want: ResourceType, want_amount: u32) -> TradeOffer {
    let mut offer_map = HashMap::new();
    offer_map.insert(offer, offer_amount);
    let mut want_map = HashMap::new();
    want_map.insert(want, want_amount);

    let proposer = Agent::new(AgentId::default(), "proposer".into(), Position::new(0, 0));
    proposer.propose_trade(AgentId::default(), offer_map, want_map)
}

#[test]
fn test_accept_trade_sufficient_resources() {
    let mut acceptor = Agent::new(AgentId::default(), "acceptor".into(), Position::new(0, 0));
    // 给予足够资源来接受交易（初始已有 food=3，再加 10）
    acceptor.gather(ResourceType::Food, 10);

    let trade = make_test_trade(ResourceType::Wood, 5, ResourceType::Food, 3);

    // 发起方有足够offer资源
    let mut proposer_inv = HashMap::new();
    proposer_inv.insert("wood".to_string(), 10);

    assert!(acceptor.accept_trade(&trade, &proposer_inv));
    // acceptor 初始 food=3, gather +10 = 13, 付出 3 后剩余 10
    assert_eq!(acceptor.inventory.get("food").copied().unwrap_or(0), 10);
    assert_eq!(acceptor.inventory.get("wood").copied().unwrap_or(0), 5);
}

#[test]
fn test_accept_trade_insufficient_own_resources() {
    let mut acceptor = Agent::new(AgentId::default(), "acceptor".into(), Position::new(0, 0));
    // 没有足够资源

    let trade = make_test_trade(ResourceType::Wood, 5, ResourceType::Food, 10);

    let proposer_inv = HashMap::new();
    assert!(!acceptor.accept_trade(&trade, &proposer_inv));
}

#[test]
fn test_accept_trade_fraud_detection() {
    let mut acceptor = Agent::new(AgentId::default(), "acceptor".into(), Position::new(0, 0));
    acceptor.gather(ResourceType::Food, 10);

    let trade = make_test_trade(ResourceType::Wood, 10, ResourceType::Food, 3);
    // 发起方实际只有5 wood，不足offer的10
    let mut proposer_inv = HashMap::new();
    proposer_inv.insert("wood".to_string(), 5);

    assert!(!acceptor.accept_trade(&trade, &proposer_inv));
}

// ===== 战斗逻辑 =====

#[test]
fn test_attack_deals_damage() {
    let mut attacker = Agent::new(AgentId::default(), "attacker".into(), Position::new(0, 0));
    let mut target = Agent::new(AgentId::default(), "target".into(), Position::new(0, 0));
    target.health = 100;

    let result = attacker.attack(&mut target, 25);

    assert_eq!(result.damage, 25);
    assert_eq!(target.health, 75);
    assert!(result.target_alive);
}

#[test]
fn test_attack_kills_target() {
    let mut attacker = Agent::new(AgentId::default(), "attacker".into(), Position::new(0, 0));
    let mut target = Agent::new(AgentId::default(), "target".into(), Position::new(0, 0));
    target.health = 10;

    let result = attacker.attack(&mut target, 15);

    assert_eq!(target.health, 0);
    assert!(!result.target_alive);
}

#[test]
fn test_attack_sets_enemy_relation() {
    let mut attacker = Agent::new(AgentId::default(), "attacker".into(), Position::new(0, 0));
    let mut target = Agent::new(AgentId::default(), "target".into(), Position::new(0, 0));

    attacker.attack(&mut target, 5);

    // 双方互相标记为敌人
    assert!(attacker.relations.get(&target.id).unwrap().trust == 0.0);
    assert!(target.relations.get(&attacker.id).unwrap().trust == 0.0);
}

#[test]
fn test_attack_no_negative_health() {
    let mut attacker = Agent::new(AgentId::default(), "attacker".into(), Position::new(0, 0));
    let mut target = Agent::new(AgentId::default(), "target".into(), Position::new(0, 0));
    target.health = 5;

    attacker.attack(&mut target, 100);

    assert_eq!(target.health, 0); // saturating_sub 不会溢出到负数
}
