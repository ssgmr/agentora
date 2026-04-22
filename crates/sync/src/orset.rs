//! OR-Set实现
//!
//! Observed-Remove Set，添加优先于未观察删除

use serde::{Deserialize, Serialize};
use crate::types::PeerId;
use std::collections::HashSet;
use std::hash::Hash;

/// OR-Set元素标记
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ElementTag {
    pub peer_id: String,
    pub counter: u64,
}

/// OR-Set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrSet<T: Clone + Hash + Eq> {
    elements: HashSet<(T, ElementTag)>,
    tombstones: HashSet<ElementTag>,
}

impl<T: Clone + Hash + Eq + Serialize + for<'de> Deserialize<'de>> OrSet<T> {
    pub fn new() -> Self {
        Self {
            elements: HashSet::new(),
            tombstones: HashSet::new(),
        }
    }

    /// 添加元素
    pub fn add(&mut self, element: T, peer_id: &PeerId, counter: u64) {
        let tag = ElementTag {
            peer_id: peer_id.0.clone(),
            counter,
        };
        // 仅添加未被标记删除的元素
        if !self.tombstones.contains(&tag) {
            self.elements.insert((element, tag));
        }
    }

    /// 删除元素（需要先观察）
    pub fn remove(&mut self, element: &T) {
        // 找到所有匹配的元素，标记删除
        let to_remove: Vec<ElementTag> = self.elements.iter()
            .filter(|(e, _)| e == element)
            .map(|(_, tag)| tag.clone())
            .collect();

        for tag in to_remove {
            self.tombstones.insert(tag.clone());
            self.elements.retain(|(_, t)| t != &tag);
        }
    }

    /// 使用 tag 删除元素（用于 OrSetRemove CRDT 操作）
    pub fn remove_with_tag(&mut self, peer_id: &PeerId, counter: u64) {
        let tag = ElementTag {
            peer_id: peer_id.0.clone(),
            counter,
        };
        // 添加到 tombstones
        self.tombstones.insert(tag.clone());
        // 从 elements 中移除
        self.elements.retain(|(_, t)| t != &tag);
    }

    /// 检查元素是否存在
    pub fn contains(&self, element: &T) -> bool {
        self.elements.iter().any(|(e, _)| e == element)
    }

    /// 获取所有元素
    pub fn elements(&self) -> Vec<T> {
        self.elements.iter().map(|(e, _)| e.clone()).collect()
    }

    /// 合合两个OR-Set
    pub fn merge(&mut self, other: &OrSet<T>) {
        // 合并tombstones
        for tag in &other.tombstones {
            self.tombstones.insert(tag.clone());
        }

        // 移除被tombstones标记的元素
        self.elements.retain(|(_, tag)| !self.tombstones.contains(tag));

        // 合并elements（添加优先）
        for (element, tag) in &other.elements {
            if !self.tombstones.contains(tag) {
                self.elements.insert((element.clone(), tag.clone()));
            }
        }
    }
}

impl<T: Clone + Hash + Eq + Serialize + for<'de> Deserialize<'de>> Default for OrSet<T> {
    fn default() -> Self {
        Self::new()
    }
}