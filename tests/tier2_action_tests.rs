//! 单元测试 - Tier 2 世界交互动作处理器
//!
//! 测试 handle_gather, handle_build, handle_attack, handle_trade_*, handle_ally_* 以及错误叙事生成

use agentora_core::types::{AgentId, Position, ResourceType, StructureType, Action, ActionType, LegacyInteraction};
use agentora_core::world::{World, ActionResult, PendingTrade, TradeStatus};
use agentora_core::agent::Agent;
use agentora_core::rule_engine::{RuleEngine, WorldState};
use agentora_core::NearbyAgentInfo;
use std::collections::HashMap;

// ===== 辅助函数 =====

fn create_test_agent(id: &str, name: &str, pos: Position) -> (AgentId, Agent) {
    let agent_id = AgentId::new(id);
    let mut agent = Agent::new(agent_id.clone(), name.to_string(), pos);
    // 给一些默认资源方便测试
    agent.inventory.insert("wood".to_string(), 20);
    agent.inventory.insert("stone".to_string(), 10);
    agent.inventory.insert("food".to_string(), 10);
    (agent_id, agent)
}

fn create_test_world() -> World {
    use agentora_core::seed::WorldSeed;
    let mut seed = WorldSeed::default();
    seed.map_size = [32, 32];
    seed.initial_agents = 0; // 手动添加
    World::new(&seed)
}

// ===== handle_gather 测试 =====

#[test]
fn test_gather_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "采集者", pos);
    agent.inventory.clear(); // 清空默认资源
    world.insert_agent_at(agent_id.clone(), agent);

    // 放置资源节点
    let resource_node = agentora_core::world::resource::ResourceNode::new(pos, ResourceType::Food, 100);
    world.resources.insert(pos, resource_node);

    let action = Action {
        reasoning: "采集食物".into(),
        action_type: ActionType::Gather { resource: ResourceType::Food },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    // 验证背包增加了资源
    let agent = world.agents.get(&agent_id).unwrap();
    assert!(*agent.inventory.get("food").unwrap_or(&0) > 0);
}

#[test]
fn test_gather_no_resource_node() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "采集者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "采集食物".into(),
        action_type: ActionType::Gather { resource: ResourceType::Food },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("没有资源节点") || r.contains("没有food")));
}

#[test]
fn test_gather_wrong_resource_type() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "采集者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    // 放置木材资源
    let resource_node = agentora_core::world::resource::ResourceNode::new(pos, ResourceType::Wood, 100);
    world.resources.insert(pos, resource_node);

    // 尝试采集食物
    let action = Action {
        reasoning: "采集食物".into(),
        action_type: ActionType::Gather { resource: ResourceType::Food },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("资源节点") || r.contains("不是")));
}

#[test]
fn test_gather_depleted_node() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "采集者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    // 放置已枯竭的资源节点
    let mut resource_node = agentora_core::world::resource::ResourceNode::new(pos, ResourceType::Food, 0);
    resource_node.is_depleted = true;
    world.resources.insert(pos, resource_node);

    let action = Action {
        reasoning: "采集食物".into(),
        action_type: ActionType::Gather { resource: ResourceType::Food },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("枯竭")));
}

// ===== handle_build 测试 =====

#[test]
fn test_build_fence_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "建造者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "建造围栏".into(),
        action_type: ActionType::Build { structure: StructureType::Fence },
        target: None,
        params: HashMap::new(),
        build_type: Some(StructureType::Fence),
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    // 验证建筑已创建
    assert!(world.structures.contains_key(&pos));

    // 验证资源已扣除（Fence 需要 2 wood）
    let agent = world.agents.get(&agent_id).unwrap();
    assert_eq!(*agent.inventory.get("wood").unwrap(), 18);
}

#[test]
fn test_build_camp_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "建造者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "建造营地".into(),
        action_type: ActionType::Build { structure: StructureType::Camp },
        target: None,
        params: HashMap::new(),
        build_type: Some(StructureType::Camp),
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));
    assert!(world.structures.contains_key(&pos));

    // Camp 需要 5 wood + 2 stone
    let agent = world.agents.get(&agent_id).unwrap();
    assert_eq!(*agent.inventory.get("wood").unwrap(), 15);
    assert_eq!(*agent.inventory.get("stone").unwrap(), 8);
}

#[test]
fn test_build_insufficient_resources() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "穷建造者", pos);
    agent.inventory.clear();
    agent.inventory.insert("wood".to_string(), 1); // 只有 1 个木材
    world.insert_agent_at(agent_id.clone(), agent);

    // Warehouse 需要 10 wood + 5 stone
    let action = Action {
        reasoning: "建造仓库".into(),
        action_type: ActionType::Build { structure: StructureType::Warehouse },
        target: None,
        params: HashMap::new(),
        build_type: Some(StructureType::Warehouse),
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("资源不足")));

    // 验证建筑未创建
    assert!(!world.structures.contains_key(&pos));
}

#[test]
fn test_build_existing_structure() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "建造者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    // 先建一个
    let action1 = Action {
        reasoning: "建造围栏".into(),
        action_type: ActionType::Build { structure: StructureType::Fence },
        target: None,
        params: HashMap::new(),
        build_type: Some(StructureType::Fence),
        direction: None,
    };
    world.apply_action(&agent_id, &action1);

    // 再建一个（同一位置）
    let action2 = Action {
        reasoning: "再建围栏".into(),
        action_type: ActionType::Build { structure: StructureType::Fence },
        target: None,
        params: HashMap::new(),
        build_type: Some(StructureType::Fence),
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action2);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("已有建筑")));
}

// ===== handle_attack 测试 =====

#[test]
fn test_attack_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (attacker_id, attacker) = create_test_agent("attacker", "攻击者", pos);
    let (target_id, mut target) = create_test_agent("target", "目标", pos);
    target.health = 50;
    world.insert_agent_at(attacker_id.clone(), attacker);
    world.insert_agent_at(target_id.clone(), target);

    let action = Action {
        reasoning: "攻击".into(),
        action_type: ActionType::Attack { target_id: target_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&attacker_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    // 验证目标受到伤害
    let target = world.agents.get(&target_id).unwrap();
    assert_eq!(target.health, 40);

    // 验证关系更新为 Enemy
    assert!(target.relations.contains_key(&attacker_id));
    let attacker = world.agents.get(&attacker_id).unwrap();
    assert!(attacker.relations.contains_key(&target_id));
}

#[test]
fn test_attack_kills_target() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (attacker_id, attacker) = create_test_agent("attacker", "攻击者", pos);
    let (target_id, mut target) = create_test_agent("target", "目标", pos);
    target.health = 5; // 低血量，一击必杀
    world.insert_agent_at(attacker_id.clone(), attacker);
    world.insert_agent_at(target_id.clone(), target);

    let action = Action {
        reasoning: "击杀".into(),
        action_type: ActionType::Attack { target_id: target_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&attacker_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    let target = world.agents.get(&target_id).unwrap();
    assert_eq!(target.health, 0);
    // is_alive 由 advance_tick 中的 check_agent_death 设置，不在 attack handler 中直接设置
}

#[test]
fn test_attack_dead_target() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (attacker_id, attacker) = create_test_agent("attacker", "攻击者", pos);
    let (target_id, mut target) = create_test_agent("target", "目标", pos);
    target.is_alive = false;
    world.insert_agent_at(attacker_id.clone(), attacker);
    world.insert_agent_at(target_id.clone(), target);

    let action = Action {
        reasoning: "攻击死人".into(),
        action_type: ActionType::Attack { target_id: target_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&attacker_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("已死亡")));
}

// ===== handle_trade_offer 测试 =====

#[test]
fn test_trade_offer_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (proposer_id, mut proposer) = create_test_agent("p1", "提议者", pos);
    let (acceptor_id, acceptor) = create_test_agent("a1", "接受者", pos);
    proposer.inventory.clear();
    proposer.inventory.insert("wood".to_string(), 10);
    proposer.inventory.insert("food".to_string(), 5);
    world.insert_agent_at(proposer_id.clone(), proposer);
    world.insert_agent_at(acceptor_id.clone(), acceptor);

    let mut offer = HashMap::new();
    offer.insert(ResourceType::Wood, 3);
    let mut want = HashMap::new();
    want.insert(ResourceType::Food, 2);

    let action = Action {
        reasoning: "交易提议".into(),
        action_type: ActionType::TradeOffer { offer, want, target_id: acceptor_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&proposer_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    // 验证待处理交易已创建
    assert_eq!(world.pending_trades.len(), 1);
    assert_eq!(world.pending_trades[0].proposer_id, proposer_id);
    assert_eq!(world.pending_trades[0].acceptor_id, acceptor_id);
}

#[test]
fn test_trade_offer_insufficient_resources() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (proposer_id, mut proposer) = create_test_agent("p1", "穷提议者", pos);
    let (acceptor_id, acceptor) = create_test_agent("a1", "接受者", pos);
    proposer.inventory.clear();
    proposer.inventory.insert("wood".to_string(), 1); // 只有 1 个
    world.insert_agent_at(proposer_id.clone(), proposer);
    world.insert_agent_at(acceptor_id.clone(), acceptor);

    let mut offer = HashMap::new();
    offer.insert(ResourceType::Wood, 5); // 需要 5 个
    let mut want = HashMap::new();
    want.insert(ResourceType::Food, 2);

    let action = Action {
        reasoning: "交易提议".into(),
        action_type: ActionType::TradeOffer { offer, want, target_id: acceptor_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&proposer_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("资源不足")));
    assert!(world.pending_trades.is_empty());
}

#[test]
fn test_trade_offer_dead_target() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (proposer_id, proposer) = create_test_agent("p1", "提议者", pos);
    let (acceptor_id, mut acceptor) = create_test_agent("a1", "死人", pos);
    acceptor.is_alive = false;
    world.insert_agent_at(proposer_id.clone(), proposer);
    world.insert_agent_at(acceptor_id.clone(), acceptor);

    let mut offer = HashMap::new();
    offer.insert(ResourceType::Wood, 1);
    let mut want = HashMap::new();
    want.insert(ResourceType::Food, 1);

    let action = Action {
        reasoning: "交易提议".into(),
        action_type: ActionType::TradeOffer { offer, want, target_id: acceptor_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&proposer_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("已死亡")));
}

// ===== handle_trade_accept 测试 =====

#[test]
fn test_trade_accept_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);

    // 发起方
    let (proposer_id, mut proposer) = create_test_agent("p1", "提议者", pos);
    proposer.inventory.clear();
    proposer.inventory.insert("wood".to_string(), 10);
    proposer.inventory.insert("food".to_string(), 5);
    world.insert_agent_at(proposer_id.clone(), proposer);

    // 接受方
    let (acceptor_id, mut acceptor) = create_test_agent("a1", "接受者", pos);
    acceptor.inventory.clear();
    acceptor.inventory.insert("wood".to_string(), 5);
    acceptor.inventory.insert("food".to_string(), 20);
    world.insert_agent_at(acceptor_id.clone(), acceptor);

    // 先冻结发起方的资源（模拟 TradeOffer 流程）
    let mut offer = std::collections::HashMap::new();
    offer.insert(agentora_core::types::ResourceType::Wood, 3);
    let proposer = world.agents.get_mut(&proposer_id).unwrap();
    proposer.freeze_resources(offer.clone(), "test-trade-1");

    // 手动创建待处理交易
    let mut offer_resources = HashMap::new();
    offer_resources.insert("wood".to_string(), 3);
    let mut want_resources = HashMap::new();
    want_resources.insert("food".to_string(), 2);

    world.pending_trades.push(PendingTrade {
        trade_id: "test-trade-1".to_string(),
        proposer_id: proposer_id.clone(),
        acceptor_id: acceptor_id.clone(),
        offer_resources,
        want_resources,
        status: TradeStatus::Pending,
        tick_created: 0,
    });

    let action = Action {
        reasoning: "接受交易".into(),
        action_type: ActionType::TradeAccept {
            trade_id: String::new(),
        },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&acceptor_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    // 验证交易已移除
    assert!(world.pending_trades.is_empty());

    // 验证资源交换
    let proposer = world.agents.get(&proposer_id).unwrap();
    let acceptor = world.agents.get(&acceptor_id).unwrap();

    // 发起方给出 3 wood，获得 2 food
    assert_eq!(*proposer.inventory.get("wood").unwrap_or(&0), 7);
    assert_eq!(*proposer.inventory.get("food").unwrap_or(&0), 7);

    // 接受方给出 2 food，获得 3 wood
    assert_eq!(*acceptor.inventory.get("wood").unwrap_or(&0), 8);
    assert_eq!(*acceptor.inventory.get("food").unwrap_or(&0), 18);
}

#[test]
fn test_trade_accept_no_pending() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "孤独者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "接受交易".into(),
        action_type: ActionType::TradeAccept {
            trade_id: String::new(),
        },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("待处理")));
}

// ===== handle_trade_reject 测试 =====

#[test]
fn test_trade_reject_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (proposer_id, proposer) = create_test_agent("p1", "提议者", pos);
    let (acceptor_id, acceptor) = create_test_agent("a1", "拒绝者", pos);
    world.insert_agent_at(proposer_id.clone(), proposer);
    world.insert_agent_at(acceptor_id.clone(), acceptor);

    world.pending_trades.push(PendingTrade {
        trade_id: "test-trade-2".to_string(),
        proposer_id: proposer_id.clone(),
        acceptor_id: acceptor_id.clone(),
        offer_resources: HashMap::new(),
        want_resources: HashMap::new(),
        status: TradeStatus::Pending,
        tick_created: 0,
    });

    let action = Action {
        reasoning: "拒绝交易".into(),
        action_type: ActionType::TradeReject {
            trade_id: String::new(),
        },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&acceptor_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));
    assert!(world.pending_trades.is_empty());
}

// ===== handle_ally_propose 测试 =====

#[test]
fn test_ally_propose_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (proposer_id, mut proposer) = create_test_agent("p1", "提议者", pos);
    let (target_id, target) = create_test_agent("t1", "目标", pos);
    // 建立高信任关系
    proposer.relations.insert(target_id.clone(), agentora_core::agent::Relation {
        trust: 0.8,
        relation_type: agentora_core::agent::RelationType::Ally,
        last_interaction_tick: 0,
    });
    world.insert_agent_at(proposer_id.clone(), proposer);
    world.insert_agent_at(target_id.clone(), target);

    let action = Action {
        reasoning: "提议结盟".into(),
        action_type: ActionType::AllyPropose { target_id: target_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&proposer_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));
}

#[test]
fn test_ally_propose_low_trust() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (proposer_id, mut proposer) = create_test_agent("p1", "低信任者", pos);
    let (target_id, target) = create_test_agent("t1", "目标", pos);
    // 低信任关系
    proposer.relations.insert(target_id.clone(), agentora_core::agent::Relation {
        trust: 0.2,
        relation_type: agentora_core::agent::RelationType::Neutral,
        last_interaction_tick: 0,
    });
    world.insert_agent_at(proposer_id.clone(), proposer);
    world.insert_agent_at(target_id.clone(), target);

    let action = Action {
        reasoning: "提议结盟".into(),
        action_type: ActionType::AllyPropose { target_id: target_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&proposer_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("信任")));
}

#[test]
fn test_ally_propose_no_relation() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (proposer_id, proposer) = create_test_agent("p1", "陌生人", pos);
    let (target_id, target) = create_test_agent("t1", "目标", pos);
    // 无任何关系记录
    world.insert_agent_at(proposer_id.clone(), proposer);
    world.insert_agent_at(target_id.clone(), target);

    let action = Action {
        reasoning: "提议结盟".into(),
        action_type: ActionType::AllyPropose { target_id: target_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&proposer_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("信任")));
}

// ===== handle_ally_accept 测试 =====

#[test]
fn test_ally_accept_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (proposer_id, proposer) = create_test_agent("p1", "提议者", pos);
    let (acceptor_id, acceptor) = create_test_agent("a1", "接受者", pos);
    world.insert_agent_at(proposer_id.clone(), proposer);
    world.insert_agent_at(acceptor_id.clone(), acceptor);

    let action = Action {
        reasoning: "接受结盟".into(),
        action_type: ActionType::AllyAccept { ally_id: proposer_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&acceptor_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    // 验证双方都建立了联盟关系
    let acceptor = world.agents.get(&acceptor_id).unwrap();
    assert!(acceptor.relations.contains_key(&proposer_id));
    let proposer = world.agents.get(&proposer_id).unwrap();
    assert!(proposer.relations.contains_key(&acceptor_id));
}

// ===== handle_ally_reject 测试 =====

#[test]
fn test_ally_reject_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (proposer_id, proposer) = create_test_agent("p1", "提议者", pos);
    let (acceptor_id, acceptor) = create_test_agent("a1", "拒绝者", pos);
    world.insert_agent_at(proposer_id.clone(), proposer);
    world.insert_agent_at(acceptor_id.clone(), acceptor);

    let action = Action {
        reasoning: "拒绝结盟".into(),
        action_type: ActionType::AllyReject { ally_id: proposer_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&acceptor_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));
}

// ===== 错误叙事生成测试 =====

#[test]
fn test_error_narrative_gather_no_resource() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "采集者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "采集食物".into(),
        action_type: ActionType::Gather { resource: ResourceType::Food },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    world.apply_action(&agent_id, &action);

    // 验证生成了错误叙事事件
    let error_events: Vec<_> = world.tick_events.iter()
        .filter(|e| e.event_type == "error")
        .collect();
    assert_eq!(error_events.len(), 1);
    assert!(error_events[0].description.contains("没有food资源节点") || error_events[0].description.contains("采集失败"));
    assert!(error_events[0].color_code == "#FF6666");
}

#[test]
fn test_error_narrative_build_existing_structure() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "建造者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    // 先放一个建筑
    let structure = agentora_core::world::structure::Structure::new(pos, StructureType::Fence, None, 0);
    world.structures.insert(pos, structure);

    let action = Action {
        reasoning: "建造围栏".into(),
        action_type: ActionType::Build { structure: StructureType::Fence },
        target: None,
        params: HashMap::new(),
        build_type: Some(StructureType::Fence),
        direction: None,
    };

    world.apply_action(&agent_id, &action);

    let error_events: Vec<_> = world.tick_events.iter()
        .filter(|e| e.event_type == "error")
        .collect();
    assert_eq!(error_events.len(), 1);
    assert!(error_events[0].description.contains("已有建筑"));
}

#[test]
fn test_error_narrative_attack_dead_target() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (attacker_id, attacker) = create_test_agent("atk", "攻击者", pos);
    let (target_id, mut target) = create_test_agent("tgt", "死人", pos);
    target.is_alive = false;
    world.insert_agent_at(attacker_id.clone(), attacker);
    world.insert_agent_at(target_id.clone(), target);

    let action = Action {
        reasoning: "攻击".into(),
        action_type: ActionType::Attack { target_id: target_id.clone() },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    world.apply_action(&attacker_id, &action);

    let error_events: Vec<_> = world.tick_events.iter()
        .filter(|e| e.event_type == "error")
        .collect();
    assert_eq!(error_events.len(), 1);
    assert!(error_events[0].description.contains("已死亡"));
}

#[test]
fn test_no_error_narrative_on_success() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "等待者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "休息".into(),
        action_type: ActionType::Wait,
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    world.apply_action(&agent_id, &action);

    let error_events: Vec<_> = world.tick_events.iter()
        .filter(|e| e.event_type == "error")
        .collect();
    assert_eq!(error_events.len(), 0);
}

// ===== RuleEngine 全套动作决策测试 =====

#[test]
fn test_rule_engine_select_target_attack() {
    let mut world_state = WorldState::default();
    world_state.agent_position = Position::new(10, 10);

    // 添加多个附近 Agent，距离不同
    world_state.nearby_agents.push(NearbyAgentInfo {
        id: AgentId::new("close"),
        name: "近敌".into(),
        position: Position::new(11, 10),
        distance: 1,
        relation_type: agentora_core::agent::RelationType::Neutral,
        trust: 0.5,
    });
    world_state.nearby_agents.push(NearbyAgentInfo {
        id: AgentId::new("far"),
        name: "远敌".into(),
        position: Position::new(15, 10),
        distance: 5,
        relation_type: agentora_core::agent::RelationType::Neutral,
        trust: 0.8,
    });

    let engine = RuleEngine::new();
    let target = engine.select_target("attack", &world_state);

    // 攻击选择最近的
    assert!(target.is_some());
    assert_eq!(target.unwrap().as_str(), "close");
}

#[test]
fn test_rule_engine_select_target_ally() {
    let mut world_state = WorldState::default();
    world_state.agent_position = Position::new(10, 10);

    world_state.nearby_agents.push(NearbyAgentInfo {
        id: AgentId::new("trusted"),
        name: "信任".into(),
        position: Position::new(15, 10),
        distance: 5,
        relation_type: agentora_core::agent::RelationType::Ally,
        trust: 0.9,
    });
    world_state.nearby_agents.push(NearbyAgentInfo {
        id: AgentId::new("distrusted"),
        name: "怀疑".into(),
        position: Position::new(11, 10),
        distance: 1,
        relation_type: agentora_core::agent::RelationType::Neutral,
        trust: 0.2,
    });

    let engine = RuleEngine::new();
    let target = engine.select_target("ally", &world_state);

    // 结盟选择信任度最高的
    assert!(target.is_some());
    assert_eq!(target.unwrap().as_str(), "trusted");
}

#[test]
fn test_rule_engine_select_target_empty() {
    let world_state = WorldState::default();
    let engine = RuleEngine::new();
    let target = engine.select_target("attack", &world_state);
    assert!(target.is_none());
}

#[test]
fn test_rule_engine_filter_build_sufficient() {
    let mut world_state = WorldState::default();
    // 提供足够资源建造 Fence (需要 2 wood)
    world_state.agent_inventory.insert(ResourceType::Wood, 5);

    let engine = RuleEngine::new();
    let candidates = engine.filter_hard_constraints(&world_state);

    assert!(candidates.iter().any(|a| matches!(a, ActionType::Build { structure: StructureType::Fence })));
}

#[test]
fn test_rule_engine_filter_build_insufficient() {
    let mut world_state = WorldState::default();
    // 只有1个木材，不够建任何建筑
    world_state.agent_inventory.insert(ResourceType::Wood, 1);

    let engine = RuleEngine::new();
    let candidates = engine.filter_hard_constraints(&world_state);

    assert!(!candidates.iter().any(|a| matches!(a, ActionType::Build { .. })));
}

#[test]
fn test_rule_engine_filter_build_no_resources() {
    let world_state = WorldState::default();
    let engine = RuleEngine::new();
    let candidates = engine.filter_hard_constraints(&world_state);

    assert!(!candidates.iter().any(|a| matches!(a, ActionType::Build { .. })));
}

#[test]
fn test_rule_engine_fallback_social() {
    // 有附近 Agent 时，rule engine 应能生成 Talk 动作
    let mut world_state = WorldState::default();
    world_state.existing_agents.insert(AgentId::new("friend"));
    world_state.nearby_agents.push(NearbyAgentInfo {
        id: AgentId::new("friend"),
        name: "朋友".into(),
        position: Position::new(11, 10),
        distance: 1,
        relation_type: agentora_core::agent::RelationType::Ally,
        trust: 0.8,
    });

    let engine = RuleEngine::new();
    let candidates = engine.filter_hard_constraints(&world_state);

    assert!(candidates.iter().any(|a| matches!(a, ActionType::Talk { .. })));
}

#[test]
fn test_rule_engine_fallback_attack_exists() {
    // 有敌对 Agent 时，rule engine 应能生成 Attack 动作
    let mut world_state = WorldState::default();
    world_state.existing_agents.insert(AgentId::new("enemy"));
    world_state.nearby_agents.push(NearbyAgentInfo {
        id: AgentId::new("enemy"),
        name: "敌人".into(),
        position: Position::new(11, 10),
        distance: 1,
        relation_type: agentora_core::agent::RelationType::Enemy,
        trust: 0.1,
    });

    let engine = RuleEngine::new();
    let candidates = engine.filter_hard_constraints(&world_state);

    assert!(candidates.iter().any(|a| matches!(a, ActionType::Attack { .. })));
}

// ===== handle_wait / handle_eat / handle_drink 测试 =====

#[test]
fn test_wait_does_not_restore_satiety() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "等待者", pos);
    agent.satiety = 50;
    agent.hydration = 50;
    agent.inventory.insert("food".to_string(), 5);
    agent.inventory.insert("water".to_string(), 5);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "单纯等待".into(),
        action_type: ActionType::Wait,
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    let agent = world.agents.get(&agent_id).unwrap();
    // Wait 不再自动进食/饮水
    assert_eq!(agent.satiety, 50);
    assert_eq!(agent.hydration, 50);
    assert_eq!(agent.inventory.get("food").copied().unwrap_or(0), 5);
    assert_eq!(agent.inventory.get("water").copied().unwrap_or(0), 5);
}

#[test]
fn test_eat_restores_satiety() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "饥饿者", pos);
    agent.satiety = 50;
    agent.hydration = 50;
    agent.inventory.insert("food".to_string(), 5);
    agent.inventory.insert("water".to_string(), 5);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "进食".into(),
        action_type: ActionType::Eat,
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    let agent = world.agents.get(&agent_id).unwrap();
    assert_eq!(agent.satiety, 80); // 50 + 30
    assert_eq!(agent.hydration, 50); // 饮水不变
    assert_eq!(agent.inventory.get("food").copied().unwrap_or(0), 4);
    assert_eq!(agent.inventory.get("water").copied().unwrap_or(0), 5); // 水不变
}

#[test]
fn test_drink_restores_hydration() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "口渴者", pos);
    agent.satiety = 50;
    agent.hydration = 40;
    agent.inventory.insert("food".to_string(), 5);
    agent.inventory.insert("water".to_string(), 3);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "饮水".into(),
        action_type: ActionType::Drink,
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    let agent = world.agents.get(&agent_id).unwrap();
    assert_eq!(agent.satiety, 50); // 饱食度不变
    assert_eq!(agent.hydration, 65); // 40 + 25
    assert_eq!(agent.inventory.get("food").copied().unwrap_or(0), 5);
    assert_eq!(agent.inventory.get("water").copied().unwrap_or(0), 2);
}

#[test]
fn test_eat_without_food_fails() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "无粮者", pos);
    agent.satiety = 30;
    agent.inventory = HashMap::new(); // 没有食物
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "尝试进食".into(),
        action_type: ActionType::Eat,
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert_eq!(result, ActionResult::Blocked("背包中没有food。当前背包：空".into()));
}

#[test]
fn test_drink_without_water_fails() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "无水者", pos);
    agent.hydration = 20;
    agent.inventory = HashMap::new(); // 没有水
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "尝试饮水".into(),
        action_type: ActionType::Drink,
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert_eq!(result, ActionResult::Blocked("背包中没有water。当前背包：空".into()));
}

#[test]
fn test_wait_full_health() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "健康者", pos);
    agent.health = 100;
    agent.max_health = 100;
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "休息".into(),
        action_type: ActionType::Wait,
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    world.apply_action(&agent_id, &action);

    let agent = world.agents.get(&agent_id).unwrap();
    assert_eq!(agent.health, 100); // 不超过最大值
}

// ===== handle_explore 测试 =====

#[test]
fn test_explore_moves() {
    let mut world = create_test_world();
    let pos = Position::new(10, 10);
    let (agent_id, agent) = create_test_agent("a1", "探索者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "探索".into(),
        action_type: ActionType::Explore { target_region: 0 },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));
}

// ===== handle_legacy_interaction 测试 =====

#[test]
fn test_legacy_pickup_success() {
    use agentora_core::Legacy;
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "拾取者", pos);
    agent.inventory.clear();
    world.insert_agent_at(agent_id.clone(), agent);

    // 创建有物品的遗产
    let mut legacy_items = HashMap::new();
    legacy_items.insert("wood".to_string(), 5);
    let legacy = Legacy {
        id: "leg1".to_string(),
        position: pos,
        legacy_type: agentora_core::LegacyType::Grave,
        original_agent_id: AgentId::new("dead"),
        original_agent_name: "已故者".to_string(),
        items: legacy_items,
        echo_log: None,
        created_tick: 0,
        decay_tick: 50,
    };
    world.legacies.push(legacy);

    let action = Action {
        reasoning: "拾取遗产".into(),
        action_type: ActionType::InteractLegacy {
            legacy_id: "leg1".to_string(),
            interaction: LegacyInteraction::Pickup,
        },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));

    let agent = world.agents.get(&agent_id).unwrap();
    assert!(*agent.inventory.get("wood").unwrap_or(&0) > 0);
}

#[test]
fn test_legacy_pickup_empty() {
    use agentora_core::Legacy;
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "拾取者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    // 创建空遗产
    let legacy = Legacy {
        id: "leg2".to_string(),
        position: pos,
        legacy_type: agentora_core::LegacyType::Grave,
        original_agent_id: AgentId::new("dead2"),
        original_agent_name: "已故者".to_string(),
        items: HashMap::new(),
        echo_log: None,
        created_tick: 0,
        decay_tick: 50,
    };
    world.legacies.push(legacy);

    let action = Action {
        reasoning: "拾取空遗产".into(),
        action_type: ActionType::InteractLegacy {
            legacy_id: "leg2".to_string(),
            interaction: LegacyInteraction::Pickup,
        },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::Blocked(ref r) if r.contains("无物品")));
}

#[test]
fn test_legacy_worship() {
    use agentora_core::Legacy;
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, agent) = create_test_agent("a1", "崇拜者", pos);
    world.insert_agent_at(agent_id.clone(), agent);

    let legacy = Legacy {
        id: "leg3".to_string(),
        position: pos,
        legacy_type: agentora_core::LegacyType::Grave,
        original_agent_id: AgentId::new("dead3"),
        original_agent_name: "先贤".to_string(),
        items: HashMap::new(),
        echo_log: None,
        created_tick: 0,
        decay_tick: 50,
    };
    world.legacies.push(legacy);

    let action = Action {
        reasoning: "崇拜遗产".into(),
        action_type: ActionType::InteractLegacy {
            legacy_id: "leg3".to_string(),
            interaction: LegacyInteraction::Worship,
        },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));
}

// ===== apply_action 前置校验测试 =====

#[test]
fn test_apply_action_invalid_agent() {
    let mut world = create_test_world();
    let fake_id = AgentId::new("nonexistent");
    let action = Action {
        reasoning: "移动".into(),
        action_type: ActionType::MoveToward { target: Position::new(6, 5) },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&fake_id, &action);
    assert_eq!(result, ActionResult::InvalidAgent);
}

#[test]
fn test_apply_action_dead_agent() {
    let mut world = create_test_world();
    let pos = Position::new(5, 5);
    let (agent_id, mut agent) = create_test_agent("a1", "死人", pos);
    agent.is_alive = false;
    world.insert_agent_at(agent_id.clone(), agent);

    let action = Action {
        reasoning: "移动".into(),
        action_type: ActionType::MoveToward { target: Position::new(6, 5) },
        target: None,
        params: HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);
    assert_eq!(result, ActionResult::AgentDead);
}
