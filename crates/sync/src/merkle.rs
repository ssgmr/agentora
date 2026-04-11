//! Merkle根校验

use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

/// Merkle树节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    hash: String,
    left: Option<Box<MerkleNode>>,
    right: Option<Box<MerkleNode>>,
}

impl MerkleNode {
    /// 计算叶子节点hash
    pub fn leaf(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hex::encode(hasher.finalize());
        Self { hash, left: None, right: None }
    }

    /// 计算中间节点hash
    pub fn combine(left: MerkleNode, right: MerkleNode) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(left.hash.as_bytes());
        hasher.update(right.hash.as_bytes());
        let hash = hex::encode(hasher.finalize());
        Self {
            hash,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    /// 获取hash值
    pub fn hash(&self) -> &str {
        &self.hash
    }
}

/// 计算Merkle根
pub fn compute_merkle_root(items: &[Vec<u8>]) -> String {
    if items.is_empty() {
        return "0".to_string();
    }

    // 构建叶子节点
    let leaves: Vec<MerkleNode> = items.iter()
        .map(|data| MerkleNode::leaf(data))
        .collect();

    // 自底向上构建树
    let mut current_level = leaves;
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        for chunk in current_level.chunks(2) {
            if chunk.len() == 2 {
                next_level.push(MerkleNode::combine(chunk[0].clone(), chunk[1].clone()));
            } else {
                next_level.push(chunk[0].clone());
            }
        }
        current_level = next_level;
    }

    current_level[0].hash.clone()
}

/// Merkle根校验结果
#[derive(Debug, Clone)]
pub enum MerkleCheckResult {
    Match,
    Mismatch { local_root: String, remote_root: String },
}