//! 决策模式分类（SparkType）
//!
//! 用于策略检索和记忆分类键，根据 Agent 当前状态推断决策模式。

use serde::{Deserialize, Serialize};
use crate::decision::WorldState;

/// 决策模式分类（原 SparkType，保留用于策略/记忆分类键）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SparkType {
    /// 资源压力（饥饿/口渴/缺资源）
    ResourcePressure,
    /// 社交压力（附近有其他 Agent）
    SocialPressure,
    /// 认知压力（学习/发现）
    CognitivePressure,
    /// 表达压力（创造/建造）
    ExpressivePressure,
    /// 权力压力（领导/影响）
    PowerPressure,
    /// 传承压力（遗产/教导）
    LegacyPressure,
    /// 闲适模式（无明确压力时）
    Idle,
}

impl SparkType {
    /// 获取模式名称
    pub fn name(&self) -> &str {
        match self {
            SparkType::ResourcePressure => "资源压力",
            SparkType::SocialPressure => "社交压力",
            SparkType::CognitivePressure => "认知压力",
            SparkType::ExpressivePressure => "表达压力",
            SparkType::PowerPressure => "权力压力",
            SparkType::LegacyPressure => "传承压力",
            SparkType::Idle => "闲适",
        }
    }
}

impl std::fmt::Display for SparkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 根据 Agent 当前状态推断决策模式，用于策略检索和记忆查询。
///
/// 这替代了原来从动机缺口推导 Spark 的机制，改为直接从
/// health/satiety/hydration/inventory 等状态值推断。
pub fn infer_state_mode(world_state: &WorldState) -> SparkType {
    // 生存优先：饥饿/口渴 → 资源压力
    if world_state.agent_satiety <= 30 || world_state.agent_hydration <= 30 {
        return SparkType::ResourcePressure;
    }
    // 社交模式：附近有其他 Agent
    if !world_state.nearby_agents.is_empty() {
        return SparkType::SocialPressure;
    }
    // 闲适模式：无生存压力且无社交 → 默认闲适
    SparkType::Idle
}