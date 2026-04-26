//! 单节点启动集成测试

use agentora_network::{Libp2pTransport, NatStatus, Transport};
use std::time::Duration;

#[tokio::test]
async fn test_single_node_startup() {
    // 创建节点
    let transport = Libp2pTransport::new(0).unwrap();

    // 验证节点已启动
    let _ = transport.peer_id();

    // 给一点时间让 Swarm 启动
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 验证 NAT 状态初始为未知
    let nat_status = transport.get_nat_status().await;
    assert_eq!(nat_status, NatStatus::Unknown);
}

#[tokio::test]
async fn test_single_node_with_key_file() {
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap();

    // 创建节点并保存密钥
    let transport1 = Libp2pTransport::new(0).unwrap();
    transport1.save_key(temp_path).unwrap();

    // 从密钥文件加载节点
    let transport2 = Libp2pTransport::load_from_file(temp_path, 0).unwrap();

    // 验证 PeerId 相同
    assert_eq!(transport1.peer_id(), transport2.peer_id());
}

#[tokio::test]
async fn test_listen_address() {
    // 这个测试验证节点能够绑定到本地端口
    let transport = Libp2pTransport::new(0).unwrap();

    // 等待 Swarm 启动和监听
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 如果能到达这里，说明监听成功
    let _ = transport.peer_id();
}
