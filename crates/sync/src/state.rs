//! SyncState合并与reconcile

use crate::lww::LwwRegister;
use crate::gcounter::GCounter;
use crate::orset::OrSet;
use crate::codec::CrdtOp;
use crate::types::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CRDT key schema 常量
pub const KEY_SCHEMA: &str = "agentora";

/// Key schema helper functions
pub mod key_schema {
    /// Agent 位置 key
    pub fn agent_position(agent_id: &str) -> String {
        format!("agent:{}:position", agent_id)
    }

    /// Agent 状态 key
    pub fn agent_state(agent_id: &str) -> String {
        format!("agent:{}:state", agent_id)
    }

    /// Agent 健康 key
    pub fn agent_health(agent_id: &str) -> String {
        format!("agent:{}:health", agent_id)
    }

    /// 资源计数 key
    pub fn resource_count(resource_type: &str, position: &(u32, u32)) -> String {
        format!("resource:{}:({},{})", resource_type, position.0, position.1)
    }

    /// 事件日志 key
    pub fn event_log() -> String {
        "event_log".to_string()
    }
}

/// 同步状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Agent状态 (LWW-Register)
    agent_states: HashMap<String, LwwRegister<Vec<u8>>>,
    /// 资源采集量 (G-Counter)
    resource_counts: HashMap<String, GCounter>,
    /// 事件日志 (OR-Set)
    event_log: OrSet<Vec<u8>>,
    /// 已删除元素的 tag 缓存（用于 OrSetRemove）
    removed_tags: HashMap<String, Vec<(String, u64)>>,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            agent_states: HashMap::new(),
            resource_counts: HashMap::new(),
            event_log: OrSet::new(),
            removed_tags: HashMap::new(),
        }
    }

    /// 应用CRDT操作
    pub fn apply_op(&mut self, op: &CrdtOp, _peer_id: &PeerId) {
        match op {
            CrdtOp::LwwSet { key, value, timestamp, peer_id } => {
                let register = self.agent_states.entry(key.clone()).or_insert(
                    LwwRegister::new(vec![], 0, PeerId::new(""))
                );
                register.set(value.clone(), *timestamp, PeerId::new(peer_id));
            }
            CrdtOp::GCounterInc { key, amount, peer_id } => {
                let counter = self.resource_counts.entry(key.clone()).or_insert_with(GCounter::new);
                counter.increment(&PeerId::new(peer_id), *amount);
            }
            CrdtOp::OrSetAdd { key, element, tag } => {
                if key == "event_log" {
                    self.event_log.add(element.clone(), &PeerId::new(&tag.0), tag.1);
                }
            }
            CrdtOp::OrSetRemove { key, tag } => {
                // OrSetRemove 实现：记录 tag 用于后续合并时过滤
                if key == "event_log" {
                    // 从 event_log 中移除对应 tag 的元素
                    self.event_log.remove_with_tag(&PeerId::new(&tag.0), tag.1);
                } else {
                    // 其他 key 的 remove，记录到 removed_tags 缓存
                    let tags = self.removed_tags.entry(key.clone()).or_default();
                    tags.push(tag.clone());
                }
            }
        }
    }

    /// 批量合并
    pub fn merge(&mut self, other: &SyncState) {
        // 合并Agent状态
        for (key, register) in &other.agent_states {
            let local = self.agent_states.entry(key.clone()).or_insert(
                LwwRegister::new(vec![], 0, PeerId::new(""))
            );
            local.merge(register);
        }

        // 合并计数器
        for (key, counter) in &other.resource_counts {
            let local = self.resource_counts.entry(key.clone()).or_insert_with(GCounter::new);
            local.merge(counter);
        }

        // 合并事件日志
        self.event_log.merge(&other.event_log);
    }

    /// 生成Merkle根
    pub fn merkle_root(&self) -> String {
        let items: Vec<Vec<u8>> = self.agent_states.iter()
            .flat_map(|(k, r)| [k.as_bytes().to_vec(), r.get().clone()])
            .collect();
        crate::merkle::compute_merkle_root(&items)
    }
}

impl Default for SyncState {
    fn default() -> Self {
        Self::new()
    }
}