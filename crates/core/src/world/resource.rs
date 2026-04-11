//! 资源节点系统

use crate::types::{Position, ResourceType};
use serde::{Deserialize, Serialize};

/// 资源节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub position: Position,
    pub resource_type: ResourceType,
    pub current_amount: u32,
    pub max_amount: u32,
    pub regeneration_rate: u32,  // 每 tick 再生量
    pub regeneration_interval: u32,  // 再生间隔 tick
    pub is_depleted: bool,
}

impl ResourceNode {
    pub fn new(position: Position, resource_type: ResourceType, max_amount: u32) -> Self {
        Self {
            position,
            resource_type,
            current_amount: max_amount,
            max_amount,
            regeneration_rate: max_amount / 10,
            regeneration_interval: 20,
            is_depleted: false,
        }
    }

    /// 采集资源
    pub fn gather(&mut self, amount: u32) -> u32 {
        if self.is_depleted {
            return 0;
        }
        let gathered = self.current_amount.min(amount);
        self.current_amount -= gathered;
        if self.current_amount == 0 {
            self.is_depleted = true;
        }
        gathered
    }

    /// 资源再生
    pub fn regenerate(&mut self) {
        if self.is_depleted {
            self.current_amount += self.regeneration_rate;
            if self.current_amount >= self.max_amount {
                self.current_amount = self.max_amount;
                self.is_depleted = false;
            }
        }
    }
}