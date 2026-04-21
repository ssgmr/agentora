//! Agent 增量事件
//!
//! 用于实时推送 Agent 状态变化到前端

use serde::{Deserialize, Serialize};

/// Agent 增量事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentDelta {
    /// Agent 移动或状态变化
    AgentMoved {
        id: String,
        name: String,
        position: (u32, u32),
        health: u32,
        max_health: u32,
        is_alive: bool,
        age: u32,
    },
    /// Agent 死亡
    AgentDied {
        id: String,
        name: String,
        position: (u32, u32),
        age: u32,
    },
    /// 新 Agent 诞生
    AgentSpawned {
        id: String,
        name: String,
        position: (u32, u32),
        health: u32,
        max_health: u32,
    },

    // ===== Tier 2 新增 =====
    /// 建筑创建
    StructureCreated {
        x: u32,
        y: u32,
        structure_type: String,
        owner_id: String,
    },
    /// 建筑销毁
    StructureDestroyed {
        x: u32,
        y: u32,
        structure_type: String,
    },
    /// 资源变化
    ResourceChanged {
        x: u32,
        y: u32,
        resource_type: String,
        amount: u32,
    },
    /// 交易完成
    TradeCompleted {
        from_id: String,
        to_id: String,
        items: String,
    },
    /// 联盟建立
    AllianceFormed {
        id1: String,
        id2: String,
    },
    /// 联盟破裂
    AllianceBroken {
        id1: String,
        id2: String,
        reason: String,
    },

    // ===== Tier 2.5 新增：生存+建筑+压力+里程碑 =====
    /// 营地治愈
    HealedByCamp {
        agent_id: String,
        agent_name: String,
        hp_restored: u32,
    },
    /// 生存状态警告
    SurvivalWarning {
        agent_id: String,
        agent_name: String,
        satiety: u32,
        hydration: u32,
        hp: u32,
    },
    /// 里程碑达成
    MilestoneReached {
        name: String,
        display_name: String,
        tick: u64,
    },
    /// 压力事件开始
    PressureStarted {
        pressure_type: String,
        description: String,
        duration: u32,
    },
    /// 压力事件结束
    PressureEnded {
        pressure_type: String,
        description: String,
    },
}