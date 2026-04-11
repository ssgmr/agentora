//! 交易系统

use crate::types::{AgentId, ResourceType};
use std::collections::HashMap;

/// 交易提议
#[derive(Debug, Clone)]
pub struct TradeOffer {
    pub proposer_id: AgentId,
    pub offer: HashMap<ResourceType, u32>,
    pub want: HashMap<ResourceType, u32>,
    pub trade_id: String,
}

impl crate::agent::Agent {
    /// 发起交易提议
    pub fn propose_trade(&self, target: AgentId, offer: HashMap<ResourceType, u32>, want: HashMap<ResourceType, u32>) -> TradeOffer {
        TradeOffer {
            proposer_id: self.id.clone(),
            offer,
            want,
            trade_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// 接受交易
    pub fn accept_trade(&mut self, trade: &TradeOffer) -> bool {
        // 检查自己是否有足够的wanted资源
        for (resource, amount) in &trade.want {
            let key = resource.as_str();
            let current = self.inventory.get(key).copied().unwrap_or(0);
            if current < *amount {
                return false;
            }
        }

        // 执行交易：给出want，获得offer
        for (resource, amount) in &trade.want {
            self.consume(*resource, *amount);
        }
        for (resource, amount) in &trade.offer {
            self.gather(*resource, *amount);
        }

        true
    }

    /// 拒绝交易（不改变背包）
    pub fn reject_trade(&self, _trade: &TradeOffer) {
        // 无操作
    }
}