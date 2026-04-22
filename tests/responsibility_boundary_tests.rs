//! 责任边界重构单元测试
//!
//! 验证 Phase 1-4 的重构正确性

use agentora_core::simulation::{WorldStateBuilder, DeltaDispatcher, SimMode, AgentDelta};
use agentora_core::decision::PerceptionBuilder;
use agentora_core::world::{ActionResultSchema, ActionResult};
use agentora_core::{World, WorldSeed, ActionType};
use std::sync::mpsc;

/// 测试 WorldStateBuilder 从 World 自动构建 WorldState
#[test]
fn test_world_state_builder() {
    let seed = WorldSeed::load("worldseeds/default.toml").unwrap();
    let world = World::new(&seed);

    // 获取第一个 Agent ID
    let agent_id = world.agents.keys().next().unwrap().clone();

    // 构建 WorldState
    let world_state = WorldStateBuilder::build(&world, &agent_id, 5);

    assert!(world_state.is_some());
    let ws = world_state.unwrap();

    // 验证基本字段
    assert_eq!(ws.self_id, agent_id);
    assert!(ws.agent_position.x < seed.map_size[0]);
    assert!(ws.agent_position.y < seed.map_size[1]);

    // 验证视野扫描
    assert!(!ws.terrain_at.is_empty());
}

/// 测试 PerceptionBuilder 生成感知摘要
#[test]
fn test_perception_builder() {
    let seed = WorldSeed::load("worldseeds/default.toml").unwrap();
    let world = World::new(&seed);

    let agent_id = world.agents.keys().next().unwrap().clone();

    let world_state = WorldStateBuilder::build(&world, &agent_id, 5).unwrap();

    // 构建感知摘要
    let summary = PerceptionBuilder::build_perception_summary(&world_state);

    // 验证摘要非空且包含位置信息
    assert!(!summary.is_empty());
    assert!(summary.contains("位置"));
}

/// 测试 ActionResultSchema 转换和反馈生成
#[test]
fn test_action_result_schema() {
    use agentora_core::world::ActionResult;

    // 测试 SuccessWithDetail 转换
    let legacy = ActionResult::SuccessWithDetail("move:10,20→(11,21)".to_string());
    let schema = ActionResultSchema::from_legacy(&legacy);

    assert!(schema.is_success());

    let feedback = schema.to_feedback_text();
    assert!(!feedback.is_empty());

    // 测试 Blocked 转换
    let blocked = ActionResult::Blocked("资源不足".to_string());
    let schema = ActionResultSchema::from_legacy(&blocked);

    assert!(!schema.is_success());
    let feedback = schema.to_feedback_text();
    assert!(feedback.contains("拒绝") || feedback.contains("失败"));
}

/// 测试 AgentDelta.for_broadcast() 生成精简 JSON
#[test]
fn test_agent_delta_for_broadcast() {
    let delta = AgentDelta::AgentMoved {
        id: "agent-001".to_string(),
        name: "Alice".to_string(),
        position: (100, 200),
        health: 90,
        max_health: 100,
        is_alive: true,
        age: 5,
    };

    let json = delta.for_broadcast();

    // 验证 JSON 结构
    assert!(json.is_object());
    let obj = json.as_object().unwrap();

    assert_eq!(obj.get("event_type").unwrap().as_str().unwrap(), "agent_moved");
    assert_eq!(obj.get("id").unwrap().as_str().unwrap(), "agent-001");
    assert!(obj.contains_key("position"));
    assert!(obj.contains_key("health"));
}

/// 测试 DeltaDispatcher 集中式模式
#[test]
fn test_delta_dispatcher_centralized() {
    let (tx, rx) = mpsc::channel();
    let mode = SimMode::Centralized;

    let dispatcher = DeltaDispatcher::new(tx, mode);

    let delta = AgentDelta::AgentMoved {
        id: "agent-001".to_string(),
        name: "Test".to_string(),
        position: (10, 20),
        health: 100,
        max_health: 100,
        is_alive: true,
        age: 0,
    };

    dispatcher.dispatch(delta);

    // 验证本地通道收到 delta
    let received = rx.try_recv().unwrap();
    assert_eq!(received.event_type(), "agent_moved");
}

/// 测试 SimMode 枚举
#[test]
fn test_sim_mode() {
    // 验证默认模式
    let default = SimMode::default();
    assert_eq!(default, SimMode::Centralized);

    // 验证 P2P 模式
    let p2p = SimMode::P2P {
        local_agent_ids: vec!["agent-001".to_string()],
        region_size: 32,
    };
    assert!(matches!(p2p, SimMode::P2P { .. }));
}

/// 测试 World execute_action 路由
#[test]
fn test_world_execute_action_routing() {
    let seed = WorldSeed::load("worldseeds/default.toml").unwrap();
    let mut world = World::new(&seed);

    let agent_id = world.agents.keys().next().unwrap().clone();
    let agent = world.agents.get(&agent_id).unwrap();
    let pos = agent.position;

    // 测试 Wait 动作
    let action = agentora_core::types::Action {
        reasoning: "等待".to_string(),
        action_type: ActionType::Wait,
        target: None,
        params: std::collections::HashMap::new(),
        build_type: None,
        direction: None,
    };

    let result = world.apply_action(&agent_id, &action);

    // 验证结果类型
    assert!(matches!(result, ActionResult::SuccessWithDetail(_)));
}

// ===== Phase 11-12 新增测试 =====

/// 测试 DeltaEnvelope 包装和过滤
#[test]
fn test_delta_envelope() {
    use agentora_core::simulation::{DeltaEnvelope, P2PMessageHandler};
    use agentora_core::agent::ShadowAgent;

    let delta = AgentDelta::AgentMoved {
        id: "agent-001".to_string(),
        name: "Test".to_string(),
        position: (10, 20),
        health: 100,
        max_health: 100,
        is_alive: true,
        age: 0,
    };

    // 本地 Delta
    let local_envelope = DeltaEnvelope::new(delta.clone(), 42);
    assert!(local_envelope.source_peer_id.is_none());
    assert!(!local_envelope.is_from_peer("peer-001"));

    // 远程 Delta
    let remote_envelope = DeltaEnvelope::from_remote(delta, "peer-001".to_string(), 42);
    assert!(remote_envelope.source_peer_id.is_some());
    assert!(remote_envelope.is_from_peer("peer-001"));
    assert!(!remote_envelope.is_from_peer("peer-002"));

    // for_broadcast 包含元数据
    let broadcast_json = remote_envelope.for_broadcast();
    assert!(broadcast_json.is_object());
    let obj = broadcast_json.as_object().unwrap();
    assert!(obj.contains_key("source_peer_id"));
    assert!(obj.contains_key("tick"));
}

/// 测试 ShadowAgent 创建和更新
#[test]
fn test_shadow_agent() {
    use agentora_core::agent::ShadowAgent;

    let delta = AgentDelta::AgentMoved {
        id: "agent-001".to_string(),
        name: "Shadow".to_string(),
        position: (50, 60),
        health: 80,
        max_health: 100,
        is_alive: true,
        age: 10,
    };

    // 从 AgentMoved 创建影子
    let shadow = ShadowAgent::from_moved(&delta, "peer-001", 100).unwrap();
    assert_eq!(shadow.id, "agent-001");
    assert_eq!(shadow.name, "Shadow");
    assert_eq!(shadow.position, (50, 60));
    assert_eq!(shadow.source_peer_id, "peer-001");

    // 应用死亡 Delta
    let mut shadow_mut = shadow;
    let death_delta = AgentDelta::AgentDied {
        id: "agent-001".to_string(),
        name: "Shadow".to_string(),
        position: (50, 60),
        age: 10,
    };
    shadow_mut.apply_delta(&death_delta);
    assert!(!shadow_mut.is_alive);
}

/// 测试 P2PMessageHandler 回环过滤
#[test]
fn test_p2p_message_handler_loopback_filter() {
    use agentora_core::simulation::{DeltaEnvelope, P2PMessageHandler};

    let (tx, rx) = mpsc::channel();
    let handler = P2PMessageHandler::new("peer-local".to_string(), tx, 50);

    // 本地回环 Delta（应被过滤）
    let local_delta = AgentDelta::AgentMoved {
        id: "agent-001".to_string(),
        name: "Local".to_string(),
        position: (10, 20),
        health: 100,
        max_health: 100,
        is_alive: true,
        age: 0,
    };
    let local_envelope = DeltaEnvelope::from_remote(local_delta, "peer-local".to_string(), 100);

    // 处理本地回环（应被过滤，不发送到本地通道）
    let mut h = handler;
    h.handle(&local_envelope, 100);

    // 验证本地通道无消息
    assert!(rx.try_recv().is_err());
}

/// 测试 P2PMessageHandler 处理远程 Delta
#[test]
fn test_p2p_message_handler_remote() {
    use agentora_core::simulation::{DeltaEnvelope, P2PMessageHandler};

    let (tx, rx) = mpsc::channel();
    let handler = P2PMessageHandler::new("peer-local".to_string(), tx, 50);

    // 远程 Delta（应被处理）
    let remote_delta = AgentDelta::AgentMoved {
        id: "agent-remote".to_string(),
        name: "RemoteAgent".to_string(),
        position: (100, 200),
        health: 80,
        max_health: 100,
        is_alive: true,
        age: 5,
    };
    let remote_envelope = DeltaEnvelope::from_remote(remote_delta, "peer-remote".to_string(), 100);

    let mut h = handler;
    h.handle(&remote_envelope, 100);

    // 验证本地通道收到消息
    let received = rx.try_recv().unwrap();
    assert_eq!(received.event_type(), "agent_moved");

    // 验证影子 Agent 被创建
    let shadows = h.get_shadow_agents();
    assert!(!shadows.is_empty());
}