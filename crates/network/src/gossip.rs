//! GossipSub 区域 topic 管理器

use crate::libp2p_transport::Libp2pTransport;
use crate::transport::{Transport, TransportError};

/// 区域 topic 管理器
pub struct RegionTopicManager {
    current_region: u32,
    subscribed_regions: Vec<u32>,
}

impl RegionTopicManager {
    pub fn new() -> Self {
        Self {
            current_region: 0,
            subscribed_regions: vec![],
        }
    }

    /// 获取区域 topic 名称
    pub fn topic_name(region_id: u32) -> String {
        format!("region_{}", region_id)
    }

    /// 更新当前区域，自动订阅/退订
    pub async fn update_region(
        &mut self,
        new_region: u32,
        transport: &Libp2pTransport,
    ) -> Result<(), TransportError> {
        if new_region == self.current_region {
            return Ok(());
        }

        // 计算需要订阅的区域（当前 + 邻区）
        let neighbors = get_neighbor_regions(new_region);
        let to_subscribe: Vec<u32> = neighbors
            .iter()
            .filter(|r| !self.subscribed_regions.contains(r))
            .copied()
            .collect();

        // 计算需要退订的区域
        let to_unsubscribe: Vec<u32> = self
            .subscribed_regions
            .iter()
            .filter(|r| !neighbors.contains(r))
            .copied()
            .collect();

        // 执行订阅
        for region in to_subscribe {
            let topic = Self::topic_name(region);
            // 注意：实际的订阅需要在 Transport 中实现消息处理器传递
            // 这里简化处理，订阅时不传递处理器
            transport.subscribe(&topic, Box::new(NullMessageHandler)).await?;
            self.subscribed_regions.push(region);
            tracing::info!("订阅区域 topic: {}", topic);
        }

        // 执行退订
        for region in to_unsubscribe {
            let topic = Self::topic_name(region);
            transport.unsubscribe(&topic).await?;
            self.subscribed_regions.retain(|&r| r != region);
            tracing::info!("退订区域 topic: {}", topic);
        }

        self.current_region = new_region;
        Ok(())
    }

    /// 获取当前区域
    pub fn current_region(&self) -> u32 {
        self.current_region
    }

    /// 获取已订阅的区域列表
    pub fn subscribed_regions(&self) -> &[u32] {
        &self.subscribed_regions
    }
}

impl Default for RegionTopicManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取邻近区域 ID
fn get_neighbor_regions(region_id: u32) -> Vec<u32> {
    // 简单实现：当前区域 + 上下左右四个邻区
    // 假设地图是 1000x1000 的网格
    let row = region_id / 1000;
    let col = region_id % 1000;

    let mut neighbors = vec![region_id];

    // 右
    if col + 1 < 1000 {
        neighbors.push(row * 1000 + (col + 1));
    }
    // 左
    if col > 0 {
        neighbors.push(row * 1000 + (col - 1));
    }
    // 下
    if row + 1 < 1000 {
        neighbors.push((row + 1) * 1000 + col);
    }
    // 上
    if row > 0 {
        neighbors.push((row - 1) * 1000 + col);
    }

    neighbors
}

/// 空消息处理器（用于订阅时占位）
struct NullMessageHandler;

#[async_trait::async_trait]
impl crate::transport::MessageHandler for NullMessageHandler {
    async fn handle(&self, _message: crate::codec::NetworkMessage) {
        // 空实现，实际消息处理由上层负责
    }
}
