//! Delta 分发器
//!
//! 同时发送到本地 mpsc 和 P2P GossipSub（可选）。

use std::sync::mpsc::Sender;
use crate::simulation::{AgentDelta, SimMode};

/// Delta 分发器：双通道分发（本地 + P2P）
pub struct DeltaDispatcher {
    /// 本地 mpsc 通道
    local_tx: Sender<AgentDelta>,
    /// 运行模式
    mode: SimMode,
}

impl DeltaDispatcher {
    /// 创建新的分发器
    pub fn new(local_tx: Sender<AgentDelta>, mode: SimMode) -> Self {
        Self { local_tx, mode }
    }

    /// 分发 Delta 到所有通道
    ///
    /// 集中式模式：只发送 local_tx
    /// P2P 模式：同时发送 local_tx 和 P2P GossipSub（待实现）
    pub fn dispatch(&self, delta: AgentDelta) {
        // 始终发送到本地通道（用于渲染）
        if let Err(e) = self.local_tx.send(delta) {
            tracing::error!("[DeltaDispatcher] local delta send failed: {:?}", e);
        }

        // P2P 广播（待 libp2p 消息处理链路完成后实现）
        if let SimMode::P2P { .. } = &self.mode {
            // TODO: 通过 GossipSub 广播到远程 peers
            // delta.for_broadcast() 返回精简 JSON
            tracing::trace!("[DeltaDispatcher] P2P broadcast pending (not yet implemented)");
        }
    }
}
