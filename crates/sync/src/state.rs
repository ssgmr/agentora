//! SyncState合并与reconcile

use crate::lww::LwwRegister;
use crate::gcounter::GCounter;
use crate::orset::OrSet;
use crate::codec::CrdtOp;
use crate::types::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 同步状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Agent状态 (LWW-Register)
    agent_states: HashMap<String, LwwRegister<Vec<u8>>>,
    /// 资源采集量 (G-Counter)
    resource_counts: HashMap<String, GCounter>,
    /// 事件日志 (OR-Set)
    event_log: OrSet<Vec<u8>>,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            agent_states: HashMap::new(),
            resource_counts: HashMap::new(),
            event_log: OrSet::new(),
        }
    }

    /// 应用CRDT操作
    pub fn apply_op(&mut self, op: &CrdtOp, peer_id: &PeerId) {
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
                // TODO: 实现删除
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