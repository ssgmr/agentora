//! 交易系统

use crate::types::{AgentId, ResourceType};
use std::collections::HashMap;

/// 交易提议（供 World 使用）
#[derive(Debug, Clone)]
pub struct TradeOffer {
    pub proposer_id: AgentId,
    pub offer: HashMap<ResourceType, u32>,
    pub want: HashMap<ResourceType, u32>,
    pub trade_id: String,
}

impl crate::agent::Agent {
    /// 冻结资源：发起交易时，offer资源移到frozen_inventory
    /// 返回是否成功（资源不足时失败）
    pub fn freeze_resources(&mut self, offer: HashMap<ResourceType, u32>, trade_id: &str) -> bool {
        // 检查资源足够
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let current = self.inventory.get(key).copied().unwrap_or(0);
            if current < *amount {
                return false;
            }
        }
        // 冻结：从inventory移到frozen_inventory
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let current = self.inventory.get(key).copied().unwrap_or(0);
            if current == *amount {
                self.inventory.remove(key);
            } else {
                self.inventory.insert(key.to_string(), current - amount);
            }
            let frozen = self.frozen_inventory.get(key).copied().unwrap_or(0);
            self.frozen_inventory.insert(key.to_string(), frozen + amount);
        }
        self.pending_trade_id = Some(trade_id.to_string());
        true
    }

    /// 完成交易发送方：解冻并实际扣减offer，接收want
    pub fn complete_trade_send(&mut self, offer: HashMap<ResourceType, u32>, want: HashMap<ResourceType, u32>) {
        // offer从frozen移除（实际扣减）
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let frozen = self.frozen_inventory.get(key).copied().unwrap_or(0);
            if frozen == *amount {
                self.frozen_inventory.remove(key);
            } else {
                self.frozen_inventory.insert(key.to_string(), frozen - amount);
            }
        }
        // want加入inventory
        for (resource, amount) in want {
            self.gather(resource, amount);
        }
        self.pending_trade_id = None;
    }

    /// 取消交易：解冻资源回到inventory
    pub fn cancel_trade(&mut self, offer: HashMap<ResourceType, u32>) {
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let frozen = self.frozen_inventory.get(key).copied().unwrap_or(0);
            if frozen == *amount {
                self.frozen_inventory.remove(key);
            } else {
                self.frozen_inventory.insert(key.to_string(), frozen - amount);
            }
            let current = self.inventory.get(key).copied().unwrap_or(0);
            self.inventory.insert(key.to_string(), current + amount);
        }
        self.pending_trade_id = None;
    }

    /// 接收方交出want资源
    pub fn give_resources(&mut self, want: HashMap<ResourceType, u32>) -> bool {
        for (resource, amount) in &want {
            if !self.consume(*resource, *amount) {
                return false;
            }
        }
        true
    }

    /// 接收方获得offer资源
    pub fn receive_resources(&mut self, offer: HashMap<ResourceType, u32>) {
        for (resource, amount) in offer {
            self.gather(resource, amount);
        }
    }
}