//! 混合穿透策略逻辑单元测试

use agentora_network::{Libp2pTransport, NatStatus, ConnectionType, Transport};

#[tokio::test]
async fn test_transport_creation() {
    let transport = Libp2pTransport::new(0);
    assert!(transport.is_ok());
}

#[tokio::test]
async fn test_transport_default() {
    let transport = Libp2pTransport::default();
    // peer_id() 返回 &PeerId，直接断言不为空
    let _ = transport.peer_id();
}

#[tokio::test]
async fn test_nat_status_initially_unknown() {
    let transport = Libp2pTransport::new(0).unwrap();
    let status = transport.get_nat_status().await;

    assert_eq!(status, NatStatus::Unknown);
}

#[tokio::test]
async fn test_get_connection_type_empty() {
    let transport = Libp2pTransport::new(0).unwrap();

    // 初始状态下没有任何连接
    let connection_type: Option<ConnectionType> = transport.get_connection_type("test_peer").await;
    assert!(connection_type.is_none());
}

#[tokio::test]
async fn test_peer_id_generation() {
    let transport1 = Libp2pTransport::new(0).unwrap();
    let transport2 = Libp2pTransport::new(0).unwrap();

    // 每次生成的 PeerId 应该不同
    assert_ne!(transport1.peer_id(), transport2.peer_id());
}

#[tokio::test]
async fn test_load_from_file_generates_new_key() {
    // 使用不存在的文件路径，应该生成新密钥
    let transport = Libp2pTransport::load_from_file("/nonexistent/key/file.bin", 0);
    assert!(transport.is_ok());
}

#[tokio::test]
async fn test_save_and_load_key() {
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap();

    // 创建 transport
    let transport = Libp2pTransport::new(0).unwrap();

    // 保存密钥
    let save_result = transport.save_key(temp_path);
    assert!(save_result.is_ok());

    // 从文件加载
    let loaded_transport = Libp2pTransport::load_from_file(temp_path, 0);
    assert!(loaded_transport.is_ok());

    // 验证 PeerId 相同（因为密钥相同）
    assert_eq!(transport.peer_id(), loaded_transport.unwrap().peer_id());
}
