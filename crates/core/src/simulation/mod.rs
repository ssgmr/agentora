//! 模拟编排模块
//!
//! 提供统一的模拟控制 API，管理 Agent 决策循环、世界时间推进、快照生成等。
//!
//! ## 模块结构
//!
//! - `simulation` — Simulation 结构体（封装完整编排逻辑）
//! - `config` — SimConfig 配置加载（Agent数量、决策间隔、视野半径）
//! - `delta` — Delta 增量事件（实时推送到前端）
//! - `agent_loop` — Agent 决策循环（LLM 或规则引擎）
//! - `tick_loop` — 世界时间推进（advance_tick、生存消耗）
//! - `snapshot_loop` — 定期快照生成（完整状态兜底）
//! - `npc` — NPC Agent 创建（规则引擎快速决策）
//! - `state_builder` — WorldStateBuilder（从 World 自动构建 WorldState）
//! - `delta_emitter` — DeltaEmitter（构建和发送 delta）
//! - `narrative_emitter` — NarrativeEmitter（提取和发送叙事事件）
//! - `memory_recorder` — MemoryRecorder（记录动作到 Agent 记忆）
//!
//! ## 数据流
//!
//! Simulation::new() → start()
//!   → spawn agent_loop / tick_loop / snapshot_loop
//!   → delta_tx / snapshot_tx → Bridge → Godot
//!
//! ## 使用方式
//!
//! ```rust
//! let sim = Simulation::new(config, seed, llm_provider, &llm_config);
//! sim.start();
//!
//! // 获取通道订阅
//! let snapshot_rx = sim.subscribe_snapshot();
//! let delta_rx = sim.subscribe_delta();
//!
//! // 外部控制
//! sim.toggle_pause();
//! sim.inject_preference(agent_id, key, boost, duration);
//! ```

pub mod simulation;
pub mod config;
pub mod delta;
pub mod agent_loop;
pub mod tick_loop;
pub mod snapshot_loop;
pub mod npc;
pub mod state_builder;
pub mod delta_emitter;
pub mod narrative_emitter;
pub mod memory_recorder;
pub mod delta_dispatcher;
pub mod p2p_handler;

// 重导出核心类型
pub use simulation::Simulation;
pub use config::{SimConfig, SimMode};
pub use delta::{Delta, DeltaEnvelope, ChangeHint, WorldEvent};
pub use agent_loop::NarrativeEvent;
pub use state_builder::WorldStateBuilder;
pub use delta_emitter::DeltaEmitter;
pub use narrative_emitter::NarrativeEmitter;
pub use memory_recorder::MemoryRecorder;
pub use delta_dispatcher::DeltaDispatcher;
pub use p2p_handler::P2PMessageHandler;