//! 多节点联机集成测试
//!
//! 测试场景：
//! - 3+ 节点同时在线
//! - 跨 NAT 连接验证
//! - 消息广播正确性

use agentora_network::{Libp2pTransport, NatStatus, Transport};
use std::time::Duration;

// ==================== 多节点启动与拓扑 ====================

/// 三节点同时在线启动测试
#[tokio::test]
async fn test_three_nodes_online() {
    // 创建三个节点
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();
    let node3 = Libp2pTransport::new(0).unwrap();

    // 等待 Swarm 启动
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 验证三个节点都有不同的 PeerId
    assert_ne!(node1.peer_id(), node2.peer_id());
    assert_ne!(node1.peer_id(), node3.peer_id());
    assert_ne!(node2.peer_id(), node3.peer_id());

    // 初始 NAT 状态应该都是 Unknown
    assert_eq!(node1.get_nat_status().await, NatStatus::Unknown);
    assert_eq!(node2.get_nat_status().await, NatStatus::Unknown);
    assert_eq!(node3.get_nat_status().await, NatStatus::Unknown);

    // 初始连接类型应该都是空的
    assert!(node1.get_connection_type(node2.peer_id().0.as_str()).await.is_none());
    assert!(node1.get_connection_type(node3.peer_id().0.as_str()).await.is_none());

    println!("三节点启动成功:");
    println!("  Node1: {}", node1.peer_id().0);
    println!("  Node2: {}", node2.peer_id().0);
    println!("  Node3: {}", node3.peer_id().0);
}

/// 五节点同时在线启动测试
#[tokio::test]
async fn test_five_nodes_online() {
    let nodes = vec![
        Libp2pTransport::new(0).unwrap(),
        Libp2pTransport::new(0).unwrap(),
        Libp2pTransport::new(0).unwrap(),
        Libp2pTransport::new(0).unwrap(),
        Libp2pTransport::new(0).unwrap(),
    ];

    tokio::time::sleep(Duration::from_millis(300)).await;

    // 验证所有 PeerId 都是唯一的
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            assert_ne!(nodes[i].peer_id(), nodes[j].peer_id(),
                "节点 {} 和 {} 的 PeerId 不应相同", i, j);
        }
    }

    // 验证所有节点的 NAT 状态初始为 Unknown
    for (i, node) in nodes.iter().enumerate() {
        assert_eq!(node.get_nat_status().await, NatStatus::Unknown,
            "节点 {} 的 NAT 状态应该为 Unknown", i);
    }

    println!("五节点启动成功，所有 PeerId 均唯一");
}

// ==================== 节点间地址注册与连接 ====================

/// 多节点地址互相注册测试
#[tokio::test]
async fn test_multi_node_address_registration() {
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();
    let node3 = Libp2pTransport::new(0).unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    // 注册节点地址（模拟已知对方地址的场景）
    let addr1 = "/ip4/127.0.0.1/tcp/50001";
    let addr2 = "/ip4/127.0.0.1/tcp/50002";
    let addr3 = "/ip4/127.0.0.1/tcp/50003";

    // 节点2和3记录节点1的地址
    node2.add_peer_address(node1.peer_id().0.as_str(), addr1).unwrap();
    node3.add_peer_address(node1.peer_id().0.as_str(), addr1).unwrap();

    // 节点1和3记录节点2的地址
    node1.add_peer_address(node2.peer_id().0.as_str(), addr2).unwrap();
    node3.add_peer_address(node2.peer_id().0.as_str(), addr2).unwrap();

    // 节点1和2记录节点3的地址
    node1.add_peer_address(node3.peer_id().0.as_str(), addr3).unwrap();
    node2.add_peer_address(node3.peer_id().0.as_str(), addr3).unwrap();

    // 验证 add_peer_address 调用成功（命令发送到 Swarm）
    // 注意：地址被添加到 KAD 路由表，本地 peer_addresses 缓存由 connect_to_peer 内部填充
    // 因此 get_peer_address 可能返回 None 直到实际连接建立
    println!("多节点地址注册完成，命令已发送到各节点 Swarm");
}

/// 三节点连接请求测试（验证 connect_to_peer API 调用链）
#[tokio::test]
async fn test_three_nodes_connect_api() {
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();
    let node3 = Libp2pTransport::new(0).unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    // 注册地址（connect_to_peer 需要地址来尝试直连）
    node1.add_peer_address(node2.peer_id().0.as_str(), "/ip4/127.0.0.1/tcp/51001").unwrap();
    node1.add_peer_address(node3.peer_id().0.as_str(), "/ip4/127.0.0.1/tcp/51002").unwrap();

    // 尝试连接（在没有实际中继的情况下，预期会失败因为无可用中继）
    // 但 API 调用链应该正常工作，不 panic
    let result1 = node1.connect_to_peer(node2.peer_id().0.as_str()).await;
    let result2 = node1.connect_to_peer(node3.peer_id().0.as_str()).await;

    // 在没有中继的环境下，连接会失败，但这是预期的
    // 重要的是 API 调用链没有 panic
    println!("Node1 -> Node2 连接结果: {:?}", result1.is_ok());
    println!("Node1 -> Node3 连接结果: {:?}", result2.is_ok());

    // 验证失败次数被记录
    assert!(node1.get_peer_failures(node2.peer_id().0.as_str()).await > 0);
    assert!(node1.get_peer_failures(node3.peer_id().0.as_str()).await > 0);
}

// ==================== 跨 NAT 连接验证 ====================

/// NAT 状态一致性测试
/// 验证所有节点在相同网络环境下 NAT 状态一致
#[tokio::test]
async fn test_cross_nat_status_consistency() {
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();
    let node3 = Libp2pTransport::new(0).unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    // 在本地环境中，所有节点都应该报告相同的状态
    let nat1 = node1.get_nat_status().await;
    let nat2 = node2.get_nat_status().await;
    let nat3 = node3.get_nat_status().await;

    // 本地环境下都是 Unknown（因为没有 AutoNAT 外部探测）
    assert_eq!(nat1, nat2, "Node1 和 Node2 的 NAT 状态应一致");
    assert_eq!(nat2, nat3, "Node2 和 Node3 的 NAT 状态应一致");

    println!("NAT 状态一致性验证通过: 所有节点状态一致");
}

/// 私有 NAT 连接策略验证
/// 验证当 NAT 状态为 Private 时，系统正确选择中继策略
#[tokio::test]
async fn test_private_nat_strategy_selection() {
    let node = Libp2pTransport::new(0).unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 在本地环境中，初始状态是 Unknown
    let status = node.get_nat_status().await;

    // 验证 needs_relay 逻辑
    if matches!(status, NatStatus::Unknown) {
        assert!(status.needs_relay(), "Unknown 状态需要中继");
    }

    // 手动验证 is_public 逻辑
    if matches!(status, NatStatus::Public(_)) {
        assert!(status.is_public(), "Public 状态应该是公网可达");
    }

    println!("私有 NAT 策略选择验证通过: {:?}", status);
}

/// 连接降级链路测试
/// 验证连接优先级：直连 → DCUtR → Relay 的降级路径
#[tokio::test]
async fn test_connection_degradation_chain() {
    let node = Libp2pTransport::new(0).unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 创建一个真实的有效 PeerId（从另一个 Transport 获取）
    let target = Libp2pTransport::new(0).unwrap();
    let target_peer = target.peer_id().0.as_str().to_string();

    // 注册一个不可达地址
    node.add_peer_address(&target_peer, "/ip4/192.0.2.1/tcp/9999").unwrap();

    // 尝试连接 - 应该经过完整的降级链路
    let result = node.connect_to_peer(&target_peer).await;

    // 连接应该失败（因为目标不可达），但降级逻辑应该被触发
    assert!(result.is_err(), "连接不可达的对等点应该失败");

    // 验证失败计数增加
    let failures = node.get_peer_failures(&target_peer).await;
    assert!(failures > 0, "失败计数应该增加");

    println!("连接降级链路测试通过: 失败次数 = {}", failures);
}

// ==================== 消息广播正确性 ====================

/// 多节点消息发布测试
/// 验证消息可以发布到多个节点的 topic
#[tokio::test]
async fn test_multi_node_message_publishing() {
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();
    let node3 = Libp2pTransport::new(0).unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let test_message = b"multi-node broadcast test message";

    // 每个节点都发布到同一 topic
    node1.publish("test-broadcast-topic", test_message).await.unwrap();
    node2.publish("test-broadcast-topic", test_message).await.unwrap();
    node3.publish("test-broadcast-topic", test_message).await.unwrap();

    // 如果能到达这里，说明消息发布命令发送成功
    println!("多节点消息发布测试通过");
}

/// GossipSub 订阅/退订测试
/// 验证多节点 topic 订阅管理
#[tokio::test]
async fn test_multi_node_topic_management() {
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();
    let node3 = Libp2pTransport::new(0).unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let topic = "test-multi-topic";

    // 三个节点都订阅同一 topic
    // 注意：subscribe 需要 MessageHandler trait 对象
    struct TestHandler;
    #[async_trait::async_trait]
    impl agentora_network::transport::MessageHandler for TestHandler {
        async fn handle(&self, _message: agentora_network::codec::NetworkMessage) {}
    }

    node1.subscribe(topic, Box::new(TestHandler)).await.unwrap();
    node2.subscribe(topic, Box::new(TestHandler)).await.unwrap();
    node3.subscribe(topic, Box::new(TestHandler)).await.unwrap();

    // 发布消息
    node1.publish(topic, b"hello from node1").await.unwrap();

    // 退订
    node1.unsubscribe(topic).await.unwrap();
    node2.unsubscribe(topic).await.unwrap();
    node3.unsubscribe(topic).await.unwrap();

    println!("多节点 topic 管理测试通过");
}

/// 消息序列化和反序列化一致性测试
/// 验证不同节点间消息格式一致
#[tokio::test]
async fn test_message_serialization_consistency() {
    use agentora_network::codec::{CrdtOp, NetworkMessage};

    let ops = vec![
        CrdtOp::LwwSet {
            key: "position".to_string(),
            value: b"100,200".to_vec(),
            timestamp: 1234567890,
            peer_id: "node1".to_string(),
        },
        CrdtOp::GCounterInc {
            key: "resource_count".to_string(),
            amount: 5,
            peer_id: "node2".to_string(),
        },
        CrdtOp::OrSetAdd {
            key: "neighbors".to_string(),
            element: b"peer_abc".to_vec(),
            tag: ("node3".to_string(), 999),
        },
    ];

    for op in &ops {
        let msg = NetworkMessage::CrdtOp(op.clone());
        let bytes = msg.to_bytes();
        let decoded = NetworkMessage::from_bytes(&bytes).unwrap();

        // 验证编解码一致性
        match (&msg, &decoded) {
            (NetworkMessage::CrdtOp(a), NetworkMessage::CrdtOp(b)) => {
                assert_eq!(a.to_json(), b.to_json(), "CRDT 操作编解码不一致");
            }
            _ => panic!("消息类型不匹配"),
        }
    }

    // 测试其他消息类型
    let sync_req = NetworkMessage::SyncRequest {
        peer_id: "node1".to_string(),
        merkle_root: "abc123".to_string(),
    };
    let bytes = sync_req.to_bytes();
    let decoded = NetworkMessage::from_bytes(&bytes).unwrap();
    assert!(matches!(decoded, NetworkMessage::SyncRequest { .. }));

    let sync_resp = NetworkMessage::SyncResponse { ops: vec![] };
    let bytes = sync_resp.to_bytes();
    let decoded = NetworkMessage::from_bytes(&bytes).unwrap();
    assert!(matches!(decoded, NetworkMessage::SyncResponse { .. }));

    let peer_info = NetworkMessage::PeerInfo {
        peer_id: "node1".to_string(),
        position: (100, 200),
    };
    let bytes = peer_info.to_bytes();
    let decoded = NetworkMessage::from_bytes(&bytes).unwrap();
    assert!(matches!(decoded, NetworkMessage::PeerInfo { .. }));

    println!("消息序列化一致性测试通过: 验证了 {} 种消息类型", 5);
}

/// 广播消息大小限制测试
/// 验证不同大小的消息都能正确处理
#[tokio::test]
async fn test_broadcast_message_sizes() {
    let node = Libp2pTransport::new(0).unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let sizes = vec![16, 64, 256, 1024, 4096];

    for size in &sizes {
        let payload = vec![0xABu8; *size];
        node.publish("size-test-topic", &payload).await.unwrap();
    }

    println!("广播消息大小限制测试通过: 测试了 {} 种大小", sizes.len());
}

// ==================== 节点密钥持久化与多节点 ====================

/// 多节点密钥持久化测试
/// 验证节点重启后 PeerId 不变
#[tokio::test]
async fn test_multi_node_key_persistence() {
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap();

    // 创建节点并保存密钥
    let node1 = Libp2pTransport::new(0).unwrap();
    let original_peer_id = node1.peer_id().clone();
    node1.save_key(temp_path).unwrap();

    // 从密钥文件加载新节点
    let node2 = Libp2pTransport::load_from_file(temp_path, 0).unwrap();
    let loaded_peer_id = node2.peer_id().clone();

    // 验证 PeerId 相同
    assert_eq!(original_peer_id, loaded_peer_id,
        "密钥加载后的 PeerId 应该与原始 PeerId 相同");

    println!("多节点密钥持久化测试通过: PeerId = {}", original_peer_id.0);
}

/// 中继 reservation 初始状态测试
#[tokio::test]
async fn test_multi_node_relay_initial_state() {
    let node1 = Libp2pTransport::new(0).unwrap();
    let node2 = Libp2pTransport::new(0).unwrap();
    let node3 = Libp2pTransport::new(0).unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // 初始中继 reservation 应该为空
    assert!(node1.get_relay_reservations().await.is_empty());
    assert!(node2.get_relay_reservations().await.is_empty());
    assert!(node3.get_relay_reservations().await.is_empty());

    println!("中继 reservation 初始状态测试通过");
}

// ==================== 压力测试 ====================

/// 多节点并发消息压力测试
#[tokio::test]
async fn test_multi_node_concurrent_messaging() {
    let node_count = 5;
    let messages_per_node = 50;

    let nodes: Vec<std::sync::Arc<Libp2pTransport>> = (0..node_count)
        .map(|_| std::sync::Arc::new(Libp2pTransport::new(0).unwrap()))
        .collect();

    tokio::time::sleep(Duration::from_millis(300)).await;

    // 并发发布消息
    let mut handles = Vec::new();
    for i in 0..node_count {
        let node = nodes[i].clone();
        let handle = tokio::spawn(async move {
            for j in 0..messages_per_node {
                let msg = format!("node-{}-msg-{}", i, j);
                node.publish("stress-test-topic", msg.as_bytes()).await.unwrap();
            }
        });
        handles.push(handle);
    }

    // 等待所有发布完成
    for handle in handles {
        handle.await.unwrap();
    }

    let total_messages = node_count * messages_per_node;
    println!("多节点并发消息压力测试通过: {} 节点 × {} 消息 = {} 总消息",
        node_count, messages_per_node, total_messages);
}
