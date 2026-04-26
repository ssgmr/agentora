//! Delta 增量事件（简化版）
//!
//! Delta 从 14 种变体简化为 AgentStateChanged + WorldEvent 两类
//! 废弃旧的 AgentDelta 枚举

use serde::{Deserialize, Serialize};
use serde_json;
use crate::snapshot::{AgentState, NarrativeEvent};

// ============================================================================
// ChangeHint - Agent 状态变化标记
// ============================================================================

/// Agent 状态变化标记（用于客户端判断如何处理 AgentStateChanged）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeHint {
    /// 新 Agent 首次出现
    Spawned,
    /// 位置变化
    Moved,
    /// 动作执行后
    ActionExecuted,
    /// 死亡
    Died,
    /// 生存状态警告
    SurvivalLow,
    /// 营地治愈
    Healed,
}

// ============================================================================
// WorldEvent - 世界级事件
// ============================================================================

/// 世界级事件（非 Agent 状态变化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldEvent {
    /// 建筑创建
    StructureCreated {
        pos: (u32, u32),
        structure_type: String,
        owner_id: String,
    },
    /// 建筑销毁
    StructureDestroyed {
        pos: (u32, u32),
        structure_type: String,
    },
    /// 资源变化
    ResourceChanged {
        pos: (u32, u32),
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
    /// Agent 叙事（通过 WorldEvent 广播）
    AgentNarrative {
        narrative: NarrativeEvent,
    },
}

impl WorldEvent {
    /// 返回事件类型名称
    pub fn event_type(&self) -> &'static str {
        match self {
            WorldEvent::StructureCreated { .. } => "structure_created",
            WorldEvent::StructureDestroyed { .. } => "structure_destroyed",
            WorldEvent::ResourceChanged { .. } => "resource_changed",
            WorldEvent::TradeCompleted { .. } => "trade_completed",
            WorldEvent::AllianceFormed { .. } => "alliance_formed",
            WorldEvent::AllianceBroken { .. } => "alliance_broken",
            WorldEvent::MilestoneReached { .. } => "milestone_reached",
            WorldEvent::PressureStarted { .. } => "pressure_started",
            WorldEvent::PressureEnded { .. } => "pressure_ended",
            WorldEvent::AgentNarrative { .. } => "agent_narrative",
        }
    }

    /// P2P 广播用精简 JSON
    pub fn for_broadcast(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("event_type".to_string(), serde_json::json!(self.event_type()));

        match self {
            WorldEvent::StructureCreated { pos, structure_type, owner_id } => {
                obj.insert("pos".to_string(), serde_json::json!(pos));
                obj.insert("structure_type".to_string(), serde_json::json!(structure_type));
                obj.insert("owner_id".to_string(), serde_json::json!(owner_id));
            }
            WorldEvent::StructureDestroyed { pos, structure_type } => {
                obj.insert("pos".to_string(), serde_json::json!(pos));
                obj.insert("structure_type".to_string(), serde_json::json!(structure_type));
            }
            WorldEvent::ResourceChanged { pos, resource_type, amount } => {
                obj.insert("pos".to_string(), serde_json::json!(pos));
                obj.insert("resource_type".to_string(), serde_json::json!(resource_type));
                obj.insert("amount".to_string(), serde_json::json!(amount));
            }
            WorldEvent::TradeCompleted { from_id, to_id, items } => {
                obj.insert("from_id".to_string(), serde_json::json!(from_id));
                obj.insert("to_id".to_string(), serde_json::json!(to_id));
                obj.insert("items".to_string(), serde_json::json!(items));
            }
            WorldEvent::AllianceFormed { id1, id2 } => {
                obj.insert("ids".to_string(), serde_json::json!([id1, id2]));
            }
            WorldEvent::AllianceBroken { id1, id2, reason } => {
                obj.insert("ids".to_string(), serde_json::json!([id1, id2]));
                obj.insert("reason".to_string(), serde_json::json!(reason));
            }
            WorldEvent::MilestoneReached { name, display_name, tick } => {
                obj.insert("name".to_string(), serde_json::json!(name));
                obj.insert("display_name".to_string(), serde_json::json!(display_name));
                obj.insert("tick".to_string(), serde_json::json!(tick));
            }
            WorldEvent::PressureStarted { pressure_type, description, duration } => {
                obj.insert("pressure_type".to_string(), serde_json::json!(pressure_type));
                obj.insert("description".to_string(), serde_json::json!(description));
                obj.insert("duration".to_string(), serde_json::json!(duration));
            }
            WorldEvent::PressureEnded { pressure_type, description } => {
                obj.insert("pressure_type".to_string(), serde_json::json!(pressure_type));
                obj.insert("description".to_string(), serde_json::json!(description));
            }
            WorldEvent::AgentNarrative { narrative } => {
                obj.insert("narrative".to_string(), serde_json::to_value(narrative).unwrap());
            }
        }

        serde_json::Value::Object(obj)
    }
}

// ============================================================================
// Delta - 简化的增量事件枚举
// ============================================================================

/// 简化的 Delta 枚举（仅两类）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Delta {
    /// Agent 状态变化（统一所有 Agent 相关事件）
    AgentStateChanged {
        agent_id: String,
        state: AgentState,
        change_hint: ChangeHint,
        /// 来源 peer ID（P2P 远程 Agent，None = 本地）
        #[serde(skip_serializing_if = "Option::is_none")]
        source_peer_id: Option<String>,
    },
    /// 世界级事件（非 Agent 状态变化）
    WorldEvent(WorldEvent),
}

impl Delta {
    /// 返回事件类型名称
    pub fn event_type(&self) -> &'static str {
        match self {
            Delta::AgentStateChanged { .. } => "agent_state_changed",
            Delta::WorldEvent(e) => e.event_type(),
        }
    }

    /// 获取 Agent ID（如果是 AgentStateChanged）
    pub fn agent_id(&self) -> Option<&str> {
        match self {
            Delta::AgentStateChanged { agent_id, .. } => Some(agent_id),
            _ => None,
        }
    }

    /// P2P 广播用精简 JSON
    pub fn for_broadcast(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("event_type".to_string(), serde_json::json!(self.event_type()));

        match self {
            Delta::AgentStateChanged { agent_id, state, change_hint, source_peer_id } => {
                obj.insert("agent_id".to_string(), serde_json::json!(agent_id));
                obj.insert("state".to_string(), serde_json::to_value(state).unwrap());
                obj.insert("change_hint".to_string(), serde_json::json!(change_hint));
                if let Some(ref peer_id) = source_peer_id {
                    obj.insert("source_peer_id".to_string(), serde_json::json!(peer_id));
                }
            }
            Delta::WorldEvent(e) => {
                let event_obj = e.for_broadcast();
                if let serde_json::Value::Object(event_map) = event_obj {
                    for (k, v) in event_map {
                        if k != "event_type" {
                            obj.insert(k, v);
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(obj)
    }
}

// ============================================================================
// calculate_region_id - 区域 ID 计算
// ============================================================================

/// 根据 Agent 位置计算区域 ID
///
/// # 参数
/// - `position`: Agent 的 (x, y) 坐标
/// - `map_width`: 地图宽度
/// - `region_size`: 每个区域的格子大小
///
/// # 返回
/// 区域 ID（用于 P2P GossipSub topic 订阅）
pub fn calculate_region_id(position: (u32, u32), map_width: u32, region_size: u32) -> u32 {
    let region_x = position.0 / region_size;
    let region_y = position.1 / region_size;
    let regions_per_row = (map_width + region_size - 1) / region_size;
    region_y * regions_per_row + region_x
}

// ============================================================================
// DeltaEnvelope - P2P 包装结构
// ============================================================================

/// Delta 包装结构体（P2P 模式）
///
/// 包含原始 Delta + 元数据（source_peer_id 用于过滤本地回环）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaEnvelope {
    /// 原始 Delta
    pub delta: Delta,
    /// 来源 peer ID（P2P 模式下用于过滤本地回环）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_peer_id: Option<String>,
    /// 时间戳（tick 或 wall-clock）
    pub tick: u64,
}

impl DeltaEnvelope {
    /// 创建新的 DeltaEnvelope（本地产生）
    pub fn new(delta: Delta, tick: u64) -> Self {
        Self {
            delta,
            source_peer_id: None,
            tick,
        }
    }

    /// 创建来自远程 peer 的 DeltaEnvelope
    pub fn from_remote(delta: Delta, source_peer_id: String, tick: u64) -> Self {
        Self {
            delta,
            source_peer_id: Some(source_peer_id),
            tick,
        }
    }

    /// 检查是否来自指定 peer（用于过滤本地回环）
    pub fn is_from_peer(&self, peer_id: &str) -> bool {
        self.source_peer_id.as_ref().map(|p| p == peer_id).unwrap_or(false)
    }

    /// 获取精简 JSON（用于 P2P 广播）
    pub fn for_broadcast(&self) -> serde_json::Value {
        let mut obj = self.delta.for_broadcast().as_object().cloned().unwrap_or_default();
        if let Some(ref peer_id) = self.source_peer_id {
            obj.insert("source_peer_id".to_string(), serde_json::json!(peer_id));
        }
        obj.insert("tick".to_string(), serde_json::json!(self.tick));
        serde_json::Value::Object(obj)
    }
}

