//! 责任边界重构单元测试
//!
//! 验证 Phase 1-4 的重构正确性

use agentora_core::simulation::{WorldStateBuilder, DeltaDispatcher, SimMode, Delta, ChangeHint, WorldEvent, DeltaEnvelope};
use agentora_core::decision::PerceptionBuilder;
use agentora_core::world::{ActionResultSchema, ActionResult};
use agentora_core::snapshot::{AgentState, NarrativeEvent, NarrativeChannel, AgentSource};
use agentora_core::{World, WorldSeed, ActionType};
use agentora_core::agent::ShadowAgent;
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

/// 测试 Delta.for_broadcast() 生成精简 JSON（使用新的 AgentStateChanged）
#[test]
fn test_agent_delta_for_broadcast() {
    let state = AgentState {
        id: "agent-001".to_string(),
        name: "Alice".to_string(),
        position: (100, 200),
        health: 90,
        max_health: 100,
        satiety: 50,
        hydration: 60,
        age: 5,
        level: 1,
        is_alive: true,
        inventory_summary: std::collections::HashMap::new(),
        current_action: "gathering".to_string(),
        action_result: "success".to_string(),
        reasoning: Some("需要食物".to_string()),
    };

    let delta = Delta::AgentStateChanged {
        agent_id: "agent-001".to_string(),
        state,
        change_hint: ChangeHint::Moved,
    };

    let json = delta.for_broadcast();

    // 验证 JSON 结构
    assert!(json.is_object());
    let obj = json.as_object().unwrap();

    assert_eq!(obj.get("event_type").unwrap().as_str().unwrap(), "agent_state_changed");
    assert_eq!(obj.get("agent_id").unwrap().as_str().unwrap(), "agent-001");
    assert!(obj.contains_key("state"));
    assert!(obj.contains_key("change_hint"));
}

/// 测试 DeltaDispatcher 集中式模式
#[test]
fn test_delta_dispatcher_centralized() {
    let (tx, rx) = mpsc::channel();
    let mode = SimMode::Centralized;

    let dispatcher = DeltaDispatcher::new(tx, mode);

    let state = AgentState {
        id: "agent-001".to_string(),
        name: "Test".to_string(),
        position: (10, 20),
        health: 100,
        max_health: 100,
        satiety: 50,
        hydration: 50,
        age: 0,
        level: 1,
        is_alive: true,
        inventory_summary: std::collections::HashMap::new(),
        current_action: "waiting".to_string(),
        action_result: "".to_string(),
        reasoning: None,
    };

    let delta = Delta::AgentStateChanged {
        agent_id: "agent-001".to_string(),
        state,
        change_hint: ChangeHint::ActionExecuted,
    };

    dispatcher.dispatch(delta);

    // 验证本地通道收到 delta
    let received = rx.try_recv().unwrap();
    assert_eq!(received.event_type(), "agent_state_changed");
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
    let state = AgentState {
        id: "agent-001".to_string(),
        name: "Test".to_string(),
        position: (10, 20),
        health: 100,
        max_health: 100,
        satiety: 50,
        hydration: 50,
        age: 0,
        level: 1,
        is_alive: true,
        inventory_summary: std::collections::HashMap::new(),
        current_action: "waiting".to_string(),
        action_result: "".to_string(),
        reasoning: None,
    };

    let delta = Delta::AgentStateChanged {
        agent_id: "agent-001".to_string(),
        state,
        change_hint: ChangeHint::Moved,
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
    let state = AgentState {
        id: "agent-001".to_string(),
        name: "Shadow".to_string(),
        position: (50, 60),
        health: 80,
        max_health: 100,
        satiety: 40,
        hydration: 40,
        age: 10,
        level: 2,
        is_alive: true,
        inventory_summary: std::collections::HashMap::new(),
        current_action: "moving".to_string(),
        action_result: "".to_string(),
        reasoning: None,
    };

    // 从 AgentState 创建影子
    let shadow = ShadowAgent::from_state(&state, "peer-001", 100);
    assert_eq!(shadow.state.id, "agent-001");
    assert_eq!(shadow.state.name, "Shadow");
    assert_eq!(shadow.state.position, (50, 60));
    assert_eq!(shadow.source_peer_id, "peer-001");

    // 应用死亡 Delta
    let mut shadow_mut = shadow;
    let death_state = AgentState {
        id: "agent-001".to_string(),
        name: "Shadow".to_string(),
        position: (50, 60),
        health: 0,
        max_health: 100,
        satiety: 0,
        hydration: 0,
        age: 10,
        level: 2,
        is_alive: false,
        inventory_summary: std::collections::HashMap::new(),
        current_action: "".to_string(),
        action_result: "".to_string(),
        reasoning: None,
    };
    let death_delta = Delta::AgentStateChanged {
        agent_id: "agent-001".to_string(),
        state: death_state,
        change_hint: ChangeHint::Died,
    };
    shadow_mut.apply_delta(&death_delta);
    assert!(!shadow_mut.state.is_alive);
}

/// 测试 WorldEvent 序列化
#[test]
fn test_world_event_for_broadcast() {
    let event = WorldEvent::StructureCreated {
        pos: (100, 200),
        structure_type: "Camp".to_string(),
        owner_id: "agent-001".to_string(),
    };

    let json = event.for_broadcast();
    assert!(json.is_object());
    let obj = json.as_object().unwrap();
    assert_eq!(obj.get("event_type").unwrap().as_str().unwrap(), "structure_created");
    assert!(obj.contains_key("pos"));
    assert!(obj.contains_key("structure_type"));
    assert!(obj.contains_key("owner_id"));
}

/// 测试 NarrativeChannel 和 AgentSource
#[test]
fn test_narrative_channel_and_source() {
    // Local narrative
    let local_event = NarrativeEvent {
        tick: 100,
        agent_id: "agent-001".to_string(),
        agent_name: "Alice".to_string(),
        event_type: "gather".to_string(),
        description: "采集食物".to_string(),
        color_code: "#00FF00".to_string(),
        channel: NarrativeChannel::Local,
        agent_source: AgentSource::Local,
    };
    assert_eq!(local_event.channel, NarrativeChannel::Local);

    // World narrative
    let world_event = NarrativeEvent {
        tick: 100,
        agent_id: "agent-001".to_string(),
        agent_name: "Alice".to_string(),
        event_type: "death".to_string(),
        description: "死亡".to_string(),
        color_code: "#FF0000".to_string(),
        channel: NarrativeChannel::World,
        agent_source: AgentSource::Local,
    };
    assert_eq!(world_event.channel, NarrativeChannel::World);

    // Remote narrative
    let remote_event = NarrativeEvent {
        tick: 100,
        agent_id: "agent-002".to_string(),
        agent_name: "Bob".to_string(),
        event_type: "gather".to_string(),
        description: "采集木材".to_string(),
        color_code: "#00FF00".to_string(),
        channel: NarrativeChannel::Nearby,
        agent_source: AgentSource::Remote { peer_id: "peer-002".to_string() },
    };
    assert_eq!(remote_event.channel, NarrativeChannel::Nearby);
    assert!(matches!(remote_event.agent_source, AgentSource::Remote { .. }));
}