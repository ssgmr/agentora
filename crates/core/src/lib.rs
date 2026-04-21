//! Agentora 核心引擎
//!
//! 包含动机引擎、决策管道、世界模型、Agent交互、记忆系统、策略库等核心组件。

pub mod types;
pub mod decision;
pub mod rule_engine;
pub mod prompt;
pub mod seed;
pub mod narrative;

pub mod agent;
pub mod memory;
pub mod strategy;
pub mod world;
pub mod storage;
pub mod snapshot;
pub mod simulation;

// 重导出常用类型
pub use types::*;
pub use decision::DecisionPipeline;
pub use agent::Agent;
pub use world::World;
pub use seed::WorldSeed;
pub use snapshot::WorldSnapshot;
pub use narrative::{NarrativeBuilder, EventType, action_type_display};
pub use simulation::{SimConfig, AgentDelta};

// 重导出 legacy 和 vision 类型（已移动到 world 模块）
pub use world::legacy::{Legacy, LegacyType, EchoLog, LegacyInteractionType, LegacyInteractionResult, LegacyEvent};
pub use world::vision::{scan_vision, calculate_direction, direction_description, VisionScanResult, NearbyAgentInfo, NearbyStructureInfo, NearbyLegacyInfo};