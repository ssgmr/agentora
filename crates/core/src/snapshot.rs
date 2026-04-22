//! WorldSnapshot序列化与反序列化
//!
//! 用于Godot渲染的世界状态快照

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 世界快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub tick: u64,
    pub agents: Vec<AgentSnapshot>,
    /// 完整地形网格（可选，仅初始snapshot包含，用数字索引压缩存储）
    /// 地形映射: 0=plains, 1=forest, 2=mountain, 3=water, 4=desert
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain_grid: Option<Vec<u8>>,
    /// 地图宽（与 terrain_grid 配套）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain_width: Option<u32>,
    /// 地图高（与 terrain_grid 配套）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain_height: Option<u32>,
    /// 单元格变化（仅包含资源/建筑变化的格子）
    pub map_changes: Vec<CellChange>,
    pub events: Vec<NarrativeEvent>,
    pub legacies: Vec<LegacyEvent>,
    pub pressures: Vec<PressureSnapshot>,
    pub milestones: Vec<MilestoneSnapshot>,
}

/// Agent快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub id: String,
    pub name: String,
    pub position: (u32, u32),
    pub health: u32,
    pub max_health: u32,
    pub satiety: u32,
    pub hydration: u32,
    pub inventory_summary: HashMap<String, u32>,
    pub current_action: String,        // 动作类型简短描述（如"移动→(134,126)"）
    pub action_result: String,
    pub reasoning: String,             // Agent 的完整思考内容
    pub age: u32,
    pub is_alive: bool,
    pub level: u32,
}

/// 地图单元格变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellChange {
    pub x: u32,
    pub y: u32,
    pub terrain: String,
    pub structure: Option<String>,
    pub resource_type: Option<String>,
    pub resource_amount: Option<u32>,
}

/// 叙事事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEvent {
    pub tick: u64,
    pub agent_id: String,
    pub agent_name: String,
    pub event_type: String,
    pub description: String,
    pub color_code: String,  // 用于Godot颜色渲染
}

/// 遗产事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyEvent {
    pub id: String,
    pub position: (u32, u32),
    pub legacy_type: String,
    pub original_agent_name: String,
}

/// 压力快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureSnapshot {
    pub id: String,
    pub pressure_type: String,
    pub description: String,
    pub remaining_ticks: u32,
}

/// 里程碑快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneSnapshot {
    pub name: String,
    pub display_name: String,
    pub achieved_tick: u64,
}

impl WorldSnapshot {
    /// 序列化为JSON字符串
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// 从JSON解析
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}