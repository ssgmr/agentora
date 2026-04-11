//! 集成测试 - 两节点P2P联机
//!
//! 测试事件广播和CRDT同步

// TODO: 实现完整集成测试
// 需要:
// 1. 启动两个节点
// 2. 各节点运行2-3个Agent
// 3. 验证30秒内建立连接
// 4. 验证事件正确广播
// 5. 验证Agent跨节点可见

#[cfg(test)]
mod tests {
    // #[test]
    // fn test_two_node_p2p_connection() {
    //     // 启动节点1
    //     let node1 = agentora_network::libp2p_transport::Libp2pTransport::new();
    //
    //     // 启动节点2
    //     let node2 = agentora_network::libp2p_transport::Libp2pTransport::new();
    //
    //     // 连接
    //     node2.connect_to_seed(node1.peer_id().0).await;
    //
    //     // 验证连接
    //     assert!(connected(node1, node2));
    // }
    //
    // #[test]
    // fn test_event_broadcast() {
    //     // 发布事件
    //     let op = agentora_sync::codec::CrdtOp::LwwSet {...};
    //     node1.publish("region_0", &op.to_json()).await;
    //
    //     // 等待接收
    //     sleep(1);
    //
    //     // 验证节点2收到
    //     assert!(node2.state.contains(op));
    // }
}