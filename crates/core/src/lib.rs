//! Agentora 核心引擎
//!
//! 包含动机引擎、决策管道、世界模型、Agent交互、记忆系统、策略库等核心组件。

pub mod types;
pub mod motivation;
pub mod decision;
pub mod rule_engine;
pub mod prompt;
pub mod seed;

pub mod agent;
pub mod memory;
pub mod strategy;
pub mod world;
pub mod legacy;
pub mod storage;
pub mod snapshot;

// 重导出常用类型
pub use types::*;
pub use motivation::MotivationVector;
pub use decision::{DecisionPipeline, Spark};
pub use agent::Agent;
pub use world::World;
pub use seed::WorldSeed;
pub use snapshot::WorldSnapshot;
pub use agentora_ai::config::MemoryConfig;