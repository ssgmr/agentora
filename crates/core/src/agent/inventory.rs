//! 采集与背包系统

use crate::types::ResourceType;
use std::sync::OnceLock;

/// 背包配置（全局单例，由 bridge 初始化时设置）
#[derive(Debug, Clone)]
pub struct InventoryConfig {
    pub max_slots: usize,
    pub max_stack_size: u32,
    pub warehouse_limit_multiplier: u32,
}

static INVENTORY_CONFIG: OnceLock<InventoryConfig> = OnceLock::new();

impl Default for InventoryConfig {
    fn default() -> Self {
        Self {
            max_slots: 20,
            max_stack_size: 20,
            warehouse_limit_multiplier: 2,
        }
    }
}

/// 初始化背包配置（由 bridge 在启动时调用一次）
pub fn init_inventory_config(config: InventoryConfig) {
    let _ = INVENTORY_CONFIG.set(config);
}

pub fn get_config() -> &'static InventoryConfig {
    INVENTORY_CONFIG.get_or_init(InventoryConfig::default)
}

impl crate::agent::Agent {
    /// 采集资源
    pub fn gather(&mut self, resource: ResourceType, amount: u32) -> bool {
        let key = resource.as_str();
        let current = self.inventory.get(key).copied().unwrap_or(0);
        let max_stack = get_config().max_stack_size;

        if current + amount > max_stack {
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
        self.inventory_count() >= get_config().max_slots
    }
}
