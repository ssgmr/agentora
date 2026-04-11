//! Transport抽象层

use async_trait::async_trait;
use crate::codec::NetworkMessage;
use agentora_sync::PeerId;

/// Transport抽象接口
#[async_trait]
pub trait Transport: Send + Sync {
    /// 发布消息到topic
    async fn publish(&self, topic: &str, data: &[u8]) -> Result<(), TransportError>;

    /// 订阅topic
    async fn subscribe(&self, topic: &str, handler: Box<dyn MessageHandler>) -> Result<(), TransportError>;

    /// 退订topic
    async fn unsubscribe(&self, topic: &str) -> Result<(), TransportError>;

    /// 获取本地PeerId
    fn peer_id(&self) -> &PeerId;

    /// 连接到种子节点
    async fn connect_to_seed(&self, addr: &str) -> Result<(), TransportError>;
}

/// 消息处理器
#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle(&self, message: NetworkMessage);
}

/// Transport错误
#[derive(Debug, Clone)]
pub enum TransportError {
    ConnectionFailed(String),
    PublishFailed(String),
    SubscribeFailed(String),
    UnsubscribeFailed(String),
    InvalidTopic(String),
}