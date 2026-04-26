//! libp2p Transport 实现
//!
//! rust-libp2p 集成：GossipSub + KAD DHT
//! 类型定义引用自独立模块：behaviour.rs、nat.rs、config.rs、swarm.rs

use async_trait::async_trait;
use crate::transport::{Transport, MessageHandler, TransportError};
use crate::codec::NetworkMessage;
use crate::swarm::{run_swarm_event_loop, SwarmCommand};
use crate::nat::{NatStatus, ConnectionType};
use crate::config::{HybridStrategyConfig, RelayReservation};
use agentora_sync::PeerId;
use libp2p::{
    identity,
    Multiaddr,
};
use std::sync::Arc;
use tokio::sync::mpsc;

/// libp2p Transport 实现
pub struct Libp2pTransport {
    peer_id: PeerId,
    #[allow(dead_code)]
    local_key: identity::Keypair,
    swarm_tx: mpsc::Sender<SwarmCommand>,
    /// 消息接收通道（供上层消费 GossipSub 消息）
    /// 使用 Option 包装，允许多次 take
    message_rx: Option<mpsc::Receiver<NetworkMessage>>,
    /// 中继 reservation 状态
    relay_reservations: Arc<tokio::sync::RwLock<Vec<RelayReservation>>>,
    /// NAT 状态（使用 AutoNAT 探测结果）
    nat_status: Arc<tokio::sync::RwLock<NatStatus>>,
    /// 直连成功的对等点列表
    direct_connections: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    /// 混合穿透策略配置
    config: HybridStrategyConfig,
    /// 对等点连接失败次数统计（用于降级决策）
    peer_failures: Arc<tokio::sync::RwLock<std::collections::HashMap<String, u32>>>,
    /// 对等点地址缓存
    peer_addresses: Arc<tokio::sync::RwLock<std::collections::HashMap<String, Multiaddr>>>,
}

impl Clone for Libp2pTransport {
    fn clone(&self) -> Self {
        Self {
            peer_id: self.peer_id.clone(),
            local_key: identity::Keypair::generate_ed25519(), // clone 时生成新 key（仅用于 publish，不需要原始 key）
            swarm_tx: self.swarm_tx.clone(),
            message_rx: None, // receiver 不 clone
            relay_reservations: self.relay_reservations.clone(),
            nat_status: self.nat_status.clone(),
            direct_connections: self.direct_connections.clone(),
            config: self.config.clone(),
            peer_failures: self.peer_failures.clone(),
            peer_addresses: self.peer_addresses.clone(),
        }
    }
}

impl Libp2pTransport {
    /// 创建新的 P2P 传输层，使用随机密钥和指定端口
    pub fn new(listen_port: u16) -> Result<Self, TransportError> {
        // 生成 ed25519 密钥
        let local_key = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::new(local_key.public().to_peer_id().to_string());

        tracing::info!("生成 PeerId: {}", peer_id.0);

        Self::with_key(local_key, peer_id, listen_port)
    }

    /// 使用指定密钥创建
    fn with_key(local_key: identity::Keypair, peer_id: PeerId, listen_port: u16) -> Result<Self, TransportError> {
        // 创建通道用于发送命令到 Swarm
        let (swarm_tx, swarm_rx) = mpsc::channel::<SwarmCommand>(100);

        // 创建消息接收通道（供上层消费）
        let (message_tx, message_rx) = mpsc::channel::<NetworkMessage>(100);

        // 创建 relay reservations 共享状态
        let relay_reservations = Arc::new(tokio::sync::RwLock::new(Vec::new()));

        // 创建 NAT 状态共享状态
        let nat_status = Arc::new(tokio::sync::RwLock::new(NatStatus::Unknown));

        // 创建直连连接共享状态
        let direct_connections = Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new()));

        // 创建对等点失败次数统计
        let peer_failures = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        // 创建对等点地址缓存
        let peer_addresses = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        // 启动 Swarm 事件循环（使用 swarm.rs 中的函数）
        let key_clone = local_key.clone();
        let peer_id_clone = peer_id.clone();
        let reservations_clone = relay_reservations.clone();
        let nat_status_clone = nat_status.clone();
        let direct_connections_clone = direct_connections.clone();
        tokio::spawn(async move {
            run_swarm_event_loop(
                key_clone,
                peer_id_clone,
                listen_port,
                swarm_rx,
                message_tx,
                reservations_clone,
                nat_status_clone,
                direct_connections_clone,
                Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())), // topic_handlers（暂不使用）
            ).await;
        });

        Ok(Self {
            peer_id,
            local_key,
            swarm_tx,
            message_rx: Some(message_rx),
            relay_reservations,
            nat_status,
            direct_connections,
            config: HybridStrategyConfig::default(),
            peer_failures,
            peer_addresses,
        })
    }

    /// 从现有密钥加载
    pub fn load_from_file(key_path: &str, listen_port: u16) -> Result<Self, TransportError> {
        // 尝试从文件加载密钥
        let local_key = Self::load_key_from_file(key_path).unwrap_or_else(|_| {
            tracing::info!("密钥文件不存在，生成新密钥");
            identity::Keypair::generate_ed25519()
        });

        let libp2p_peer_id = local_key.public().to_peer_id();
        let peer_id = PeerId::new(libp2p_peer_id.to_string());

        tracing::info!("加载 PeerId: {}", peer_id.0);

        Self::with_key(local_key, peer_id, listen_port)
    }

    /// 保存密钥到文件
    pub fn save_key(&self, key_path: &str) -> Result<(), TransportError> {
        self.save_key_to_file(key_path)
    }

    /// 从文件加载密钥
    fn load_key_from_file(path: &str) -> Result<identity::Keypair, std::io::Error> {
        use std::fs;

        let key_bytes = fs::read(path)?;
        identity::Keypair::from_protobuf_encoding(&key_bytes)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid key format"))
    }

    /// 保存密钥到文件
    fn save_key_to_file(&self, path: &str) -> Result<(), TransportError> {
        use std::fs;

        let key_bytes = self.local_key.to_protobuf_encoding().map_err(|e| {
            TransportError::PublishFailed(format!("密钥编码失败：{}", e))
        })?;
        fs::write(path, key_bytes)
            .map_err(|e| TransportError::PublishFailed(format!("保存密钥失败：{}", e)))
    }

    /// 获取 NAT 状态
    pub async fn get_nat_status(&self) -> NatStatus {
        self.nat_status.read().await.clone()
    }

    /// 获取消息接收通道（供上层消费 GossipSub 消息）
    ///
    /// 注意：由于 Receiver 不能 clone，此方法只能调用一次。
    /// 调用后 message_rx 字段变为 None，后续调用返回 None。
    pub fn take_message_receiver(&mut self) -> Option<mpsc::Receiver<NetworkMessage>> {
        self.message_rx.take()
    }

    /// 尝试接收一条消息（非阻塞，仅当 receiver 存在时）
    pub fn try_recv_message(&mut self) -> Option<NetworkMessage> {
        if let Some(ref mut rx) = self.message_rx {
            rx.try_recv().ok()
        } else {
            None
        }
    }

    /// 获取连接类型
    pub async fn get_connection_type(&self, peer_id: &str) -> Option<ConnectionType> {
        if self.direct_connections.read().await.contains(peer_id) {
            Some(ConnectionType::Direct)
        } else {
            // TODO: 跟踪 DCUtR 和 Relay 连接状态
            None
        }
    }

    /// 智能连接对等点（混合穿透策略）
    ///
    /// 连接优先级：直连 → DCUtR → Relay
    pub async fn connect_to_peer(&self, peer_id: &str) -> Result<ConnectionType, TransportError> {
        let peer_id_parsed: libp2p::PeerId = peer_id.parse()
            .map_err(|e| TransportError::ConnectionFailed(format!("无效 PeerId: {}", e)))?;

        // 1. 检查 NAT 类型，如果是对称型 NAT 直接使用 Relay
        let nat_status = self.get_nat_status().await;
        if matches!(nat_status, NatStatus::Private) {
            tracing::warn!("NAT 类型为私有网络，尝试直接使用 Relay");
        }

        // 2. 尝试获取对等点地址
        let peer_addr = self.get_peer_address(peer_id).await;

        // 3. 检查失败次数，决定是否降级
        let failure_count = self.get_peer_failures(peer_id).await;
        let should_skip_direct = failure_count >= self.config.degradation_threshold;

        // 4. 尝试直连（如果有地址且未超过失败阈值）
        if !should_skip_direct {
            if let Some(addr) = peer_addr.as_ref() {
                tracing::info!("尝试直连对等点：{} -> {}", peer_id, addr);
                if self.try_direct_connect(peer_id_parsed, addr.clone()).await {
                    self.record_peer_failure(peer_id, 0).await; // 重置失败计数
                    self.add_direct_connection(peer_id).await;
                    return Ok(ConnectionType::Direct);
                }
            }
        }

        // 5. 尝试 DCUtR 打洞（如果启用）
        if self.config.enable_dcutr {
            tracing::info!("尝试 DCUtR 打洞连接：{}", peer_id);
            if let Some(relay) = self.find_available_relay().await {
                if self.try_dcutr_connect(peer_id_parsed, relay).await {
                    self.record_peer_failure(peer_id, 0).await; // 重置失败计数
                    return Ok(ConnectionType::Dcutr);
                }
            }
        } else {
            tracing::debug!("DCUtR 已禁用，跳过打洞");
        }

        // 6. 降级到 Relay（保底）
        tracing::warn!("DCUtR 失败，降级到 Relay：{}", peer_id);
        if let Some(relay) = self.find_available_relay().await {
            let result = self.connect_via_relay_to_peer(&relay.relay_addr, peer_id);
            match result {
                Ok(()) => {
                    self.record_peer_failure(peer_id, 0).await;
                    return Ok(ConnectionType::Relay);
                }
                Err(e) => {
                    self.record_peer_failure(peer_id, 1).await;
                    return Err(TransportError::ConnectionFailed(
                        format!("无法连接到对等点 {}，中继也失败：{:?}", peer_id, e)
                    ));
                }
            }
        }
        self.record_peer_failure(peer_id, 1).await;
        Err(TransportError::ConnectionFailed(
            format!("无法连接到对等点 {}，没有可用的中继节点", peer_id)
        ))
    }

    /// 尝试直连
    async fn try_direct_connect(&self, _peer_id: libp2p::PeerId, addr: Multiaddr) -> bool {
        let cmd = SwarmCommand::DialDirect { addr };
        self.swarm_tx.try_send(cmd).is_ok()
    }

    /// 尝试 DCUtR 打洞连接
    async fn try_dcutr_connect(&self, peer_id: libp2p::PeerId, relay: RelayReservation) -> bool {
        let relay_addr: Multiaddr = match relay.relay_addr.parse() {
            Ok(addr) => addr,
            Err(_) => return false,
        };

        let cmd = SwarmCommand::DialViaDcutr { peer_id, relay_addr };
        self.swarm_tx.try_send(cmd).is_ok()
    }

    /// 获取对等点地址
    pub async fn get_peer_address(&self, peer_id: &str) -> Option<Multiaddr> {
        self.peer_addresses.read().await.get(peer_id).cloned()
    }

    /// 添加对等点地址
    pub fn add_peer_address(&self, peer_id: &str, addr: &str) -> Result<(), TransportError> {
        let peer_id_parsed: libp2p::PeerId = peer_id.parse()
            .map_err(|e| TransportError::ConnectionFailed(format!("无效 PeerId: {}", e)))?;
        let multiaddr: Multiaddr = addr.parse()
            .map_err(|e| TransportError::ConnectionFailed(format!("无效地址：{}", e)))?;
        let multiaddr_clone = multiaddr.clone();

        let cmd = SwarmCommand::AddPeerAddress { peer_id: peer_id_parsed, addr: multiaddr };

        // 使用 blocking_send 因为这是同步方法
        self.swarm_tx.try_send(cmd)
            .map_err(|e| TransportError::ConnectionFailed(format!("发送命令失败：{}", e)))?;

        tracing::info!("已添加对等点地址：{} -> {}", peer_id, multiaddr_clone);
        Ok(())
    }

    /// 请求中继 reservation（用于内网穿透）
    pub fn request_reservation(&self, relay_peer_id: &str, relay_addr: &str) -> Result<(), TransportError> {
        let peer_id: libp2p::PeerId = relay_peer_id.parse()
            .map_err(|e| TransportError::ConnectionFailed(format!("无效 PeerId: {}", e)))?;
        let multiaddr: Multiaddr = relay_addr.parse()
            .map_err(|e| TransportError::ConnectionFailed(format!("无效地址：{}", e)))?;

        let cmd = SwarmCommand::RequestReservation { relay_peer_id: peer_id, relay_addr: multiaddr };

        self.swarm_tx.try_send(cmd)
            .map_err(|e| TransportError::ConnectionFailed(format!("发送命令失败：{}", e)))?;

        tracing::info!("已请求中继 reservation: {} -> {}", relay_peer_id, relay_addr);
        Ok(())
    }

    /// 通过中继节点连接目标对等点
    pub fn connect_via_relay(&self, relay_addr: &str, target_peer_id: &str) -> Result<(), TransportError> {
        let multiaddr: Multiaddr = relay_addr.parse()
            .map_err(|e| TransportError::ConnectionFailed(format!("无效地址：{}", e)))?;
        let target: libp2p::PeerId = target_peer_id.parse()
            .map_err(|e| TransportError::ConnectionFailed(format!("无效 PeerId: {}", e)))?;

        let cmd = SwarmCommand::ConnectViaRelay { relay_addr: multiaddr, target_peer_id: target };

        self.swarm_tx.try_send(cmd)
            .map_err(|e| TransportError::ConnectionFailed(format!("发送命令失败：{}", e)))?;

        tracing::info!("已请求通过中继连接目标对等点：{} via {}", target_peer_id, relay_addr);
        Ok(())
    }

    /// 通过中继连接目标对等点（内部方法，供 connect_to_peer 调用）
    fn connect_via_relay_to_peer(&self, relay_addr: &str, target_peer_id: &str) -> Result<(), TransportError> {
        self.connect_via_relay(relay_addr, target_peer_id)
    }

    /// 获取当前的中继 reservations 列表
    pub async fn get_relay_reservations(&self) -> Vec<RelayReservation> {
        self.relay_reservations.read().await.clone()
    }

    /// 查找可用的中继节点
    async fn find_available_relay(&self) -> Option<RelayReservation> {
        let reservations = self.relay_reservations.read().await;
        reservations.iter().find(|r| r.active).cloned()
    }

    /// 记录对等点连接失败次数
    async fn record_peer_failure(&self, peer_id: &str, success: u32) {
        let mut failures = self.peer_failures.write().await;
        if success == 0 {
            // 成功，重置计数
            failures.insert(peer_id.to_string(), 0);
        } else {
            // 失败，增加计数
            let count = failures.entry(peer_id.to_string()).or_insert(0);
            *count += 1;
        }
    }

    /// 获取对等点失败次数
    pub async fn get_peer_failures(&self, peer_id: &str) -> u32 {
        self.peer_failures.read().await.get(peer_id).copied().unwrap_or(0)
    }

    /// 添加直连连接记录
    async fn add_direct_connection(&self, peer_id: &str) {
        self.direct_connections.write().await.insert(peer_id.to_string());
    }

    /// 获取 Swarm 发送器（供外部发送命令）
    pub fn swarm_sender(&self) -> &mpsc::Sender<SwarmCommand> {
        &self.swarm_tx
    }

    /// 获取 relay_reservations 共享状态引用（供 swarm 事件处理更新）
    pub fn relay_reservations_ref(&self) -> &Arc<tokio::sync::RwLock<Vec<RelayReservation>>> {
        &self.relay_reservations
    }
}

#[async_trait]
impl Transport for Libp2pTransport {
    async fn publish(&self, topic: &str, data: &[u8]) -> Result<(), TransportError> {
        let cmd = SwarmCommand::Publish {
            topic: topic.to_string(),
            data: data.to_vec(),
        };

        self.swarm_tx.send(cmd).await
            .map_err(|e| TransportError::PublishFailed(format!("发送命令失败：{}", e)))?;

        tracing::debug!("已发送发布命令到 topic: {}", topic);
        Ok(())
    }

    async fn subscribe(&self, topic: &str, _handler: Box<dyn MessageHandler>) -> Result<(), TransportError> {
        let cmd = SwarmCommand::Subscribe {
            topic: topic.to_string(),
        };

        self.swarm_tx.send(cmd).await
            .map_err(|e| TransportError::SubscribeFailed(format!("发送命令失败：{}", e)))?;

        tracing::info!("已订阅 topic: {}", topic);
        Ok(())
    }

    async fn unsubscribe(&self, topic: &str) -> Result<(), TransportError> {
        let cmd = SwarmCommand::Unsubscribe {
            topic: topic.to_string(),
        };

        self.swarm_tx.send(cmd).await
            .map_err(|e| TransportError::UnsubscribeFailed(format!("发送命令失败：{}", e)))?;

        tracing::info!("已退订 topic: {}", topic);
        Ok(())
    }

    fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    async fn connect_to_seed(&self, addr: &str) -> Result<(), TransportError> {
        let multiaddr: Multiaddr = addr.parse()
            .map_err(|e| TransportError::ConnectionFailed(format!("无效地址：{}", e)))?;

        let cmd = SwarmCommand::DialDirect { addr: multiaddr };

        self.swarm_tx.send(cmd).await
            .map_err(|e| TransportError::ConnectionFailed(format!("发送命令失败：{}", e)))?;

        tracing::info!("已发送连接命令到：{}", addr);
        Ok(())
    }
}

impl Default for Libp2pTransport {
    fn default() -> Self {
        Self::new(0).unwrap()  // 使用随机端口
    }
}
