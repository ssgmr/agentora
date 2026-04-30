//! CRDT操作序列化与广播

use serde::{Deserialize, Serialize};

/// CRDT操作定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtOp {
    LwwSet {
        key: String,
        value: Vec<u8>,
        timestamp: u64,
        peer_id: String,
    },
    GCounterInc {
        key: String,
        amount: u64,
        peer_id: String,
    },
    OrSetAdd {
        key: String,
        element: Vec<u8>,
        tag: (String, u64),
    },
    OrSetRemove {
        key: String,
        tag: (String, u64),
    },
}

impl CrdtOp {
    /// 序列化为JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// 从JSON解析
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 获取操作的 peer ID
    pub fn peer_id(&self) -> &str {
        match self {
            CrdtOp::LwwSet { peer_id, .. } => peer_id,
            CrdtOp::GCounterInc { peer_id, .. } => peer_id,
            CrdtOp::OrSetAdd { tag, .. } => &tag.0,
            CrdtOp::OrSetRemove { tag, .. } => &tag.0,
        }
    }
}

/// Agent Delta 广播消息（P2P 模式）
///
/// 用于远程 Agent 状态同步
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeltaMessage {
    /// Delta JSON（for_broadcast 格式）
    pub delta_json: serde_json::Value,
    /// 来源 peer ID
    pub source_peer_id: String,
    /// tick 时间戳
    pub tick: u64,
}

/// 叙事事件广播消息（P2P 模式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeMessage {
    /// 叙事事件 JSON
    pub narrative_json: serde_json::Value,
    /// 来源 peer ID
    pub source_peer_id: String,
    /// tick 时间戳
    pub tick: u64,
    /// 叙事频道（"local" / "nearby" / "world"）
    pub channel: String,
}

/// 网络消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    CrdtOp(CrdtOp),
    SyncRequest { peer_id: String, merkle_root: String },
    SyncResponse { ops: Vec<CrdtOp> },
    LegacyBroadcast(LegacyBroadcastMessage),
    PeerInfo { peer_id: String, position: (u32, u32) },
    /// Agent Delta 广播（P2P 模式）
    AgentDelta(AgentDeltaMessage),
    /// 叙事事件广播（P2P 模式）
    Narrative(NarrativeMessage),
    /// WorldSeed 同步（种子节点 -> 加入节点）
    WorldSeedSync {
        /// WorldSeed 序列化后的 JSON 字符串
        world_seed_json: String,
        /// 来源 peer ID
        source_peer_id: String,
    },
}

/// 遗产广播消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyBroadcastMessage {
    pub legacy_id: String,
    pub original_agent_id: String,
    pub original_agent_name: String,
    pub position: (u32, u32),
    pub legacy_type: String,
    pub created_tick: u64,
    pub summary: String,
}

impl NetworkMessage {
    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }

    /// 从字节解析
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}