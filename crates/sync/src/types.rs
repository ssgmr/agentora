//! CRDT同步类型定义

use serde::{Deserialize, Serialize};

/// Peer标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for PeerId {
    fn default() -> Self {
        Self::new("")
    }
}