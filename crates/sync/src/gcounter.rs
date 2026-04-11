//! G-Counter实现
//!
//! Grow-only Counter，各Peer独立增量，合并取各分量max

use serde::{Deserialize, Serialize};
use crate::types::PeerId;
use std::collections::HashMap;

/// G-Counter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCounter {
    counts: HashMap<String, u64>,
}

impl GCounter {
    pub fn new() -> Self {
        Self { counts: HashMap::new() }
    }

    /// 增加计数（仅本地Peer可操作）
    pub fn increment(&mut self, peer_id: &PeerId, amount: u64) {
        let entry = self.counts.entry(peer_id.0.clone()).or_insert(0);
        *entry += amount;
    }

    /// 获取总计数
    pub fn total(&self) -> u64 {
        self.counts.values().sum()
    }

    /// 获取本地计数
    pub fn local_count(&self, peer_id: &PeerId) -> u64 {
        self.counts.get(&peer_id.0).copied().unwrap_or(0)
    }

    /// 合并两个G-Counter
    pub fn merge(&mut self, other: &GCounter) {
        for (peer, count) in &other.counts {
            let entry = self.counts.entry(peer.clone()).or_insert(0);
            *entry = (*entry).max(*count);
        }
    }

    /// 获取各分量
    pub fn components(&self) -> &HashMap<String, u64> {
        &self.counts
    }
}

impl Default for GCounter {
    fn default() -> Self {
        Self::new()
    }
}