//! LWW-Register实现
//!
//! Last-Write-Wins Register，取timestamp最大的值

use serde::{Deserialize, Serialize};
use crate::types::PeerId;

/// LWW-Register
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LwwRegister<T> {
    value: T,
    timestamp: u64,
    peer_id: PeerId,
}

impl<T: Clone> LwwRegister<T> {
    pub fn new(value: T, timestamp: u64, peer_id: PeerId) -> Self {
        Self { value, timestamp, peer_id }
    }

    /// 设置值
    pub fn set(&mut self, value: T, timestamp: u64, peer_id: PeerId) {
        if timestamp > self.timestamp || (timestamp == self.timestamp && peer_id.0 > self.peer_id.0) {
            self.value = value;
            self.timestamp = timestamp;
            self.peer_id = peer_id;
        }
    }

    /// 获取值
    pub fn get(&self) -> &T {
        &self.value
    }

    /// 合并两个LWW-Register
    pub fn merge(&mut self, other: &LwwRegister<T>) {
        if other.timestamp > self.timestamp || (other.timestamp == self.timestamp && other.peer_id.0 > self.peer_id.0) {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.peer_id = other.peer_id.clone();
        }
    }

    /// 获取timestamp
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}