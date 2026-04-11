//! 采集与背包系统

use crate::types::ResourceType;
use std::collections::HashMap;

const MAX_INVENTORY_SLOTS: usize = 20;
const MAX_STACK_SIZE: u32 = 99;

impl crate::agent::Agent {
    /// 采集资源
    pub fn gather(&mut self, resource: ResourceType, amount: u32) -> bool {
        let key = resource.as_str();
        let current = self.inventory.get(key).copied().unwrap_or(0);

        if current + amount > MAX_STACK_SIZE {
            // 超出堆叠上限
            return false;
        }

        self.inventory.insert(key.to_string(), current + amount);
        true
    }

    /// 消耗资源
    pub fn consume(&mut self, resource: ResourceType, amount: u32) -> bool {
        let key = resource.as_str();
        let current = self.inventory.get(key).copied().unwrap_or(0);

        if current < amount {
            return false;
        }

        if current == amount {
            self.inventory.remove(key);
        } else {
            self.inventory.insert(key.to_string(), current - amount);
        }
        true
    }

    /// 获取背包总数
    pub fn inventory_count(&self) -> usize {
        self.inventory.len()
    }

    /// 判断背包是否已满
    pub fn is_inventory_full(&self) -> bool {
        self.inventory_count() >= MAX_INVENTORY_SLOTS
    }
}