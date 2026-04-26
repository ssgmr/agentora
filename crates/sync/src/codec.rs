//! CRDT操作序列化与编解码

use crate::types::PeerId;
use crate::orset::ElementTag;
use serde::{Deserialize, Serialize};

/// CRDT操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrdtOp {
    /// LWW-Register设置
    LwwSet {
        key: String,
        value: Vec<u8>,
        timestamp: u64,
        peer_id: String,
    },
    /// G-Counter增量
    GCounterInc {
        key: String,
        amount: u64,
        peer_id: String,
    },
    /// OR-Set添加
    OrSetAdd {
        key: String,
        element: Vec<u8>,
        tag: (String, u64),
    },
    /// OR-Set删除
    OrSetRemove {
        key: String,
        tag: (String, u64),
    },
}

impl CrdtOp {
    /// 序列化为JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 从JSON反序列化
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 创建LWW设置操作
    pub fn lww_set(key: String, value: Vec<u8>, timestamp: u64, peer_id: &PeerId) -> Self {
        CrdtOp::LwwSet {
            key,
            value,
            timestamp,
            peer_id: peer_id.0.clone(),
        }
    }

    /// 创建计数器增量操作
    pub fn gcounter_inc(key: String, amount: u64, peer_id: &PeerId) -> Self {
        CrdtOp::GCounterInc {
            key,
            amount,
            peer_id: peer_id.0.clone(),
        }
    }

    /// 创建OR-Set添加操作
    pub fn orset_add(key: String, element: Vec<u8>, tag: &ElementTag) -> Self {
        CrdtOp::OrSetAdd {
            key,
            element,
            tag: (tag.peer_id.clone(), tag.counter),
        }
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