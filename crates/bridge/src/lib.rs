//! Agentora Godot GDExtension 桥接
//!
//! 薄桥接层：SimulationBridge 节点定义 + 类型转换 + 信号发射
//!
//! ## 模块结构
//!
//! - `bridge` — SimulationBridge 节点定义 + INode 实现 + GDExtension API
//! - `conversion` — delta_to_dict, agent_to_dict, snapshot_to_dict 类型转换
//! - `logging` — LogConfig 配置 + init_logging 初始化
//! - `simulation_runner` — 模拟线程运行逻辑

mod logging;
mod conversion;
mod bridge;
mod simulation_runner;

// 重导出核心类型
pub use bridge::{SimulationBridge, SimCommand};

use godot::prelude::*;
use godot::init::ExtensionLibrary;

struct AgentoraExtension;

#[gdextension(entry_symbol = agentora_bridge_init)]
unsafe impl ExtensionLibrary for AgentoraExtension {}