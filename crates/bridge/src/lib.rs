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
//! - `user_config` — 用户配置管理（引导页面）
//! - `icon_processor` — 图标缩放处理

mod logging;
mod conversion;
mod bridge;
mod simulation_runner;
pub mod user_config;
mod icon_processor;

// 重导出核心类型
pub use bridge::{SimulationBridge, SimCommand};
pub use user_config::UserConfig;

use godot::prelude::*;
use godot::init::ExtensionLibrary;

struct AgentoraExtension;

#[gdextension(entry_symbol = agentora_bridge_init)]
unsafe impl ExtensionLibrary for AgentoraExtension {}