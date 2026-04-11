//! libp2p Transport 实现
//!
//! rust-libp2p 集成：GossipSub + KAD DHT

use async_trait::async_trait;
use crate::transport::{Transport, MessageHandler, TransportError};
use crate::codec::NetworkMessage;
use agentora_sync::PeerId;
use libp2p::{
    identity,
    swarm::{Swarm, SwarmEvent},
    Multiaddr,
    StreamProtocol,
};
use libp2p_gossipsub as gossipsub;
use libp2p_kad as kad;
use libp2p_relay as relay;
use libp2p_tcp as tcp;
use libp2p_noise as noise;
use libp2p_yamux as yamux;
use libp2p_dcutr as dcutr;
use libp2p_autonat as autonat;
use std::time::Duration;
use tokio::sync::mpsc;
use libp2p::futures::StreamExt;
use libp2p::Transport as _;

// 导入 NetworkBehaviour derive 宏
use libp2p_swarm_derive::NetworkBehaviour;

/// 网络行为事件枚举
pub enum AgentoraBehaviourEvent {
    Gossipsub(gossipsub::Event),
    Kademlia(kad::Event),
    RelayClient(relay::client::Event),
    Dcutr(dcutr::Event),
    Autonat(autonat::Event),
    Ping(libp2p_ping::Event),
    Identify(libp2p_identify::Event),
}

impl From<gossipsub::Event> for AgentoraBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        AgentoraBehaviourEvent::Gossipsub(event)
    }
}

impl From<kad::Event> for AgentoraBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        AgentoraBehaviourEvent::Kademlia(event)
    }
}

impl From<relay::client::Event> for AgentoraBehaviourEvent {
    fn from(event: relay::client::Event) -> Self {
        AgentoraBehaviourEvent::RelayClient(event)
    }
}

impl From<dcutr::Event> for AgentoraBehaviourEvent {
    fn from(event: dcutr::Event) -> Self {
        AgentoraBehaviourEvent::Dcutr(event)
    }
}

impl From<autonat::Event> for AgentoraBehaviourEvent {
    fn from(event: autonat::Event) -> Self {
        AgentoraBehaviourEvent::Autonat(event)
    }
}

impl From<libp2p_ping::Event> for AgentoraBehaviourEvent {
    fn from(event: libp2p_ping::Event) -> Self {
        AgentoraBehaviourEvent::Ping(event)
    }
}

impl From<libp2p_identify::Event> for AgentoraBehaviourEvent {
    fn from(event: libp2p_identify::Event) -> Self {
        AgentoraBehaviourEvent::Identify(event)
    }
}

/// 网络行为组合
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "AgentoraBehaviourEvent")]
pub struct AgentoraBehaviour {
    #[behaviour(to_event = "AgentoraBehaviourEvent::Gossipsub")]
    pub gossipsub: gossipsub::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Kademlia")]
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    #[behaviour(to_event = "AgentoraBehaviourEvent::RelayClient")]
    pub relay_client: relay::client::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Dcutr")]
    pub dcutr: dcutr::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Autonat")]
    pub autonat: autonat::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Ping")]
    pub ping: libp2p_ping::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Identify")]
    pub identify: libp2p_identify::Behaviour,
}

/// libp2p Transport 实现
pub struct Libp2pTransport {
    peer_id: PeerId,
    local_key: identity::Keypair,
    swarm_tx: mpsc::Sender<SwarmCommand>,
    /// 中继 reservation 状态
    relay_reservations: std::sync::Arc<tokio::sync::RwLock<Vec<RelayReservation>>>,
    /// NAT 状态（使用 AutoNAT 探测结果）
    nat_status: std::sync::Arc<tokio::sync::RwLock<NatStatus>>,
    /// 直连成功的对等点列表
    direct_connections: std::sync::Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    /// 混合穿透策略配置
    config: HybridStrategyConfig,
    /// 对等点连接失败次数统计（用于降级决策）
    peer_failures: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, u32>>>,
    /// 对等点地址缓存
    peer_addresses: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Multiaddr>>>,
}

/// NAT 状态（来自 AutoNAT）
#[derive(Debug, Clone, PartialEq)]
pub enum NatStatus {
    /// 公网可达，地址为观察到的公网地址
    Public(Multiaddr),
    /// 私有网络，需要中继
    Private,
    /// 未知，正在探测
    Unknown,
}

impl Default for NatStatus {
    fn default() -> Self {
        NatStatus::Unknown
    }
}

impl NatStatus {
    /// 是否为公网可达
    pub fn is_public(&self) -> bool {
        matches!(self, NatStatus::Public(_))
    }

    /// 是否需要中继
    pub fn needs_relay(&self) -> bool {
        matches!(self, NatStatus::Private | NatStatus::Unknown)
    }
}

/// 连接类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    /// 直连
    Direct,
    /// DCUtR 打洞连接
    Dcutr,
    /// 中继连接
    Relay,
}

/// DCUtR 配置
#[derive(Debug, Clone)]
pub struct DcutrConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 单次尝试超时时间（秒）
    pub timeout_secs: u64,
    /// 并发打洞数量
    pub concurrent_attempts: u32,
}

impl Default for DcutrConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout_secs: 10,
            concurrent_attempts: 2,
        }
    }
}

/// AutoNAT 配置
#[derive(Debug, Clone)]
pub struct AutonatConfig {
    /// 是否探测内网地址
    pub only_global_ips: bool,
    /// 探测频率（秒）
    pub probe_interval_secs: u64,
    /// 探测超时（秒）
    pub probe_timeout_secs: u64,
}

impl Default for AutonatConfig {
    fn default() -> Self {
        Self {
            only_global_ips: false,
            probe_interval_secs: 30,
            probe_timeout_secs: 15,
        }
    }
}

/// 混合穿透策略配置
#[derive(Debug, Clone)]
pub struct HybridStrategyConfig {
    /// 直连超时（秒）
    pub direct_timeout_secs: u64,
    /// DCUtR 超时（秒）
    pub dcutr_timeout_secs: u64,
    /// 降级阈值：直连失败多少次后降级到 DCUtR
    pub degradation_threshold: u32,
    /// DCUtR 配置
    pub dcutr: DcutrConfig,
    /// AutoNAT 配置
    pub autonat: AutonatConfig,
    /// 是否启用 DCUtR 打洞功能，默认 true
    pub enable_dcutr: bool,
    /// 是否启用 AutoNAT NAT 探测功能，默认 true
    pub enable_autonat: bool,
}

impl Default for HybridStrategyConfig {
    fn default() -> Self {
        Self {
            direct_timeout_secs: 5,
            dcutr_timeout_secs: 15,
            degradation_threshold: 2,
            dcutr: DcutrConfig::default(),
            autonat: AutonatConfig::default(),
            enable_dcutr: true,
            enable_autonat: true,
        }
    }
}

/// 中继 reservation 信息
#[derive(Debug, Clone)]
pub struct RelayReservation {
    /// 中继节点 PeerId
    pub relay_peer_id: String,
    /// 中继地址
    pub relay_addr: String,
    /// 监听地址（电路地址）
    pub listen_addr: String,
    /// 是否激活
    pub active: bool,
}

/// Swarm 命令枚举
#[derive(Debug)]
enum SwarmCommand {
    Publish {
        topic: String,
        data: Vec<u8>,
    },
    Subscribe {
        topic: String,
    },
    /// 退订 topic
    Unsubscribe {
        topic: String,
    },
    /// 直接连接对等点
    DialDirect {
        addr: Multiaddr,
    },
    /// 通过 DCUtR 打洞连接
    DialViaDcutr {
        peer_id: libp2p::PeerId,
        relay_addr: Multiaddr,
    },
    /// 通过中继连接（保底）
    ConnectViaRelay {
        relay_addr: Multiaddr,
        target_peer_id: libp2p::PeerId,
    },
    AddPeerAddress {
        peer_id: libp2p::PeerId,
        addr: Multiaddr,
    },
    /// 请求中继 reservation
    RequestReservation {
        relay_peer_id: libp2p::PeerId,
        relay_addr: Multiaddr,
    },
}

impl Libp2pTransport {
    pub fn new() -> Result<Self, TransportError> {
        // 生成 ed25519 密钥
        let local_key = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::new(local_key.public().to_peer_id().to_string());

        tracing::info!("生成 PeerId: {}", peer_id.0);

        // 创建通道用于发送命令到 Swarm
        let (swarm_tx, swarm_rx) = mpsc::channel::<SwarmCommand>(100);

        // 创建 relay reservations 共享状态
        let relay_reservations = std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new()));

        // 创建 NAT 状态共享状态
        let nat_status = std::sync::Arc::new(tokio::sync::RwLock::new(NatStatus::Unknown));

        // 创建直连连接共享状态
        let direct_connections = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new()));

        // 创建对等点失败次数统计
        let peer_failures = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        // 创建对等点地址缓存
        let peer_addresses = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        // 启动 Swarm 事件循环
        let key_clone = local_key.clone();
        let peer_id_clone = peer_id.clone();
        let reservations_clone = relay_reservations.clone();
        let nat_status_clone = nat_status.clone();
        let direct_connections_clone = direct_connections.clone();
        tokio::spawn(async move {
            Self::run_swarm_event_loop(
                key_clone,
                peer_id_clone,
                swarm_rx,
                reservations_clone,
                nat_status_clone,
                direct_connections_clone,
            ).await;
        });

        Ok(Self {
            peer_id,
            local_key,
            swarm_tx,
            relay_reservations,
            nat_status,
            direct_connections,
            config: HybridStrategyConfig::default(),
            peer_failures,
            peer_addresses,
        })
    }

    /// 从现有密钥加载
    pub fn load_from_file(key_path: &str) -> Result<Self, TransportError> {
        // 尝试从文件加载密钥
        let local_key = Self::load_key_from_file(key_path).unwrap_or_else(|_| {
            tracing::info!("密钥文件不存在，生成新密钥");
            identity::Keypair::generate_ed25519()
        });

        let libp2p_peer_id = local_key.public().to_peer_id();
        let peer_id = PeerId::new(libp2p_peer_id.to_string());

        tracing::info!("加载 PeerId: {}", peer_id.0);

        // 创建通道用于发送命令到 Swarm
        let (swarm_tx, swarm_rx) = mpsc::channel::<SwarmCommand>(100);

        // 创建 relay reservations 共享状态
        let relay_reservations = std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new()));

        // 创建 NAT 状态共享状态
        let nat_status = std::sync::Arc::new(tokio::sync::RwLock::new(NatStatus::Unknown));

        // 创建直连连接共享状态
        let direct_connections = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new()));

        // 创建对等点失败次数统计
        let peer_failures = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        // 创建对等点地址缓存
        let peer_addresses = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        // 启动 Swarm 事件循环
        let key_clone = local_key.clone();
        let peer_id_clone = peer_id.clone();
        let reservations_clone = relay_reservations.clone();
        let nat_status_clone = nat_status.clone();
        let direct_connections_clone = direct_connections.clone();
        tokio::spawn(async move {
            Self::run_swarm_event_loop(
                key_clone,
                peer_id_clone,
                swarm_rx,
                reservations_clone,
                nat_status_clone,
                direct_connections_clone,
            ).await;
        });

        Ok(Self {
            peer_id,
            local_key,
            swarm_tx,
            relay_reservations,
            nat_status,
            direct_connections,
            config: HybridStrategyConfig::default(),
            peer_failures,
            peer_addresses,
        })
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

    /// 运行 Swarm 事件循环
    async fn run_swarm_event_loop(
        local_key: identity::Keypair,
        peer_id: PeerId,
        mut command_rx: mpsc::Receiver<SwarmCommand>,
        _relay_reservations: std::sync::Arc<tokio::sync::RwLock<Vec<RelayReservation>>>,
        nat_status: std::sync::Arc<tokio::sync::RwLock<NatStatus>>,
        direct_connections: std::sync::Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    ) {
        // 创建 GossipSub 配置
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(|message: &gossipsub::Message| {
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(&message.data);
                gossipsub::MessageId::from(hash.to_vec())
            })
            .build()
            .expect("Valid config");

        // 创建 GossipSub 行为
        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        ).expect("Correct configuration");

        // 创建 Kademlia 配置 - 使用自定义协议名称
        let kad_config = kad::Config::new(StreamProtocol::new("/agentora/kad/1.0.0"));

        // 创建 Kademlia 行为（使用内存存储）
        let store = kad::store::MemoryStore::new(local_key.public().to_peer_id());
        let kademlia = kad::Behaviour::with_config(local_key.public().to_peer_id(), store, kad_config);

        // 创建 Relay 客户端行为 - libp2p-relay 0.18 使用 client::new() 返回 (Transport, Behaviour)
        let (_relay_transport, relay_client) = relay::client::new(local_key.public().to_peer_id());

        // 创建 DCUtR 行为 - 用于 Hole Punching 直连升级
        let dcutr = dcutr::Behaviour::new(local_key.public().to_peer_id());

        // 创建 AutoNAT 行为 - 用于 NAT 类型探测
        let autonat_config = autonat::Config {
            only_global_ips: false,  // 允许探测内网地址
            ..Default::default()
        };
        let autonat = autonat::Behaviour::new(local_key.public().to_peer_id(), autonat_config);

        // 创建 Ping 行为
        let ping = libp2p_ping::Behaviour::default();

        // 创建 Identify 行为
        let identify_config = libp2p_identify::Config::new(
            "/agentora/1.0.0".to_string(),
            local_key.public(),
        );
        let identify = libp2p_identify::Behaviour::new(identify_config);

        // 创建组合行为
        let behaviour = AgentoraBehaviour {
            gossipsub,
            kademlia,
            relay_client,
            dcutr,
            autonat,
            ping,
            identify,
        };

        // 创建传输 - 使用 tokio 特性
        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(libp2p::core::upgrade::Version::V1Lazy)
            .authenticate(noise::Config::new(&local_key).expect("Noise config creation failed"))
            .multiplex(yamux::Config::default())
            .boxed();

        // 构建 Swarm
        let mut swarm = Swarm::new(
            transport,
            behaviour,
            local_key.public().to_peer_id(),
            libp2p::swarm::Config::with_tokio_executor(),
        );

        tracing::info!("Swarm 已启动，PeerId: {}", peer_id.0);

        // 监听所有接口 - 使用 TCP
        let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
        match swarm.listen_on(listen_addr.clone()) {
            Ok(_) => tracing::info!("正在监听：{}", listen_addr),
            Err(e) => tracing::error!("监听失败：{:?}", e),
        }

        // 事件循环
        loop {
            tokio::select! {
                biased;

                command = command_rx.recv() => {
                    let Some(cmd) = command else {
                        break;
                    };
                    match cmd {
                        SwarmCommand::Publish { topic, data } => {
                            let topic = gossipsub::IdentTopic::new(&topic);
                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                                tracing::error!("发布失败：{:?}", e);
                            }
                        }
                        SwarmCommand::Subscribe { topic } => {
                            let topic = gossipsub::IdentTopic::new(&topic);
                            if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                                tracing::error!("订阅失败：{:?}", e);
                            }
                        }
                        SwarmCommand::Unsubscribe { topic } => {
                            let topic = gossipsub::IdentTopic::new(&topic);
                            if !swarm.behaviour_mut().gossipsub.unsubscribe(&topic) {
                                tracing::error!("退订失败：topic={}", topic);
                            }
                        }
                        SwarmCommand::DialDirect { addr } => {
                            tracing::info!("尝试直连对等点：{}", addr);
                            if let Err(e) = swarm.dial(addr) {
                                tracing::error!("直连拨号失败：{:?}", e);
                            }
                        }
                        SwarmCommand::DialViaDcutr { peer_id, relay_addr } => {
                            // DCUtR 打洞：先连接到中继，等待对等点也连接上来
                            let circuit_addr = relay_addr
                                .with(libp2p::multiaddr::Protocol::P2p(peer_id))
                                .with(libp2p::multiaddr::Protocol::P2pCircuit);
                            tracing::info!("发起 DCUtR 打洞：{}", circuit_addr);
                            if let Err(e) = swarm.dial(circuit_addr) {
                                tracing::error!("DCUtR 打洞失败：{:?}", e);
                            }
                        }
                        SwarmCommand::AddPeerAddress { peer_id, addr } => {
                            let addr_clone = addr.clone();
                            // 添加地址到 KAD 路由表
                            swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                            // 触发 KAD 查询以建立连接
                            let _ = swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);
                            tracing::info!("已添加对等点地址到 KAD: {} -> {}", peer_id, addr_clone);
                        }
                        SwarmCommand::RequestReservation { relay_peer_id, relay_addr } => {
                            // 请求中继 reservation - libp2p-relay 0.18 通过拨号到中继地址来请求
                            // 构建中继地址：/ip4/.../tcp/.../p2p/RELAY_PEER_ID
                            let reservation_addr = relay_addr
                                .with(libp2p::multiaddr::Protocol::P2p(relay_peer_id));
                            tracing::info!("请求中继 reservation: {}", reservation_addr);
                            if let Err(e) = swarm.dial(reservation_addr) {
                                tracing::error!("拨号失败：{:?}", e);
                            }
                        }
                        SwarmCommand::ConnectViaRelay { relay_addr, target_peer_id } => {
                            // 构建完整电路地址：relay_addr/p2p-circuit/p2p/TARGET_PEER
                            let circuit_addr = relay_addr
                                .with(libp2p::multiaddr::Protocol::P2pCircuit)
                                .with(libp2p::multiaddr::Protocol::P2p(target_peer_id));
                            tracing::info!("通过中继电路连接目标：{}", circuit_addr);
                            if let Err(e) = swarm.dial(circuit_addr) {
                                tracing::error!("中继拨号失败：{:?}", e);
                            }
                        }
                    }
                }
                event = swarm.select_next_some() => {
                    Self::handle_swarm_event(event, nat_status.clone(), direct_connections.clone()).await;
                }
            }
        }
    }

    /// 处理 Swarm 事件
    async fn handle_swarm_event(
        event: SwarmEvent<AgentoraBehaviourEvent>,
        nat_status: std::sync::Arc<tokio::sync::RwLock<NatStatus>>,
        direct_connections: std::sync::Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    ) {
        use gossipsub::Event as GossipsubEvent;
        use kad::{Event as KademliaEvent, QueryResult};

        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                tracing::info!("新的监听地址：{}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                tracing::info!("连接到 peer: {} ({:?})", peer_id, endpoint);
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                tracing::info!("与 peer {} 连接关闭：{:?}", peer_id, cause);
            }
            SwarmEvent::Behaviour(behaviour_event) => {
                match behaviour_event {
                    AgentoraBehaviourEvent::Gossipsub(gossipsub_event) => {
                        match gossipsub_event {
                            GossipsubEvent::Message {
                                propagation_source: peer_id,
                                message_id: id,
                                message,
                            } => {
                                tracing::debug!("收到 GossipSub 消息：from={}, id={}", peer_id, id);

                                // 解析消息
                                if let Ok(network_msg) = NetworkMessage::from_bytes(&message.data) {
                                    tracing::debug!("消息内容：{:?}", network_msg);
                                }
                            }
                            GossipsubEvent::Subscribed { peer_id, topic } => {
                                tracing::debug!("Peer {} 订阅 topic: {}", peer_id, topic);
                            }
                            GossipsubEvent::Unsubscribed { peer_id, topic } => {
                                tracing::debug!("Peer {} 退订 topic: {}", peer_id, topic);
                            }
                            _ => {}
                        }
                    }
                    AgentoraBehaviourEvent::Kademlia(kad_event) => {
                        match kad_event {
                            KademliaEvent::OutboundQueryProgressed { result, .. } => {
                                match result {
                                    QueryResult::GetClosestPeers(result) => {
                                        match result {
                                            Ok(ok) => tracing::info!("KAD 查询完成：找到 {} peers", ok.peers.len()),
                                            Err(e) => tracing::warn!("KAD 查询失败：{:?}", e),
                                        }
                                    }
                                    QueryResult::StartProviding(result) => {
                                        match result {
                                            Ok(ok) => tracing::info!("KAD 提供记录成功：{:?}", ok),
                                            Err(e) => tracing::warn!("KAD 提供记录失败：{:?}", e),
                                        }
                                    }
                                    QueryResult::GetProviders(result) => {
                                        match result {
                                            Ok(_) => tracing::info!("KAD 获取提供者成功"),
                                            Err(e) => tracing::warn!("KAD 获取提供者失败：{:?}", e),
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            KademliaEvent::RoutingUpdated { peer, is_new_peer, .. } => {
                                tracing::debug!("KAD 路由更新：peer={}, new={}", peer, is_new_peer);
                            }
                            _ => {}
                        }
                    }
                    AgentoraBehaviourEvent::RelayClient(relay_event) => {
                        // libp2p-relay 0.18 client 事件处理
                        match relay_event {
                            relay::client::Event::ReservationReqAccepted { relay_peer_id, renewal, limit } => {
                                tracing::info!("中继 reservation 请求已接受：{} (renewal={:?}, limit={:?})", relay_peer_id, renewal, limit);
                            }
                            relay::client::Event::OutboundCircuitEstablished { relay_peer_id, limit } => {
                                tracing::info!("通过中继建立出站电路连接：{} (limit={:?})", relay_peer_id, limit);
                            }
                            relay::client::Event::InboundCircuitEstablished { src_peer_id, limit } => {
                                tracing::info!("通过中继建立入站电路连接：{} (limit={:?})", src_peer_id, limit);
                            }
                        }
                    }
                    AgentoraBehaviourEvent::Dcutr(dcutr_event) => {
                        // DCUtR Hole Punching 事件处理
                        // dcutr::Event 结构：{ remote_peer_id, result: Result<ConnectionId, Error> }
                        match dcutr_event.result {
                            Ok(connection_id) => {
                                tracing::info!("DCUtR 直连升级成功：{} (connection={:?})", dcutr_event.remote_peer_id, connection_id);
                                // 记录直连成功
                                direct_connections.write().await.insert(dcutr_event.remote_peer_id.to_string());
                            }
                            Err(error) => {
                                tracing::warn!("DCUtR 直连升级失败：{} - {:?}", dcutr_event.remote_peer_id, error);
                            }
                        }
                    }
                    AgentoraBehaviourEvent::Autonat(autonat_event) => {
                        // AutoNAT NAT 类型探测事件处理
                        match autonat_event {
                            autonat::Event::OutboundProbe(event) => {
                                match event {
                                    autonat::OutboundProbeEvent::Response { address, .. } => {
                                        tracing::info!("AutoNAT 出站探测成功，观察到的公网地址：{}", address);
                                        // 更新 NAT 状态为 Public
                                        *nat_status.write().await = NatStatus::Public(address);
                                    }
                                    autonat::OutboundProbeEvent::Error { error, .. } => {
                                        tracing::warn!("AutoNAT 出站探测失败：{:?}", error);
                                    }
                                    _ => {}
                                }
                            }
                            autonat::Event::InboundProbe(event) => {
                                match event {
                                    autonat::InboundProbeEvent::Response { .. } => {
                                        tracing::info!("AutoNAT 入站探测成功");
                                    }
                                    autonat::InboundProbeEvent::Error { error, .. } => {
                                        tracing::warn!("AutoNAT 入站探测失败：{:?}", error);
                                    }
                                    _ => {}
                                }
                            }
                            autonat::Event::StatusChanged { old, new } => {
                                tracing::info!("AutoNAT NAT 状态变更：{:?} -> {:?}", old, new);
                                // 根据新的 NAT 状态更新内部状态
                                let status = match new {
                                    autonat::NatStatus::Public(addr) => NatStatus::Public(addr),
                                    autonat::NatStatus::Private => NatStatus::Private,
                                    autonat::NatStatus::Unknown => NatStatus::Unknown,
                                };
                                *nat_status.write().await = status;
                            }
                        }
                    }
                    AgentoraBehaviourEvent::Ping(ping_event) => {
                        tracing::debug!("Ping 事件：{:?}", ping_event);
                    }
                    AgentoraBehaviourEvent::Identify(identify_event) => {
                        match identify_event {
                            libp2p_identify::Event::Received { peer_id, info, .. } => {
                                tracing::info!("Identify 信息：from={}, agent={}", peer_id, info.agent_version);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
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

impl Libp2pTransport {
    /// 获取 NAT 状态
    pub async fn get_nat_status(&self) -> NatStatus {
        self.nat_status.read().await.clone()
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
    ///
    /// # 参数
    /// * `peer_id` - 目标对等点的 PeerId
    ///
    /// # 返回
    /// * `Ok(ConnectionType)` - 连接成功，返回连接类型
    /// * `Err(TransportError)` - 连接失败
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
    ///
    /// # 参数
    /// * `relay_peer_id` - 中继节点的 PeerId
    /// * `relay_addr` - 中继节点的地址（不包含 /p2p-circuit）
    ///
    /// # 示例
    /// ```ignore
    /// let transport = Libp2pTransport::new()?;
    /// let relay_peer_id = "12D3KooW...";
    /// let relay_addr = "/ip4/1.2.3.4/tcp/4001";
    /// transport.request_reservation(relay_peer_id, relay_addr)?;
    /// ```
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
    ///
    /// # 参数
    /// * `relay_addr` - 中继节点的地址（不包含 /p2p-circuit）
    /// * `target_peer_id` - 目标对等点的 PeerId
    ///
    /// # 示例
    /// ```ignore
    /// let transport = Libp2pTransport::new()?;
    /// let relay_addr = "/ip4/1.2.3.4/tcp/4001/p2p/RELAY_PEER";
    /// let target = "12D3KooW...";
    /// transport.connect_via_relay(relay_addr, target)?;
    /// ```
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
}

impl Default for Libp2pTransport {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
