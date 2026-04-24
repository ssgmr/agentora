//! AgentState统一数据模型 + WorldSnapshot序列化
//!
//! AgentState 是 Agent 状态的唯一表示，消除 AgentSnapshot 和 AgentDelta::AgentMoved 重复

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// AgentState - 统一的 Agent 状态数据模型
// ============================================================================

/// 统一的 Agent 状态表示（替代 AgentSnapshot 和 AgentDelta::AgentMoved）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub id: String,
    pub name: String,
    pub position: (u32, u32),
    pub health: u32,
    pub max_health: u32,
    pub satiety: u32,
    pub hydration: u32,
    pub age: u32,
    pub level: u32,
    pub is_alive: bool,
    pub inventory_summary: HashMap<String, u32>,
    pub current_action: String,
    pub action_result: String,
    /// Agent 的完整思考内容（本地有，远程为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

impl AgentState {
    /// 转换为 Delta（统一 Delta 构建入口）
    pub fn to_delta(&self, change_hint: crate::simulation::delta::ChangeHint) -> crate::simulation::delta::Delta {
        crate::simulation::delta::Delta::AgentStateChanged {
            agent_id: self.id.clone(),
            state: self.clone(),
            change_hint,
        }
    }
}

// ============================================================================
// NarrativeChannel + AgentSource - 叙事频道系统
// ============================================================================

/// 叙事频道分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NarrativeChannel {
    /// 本地频道（不广播）
    Local = 0,
    /// 附近频道（按区域广播）
    Nearby = 1,
    /// 世界频道（全局广播）
    World = 2,
}

impl Default for NarrativeChannel {
    fn default() -> Self {
        NarrativeChannel::Local
    }
}

/// Agent 来源标识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentSource {
    /// 本地 Agent
    Local,
    /// 远程 Agent（P2P）
    Remote { peer_id: String },
}

impl Default for AgentSource {
    fn default() -> Self {
        AgentSource::Local
    }
}

// ============================================================================
// WorldSnapshot - 世界快照（简化版）
// ============================================================================

/// 世界快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub tick: u64,
    /// 使用统一的 AgentState
    pub agents: Vec<AgentState>,
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
    /// 结构变化（建筑等）
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub structures: HashMap<(u32, u32), String>,
    /// 资源变化
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub resources: HashMap<(u32, u32), (String, u32)>,
    /// 压力事件
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub pressures: Vec<PressureSnapshot>,
    /// 里程碑（简化为列表）
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub milestones: Vec<MilestoneSnapshot>,
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

/// 叙事事件（含频道和来源）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEvent {
    pub tick: u64,
    pub agent_id: String,
    pub agent_name: String,
    pub event_type: String,
    pub description: String,
    pub color_code: String,
    /// 新增：频道归属
    #[serde(default)]
    pub channel: NarrativeChannel,
    /// 新增：来源标识
    #[serde(default)]
    pub agent_source: AgentSource,
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