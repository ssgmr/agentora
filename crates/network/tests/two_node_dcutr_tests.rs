//! 两节点 DCUtR 穿透集成测试

use agentora_network::{Libp2pTransport, ConnectionType, Transport};
use std::time::Duration;

#[tokio::test]
async fn test_dcutr_connection_flow() {
    // 创建两个节点
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();

    // 等待节点启动
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 验证节点有不同的 PeerId
    assert_ne!(node1.peer_id(), node2.peer_id());

    // 初始状态下没有直连记录
    let conn_type1: Option<ConnectionType> = node1.get_connection_type(node2.peer_id().0.as_str()).await;
    assert!(conn_type1.is_none());

    let conn_type2: Option<ConnectionType> = node2.get_connection_type(node1.peer_id().0.as_str()).await;
    assert!(conn_type2.is_none());

    // 注意：完整的 DCUtR 测试需要：
    // 1. 一个公网上可访问的中继节点
    // 2. 两个节点都在中继节点上注册
    // 3. 通过中继协调进行打洞
    // 这在本地测试环境中需要特殊配置
    // 此测试主要验证 DCUtR 基础设施正常工作
}

#[tokio::test]
async fn test_relay_reservation_flow() {
    // 创建节点
    let node = Libp2pTransport::new(0).unwrap();

    // 等待节点启动
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 验证初始 reservations 为空
    let reservations = node.get_relay_reservations().await;
    assert!(reservations.is_empty());

    // 注意：实际的中继 reservation 测试需要：
    // 1. 一个运行的中继节点
    // 2. 向中继节点发送 reservation 请求
    // 3. 验证 reservation 状态
    // 此测试验证 API 正常工作
}
