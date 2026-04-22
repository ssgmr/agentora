//! Agent 增量事件
//!
//! 用于实时推送 Agent 状态变化到前端

use serde::{Deserialize, Serialize};
use serde_json;

/// Delta 包装结构体（P2P 模式）
///
/// 包含原始 Delta + 元数据（source_peer_id 用于过滤本地回环）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaEnvelope {
    /// 原始 Delta
    pub delta: AgentDelta,
    /// 来源 peer ID（P2P 模式下用于过滤本地回环）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_peer_id: Option<String>,
    /// 时间戳（tick 或 wall-clock）
    pub tick: u64,
}

impl DeltaEnvelope {
    /// 创建新的 DeltaEnvelope（本地产生）
    pub fn new(delta: AgentDelta, tick: u64) -> Self {
        Self {
            delta,
            source_peer_id: None,
            tick,
        }
    }

    /// 创建来自远程 peer 的 DeltaEnvelope
    pub fn from_remote(delta: AgentDelta, source_peer_id: String, tick: u64) -> Self {
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

impl AgentDelta {
    /// 返回事件类型名称（用于 P2P topic 路由）
    pub fn event_type(&self) -> &'static str {
        match self {
            AgentDelta::AgentMoved { .. } => "agent_moved",
            AgentDelta::AgentDied { .. } => "agent_died",
            AgentDelta::AgentSpawned { .. } => "agent_spawned",
            AgentDelta::StructureCreated { .. } => "structure_created",
            AgentDelta::StructureDestroyed { .. } => "structure_destroyed",
            AgentDelta::ResourceChanged { .. } => "resource_changed",
            AgentDelta::TradeCompleted { .. } => "trade_completed",
            AgentDelta::AllianceFormed { .. } => "alliance_formed",
            AgentDelta::AllianceBroken { .. } => "alliance_broken",
            AgentDelta::HealedByCamp { .. } => "healed_by_camp",
            AgentDelta::SurvivalWarning { .. } => "survival_warning",
            AgentDelta::MilestoneReached { .. } => "milestone_reached",
            AgentDelta::PressureStarted { .. } => "pressure_started",
            AgentDelta::PressureEnded { .. } => "pressure_ended",
        }
    }

    /// P2P 广播用精简 JSON
    ///
    /// 只包含核心字段，移除内部追踪信息，满足 P2P 带宽约束。
    pub fn for_broadcast(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("event_type".to_string(), serde_json::json!(self.event_type()));

        match self {
            AgentDelta::AgentMoved { id, name, position, health, max_health, is_alive, age } => {
                obj.insert("id".to_string(), serde_json::json!(id));
                obj.insert("name".to_string(), serde_json::json!(name));
                obj.insert("position".to_string(), serde_json::json!(position));
                obj.insert("health".to_string(), serde_json::json!([health, max_health]));
                obj.insert("is_alive".to_string(), serde_json::json!(is_alive));
                obj.insert("age".to_string(), serde_json::json!(age));
            }
            AgentDelta::AgentDied { id, name, position, age } => {
                obj.insert("id".to_string(), serde_json::json!(id));
                obj.insert("name".to_string(), serde_json::json!(name));
                obj.insert("position".to_string(), serde_json::json!(position));
                obj.insert("age".to_string(), serde_json::json!(age));
            }
            AgentDelta::AgentSpawned { id, name, position, health, max_health } => {
                obj.insert("id".to_string(), serde_json::json!(id));
                obj.insert("name".to_string(), serde_json::json!(name));
                obj.insert("position".to_string(), serde_json::json!(position));
                obj.insert("health".to_string(), serde_json::json!([health, max_health]));
            }
            AgentDelta::StructureCreated { x, y, structure_type, owner_id } => {
                obj.insert("position".to_string(), serde_json::json!([x, y]));
                obj.insert("structure_type".to_string(), serde_json::json!(structure_type));
                obj.insert("owner_id".to_string(), serde_json::json!(owner_id));
            }
            AgentDelta::StructureDestroyed { x, y, structure_type } => {
                obj.insert("position".to_string(), serde_json::json!([x, y]));
                obj.insert("structure_type".to_string(), serde_json::json!(structure_type));
            }
            AgentDelta::ResourceChanged { x, y, resource_type, amount } => {
                obj.insert("position".to_string(), serde_json::json!([x, y]));
                obj.insert("resource_type".to_string(), serde_json::json!(resource_type));
                obj.insert("amount".to_string(), serde_json::json!(amount));
            }
            AgentDelta::TradeCompleted { from_id, to_id, items } => {
                obj.insert("from_id".to_string(), serde_json::json!(from_id));
                obj.insert("to_id".to_string(), serde_json::json!(to_id));
                obj.insert("items".to_string(), serde_json::json!(items));
            }
            AgentDelta::AllianceFormed { id1, id2 } | AgentDelta::AllianceBroken { id1, id2, .. } => {
                obj.insert("ids".to_string(), serde_json::json!([id1, id2]));
            }
            AgentDelta::HealedByCamp { agent_id, agent_name, hp_restored } => {
                obj.insert("agent_id".to_string(), serde_json::json!(agent_id));
                obj.insert("agent_name".to_string(), serde_json::json!(agent_name));
                obj.insert("hp_restored".to_string(), serde_json::json!(hp_restored));
            }
            AgentDelta::SurvivalWarning { agent_id, agent_name, satiety, hydration, hp } => {
                obj.insert("agent_id".to_string(), serde_json::json!(agent_id));
                obj.insert("agent_name".to_string(), serde_json::json!(agent_name));
                obj.insert("status".to_string(), serde_json::json!({
                    "satiety": satiety, "hydration": hydration, "hp": hp
                }));
            }
            AgentDelta::MilestoneReached { name, display_name, tick } => {
                obj.insert("name".to_string(), serde_json::json!(name));
                obj.insert("display_name".to_string(), serde_json::json!(display_name));
                obj.insert("tick".to_string(), serde_json::json!(tick));
            }
            AgentDelta::PressureStarted { pressure_type, description, duration } => {
                obj.insert("pressure_type".to_string(), serde_json::json!(pressure_type));
                obj.insert("description".to_string(), serde_json::json!(description));
                obj.insert("duration".to_string(), serde_json::json!(duration));
            }
            AgentDelta::PressureEnded { pressure_type, description } => {
                obj.insert("pressure_type".to_string(), serde_json::json!(pressure_type));
                obj.insert("description".to_string(), serde_json::json!(description));
            }
        }

        serde_json::Value::Object(obj)
    }
}