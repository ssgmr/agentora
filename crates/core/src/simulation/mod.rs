//! 模拟编排模块
//!
//! 提供统一的模拟控制 API，管理 Agent 决策循环、世界时间推进、快照生成等。
//!
//! ## 模块结构
//!
//! - `config` — SimConfig 配置加载（Agent数量、决策间隔、视野半径）
//! - `delta` — AgentDelta 增量事件（实时推送到前端）
//! - `agent_loop` — Agent 决策循环（LLM 或规则引擎）
//! - `tick_loop` — 世界时间推进（advance_tick、生存消耗）
//! - `snapshot_loop` — 定期快照生成（完整状态兜底）
//! - `npc` — NPC Agent 创建（规则引擎快速决策）
//!
//! ## 数据流
//!
//! World (Arc<Mutex>) -> agent_loop -> delta_tx -> Godot
//! World -> snapshot_loop -> snapshot_tx -> Godot
//! World -> tick_loop -> advance_tick
//!
//! ## 使用方式
//!
//! Bridge 通过以下函数调用本模块：
//! - `SimConfig::load()` 加载配置
//! - `agent_loop::run_agent_loop()` 运行 Agent 决策循环
//! - `tick_loop::run_tick_loop()` 运行世界时间推进
//! - `snapshot_loop::run_snapshot_loop()` 运行快照生成
//! - `npc::create_npc_agents()` 创建 NPC Agent

pub mod config;
pub mod delta;
pub mod agent_loop;
pub mod tick_loop;
pub mod snapshot_loop;
pub mod npc;

// 重导出核心类型
pub use config::SimConfig;
pub use delta::AgentDelta;