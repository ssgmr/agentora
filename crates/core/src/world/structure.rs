//! 结构与建筑系统

use crate::types::{Position, StructureType, AgentId};
use serde::{Deserialize, Serialize};

/// 结构/建筑
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Structure {
    pub position: Position,
    pub structure_type: StructureType,
    pub owner_id: Option<AgentId>,
    pub durability: u32,
    pub max_durability: u32,
    pub created_tick: u64,
}

impl Structure {
    pub fn new(position: Position, structure_type: StructureType, owner: Option<AgentId>, tick: u64) -> Self {
        Self {
            position,
            structure_type,
            owner_id: owner,
            durability: 100,
            max_durability: 100,
            created_tick: tick,
        }
    }

    /// 消耗耐久
    pub fn damage(&mut self, amount: u32) {
        self.durability = self.durability.saturating_sub(amount);
    }

    /// 检查是否损坏
    pub fn is_destroyed(&self) -> bool {
        self.durability == 0
    }
}