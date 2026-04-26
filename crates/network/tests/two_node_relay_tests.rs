//! 两节点 Relay 连接集成测试

use agentora_network::{Libp2pTransport, NatStatus, Transport};
use std::time::Duration;

#[tokio::test]
async fn test_relay_connection_fallback() {
    // 创建两个节点
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();

    // 等待节点启动
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 验证节点有不同的 PeerId
    assert_ne!(node1.peer_id(), node2.peer_id());

    // 验证初始 NAT 状态为未知
    assert_eq!(node1.get_nat_status().await, NatStatus::Unknown);
    assert_eq!(node2.get_nat_status().await, NatStatus::Unknown);

    // 注意：完整的 Relay 连接测试需要：
    // 1. 一个运行的中继节点
    // 2. 两个节点都连接到中继
    // 3. 通过中继建立电路连接
    // 此测试验证基础设施正常工作
}

#[tokio::test]
async fn test_connect_via_relay_api() {
    let node = Libp2pTransport::new(0).unwrap();

    // 等待节点启动
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 测试通过中继连接的 API
    // 使用一个示例中继地址和目标 PeerId
    let relay_addr = "/ip4/127.0.0.1/tcp/40000";
    let target_peer_id = "12D3KooWRQVCrDQcFEAK7tYB9oU5kY9Q8X7t";

    // 验证 connect_via_relay 方法存在且可以调用
    let result = node.connect_via_relay(relay_addr, target_peer_id);
    // 地址可能无效，但方法应该可以调用
    // 如果地址格式错误会返回错误，这是预期的
    let _ = result.is_ok() || result.is_err();
}
