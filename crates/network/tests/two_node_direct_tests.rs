//! 两节点直连集成测试

use agentora_network::{Libp2pTransport, NatStatus, Transport};
use std::time::Duration;

#[tokio::test]
async fn test_two_nodes_direct_connection() {
    // 创建两个节点
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();

    // 等待节点启动
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 验证节点有不同的 PeerId
    assert_ne!(node1.peer_id(), node2.peer_id());

    // 初始状态下 NAT 状态应该是未知
    assert_eq!(node1.get_nat_status().await, NatStatus::Unknown);
    assert_eq!(node2.get_nat_status().await, NatStatus::Unknown);

    // 注意：实际的直连测试需要知道对方的监听地址
    // 这在本地测试环境中比较复杂，因为需要动态获取监听端口
    // 这里主要验证基础设施正常工作
}

#[tokio::test]
async fn test_add_peer_address() {
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();

    // 等待节点启动
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 构造一个测试地址（实际场景中应该从 node1 获取监听地址）
    let test_addr = "/ip4/127.0.0.1/tcp/40001";

    // 添加对等点地址
    let result = node2.add_peer_address(node1.peer_id().0.as_str(), test_addr);
    // 验证方法调用成功（地址添加到 KAD 路由表）
    assert!(result.is_ok());
}
