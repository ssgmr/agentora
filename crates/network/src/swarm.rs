//! Swarm 命令和事件循环
//!
//! SwarmCommand 命令定义、run_swarm_event_loop、handle_swarm_event

use crate::behaviour::{AgentoraBehaviour, AgentoraBehaviourEvent};
use crate::nat::{NatStatus, ConnectionType};
use crate::config::{RelayReservation, ConnectedPeer};
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
use libp2p_autonat as autonat;
use std::time::Duration;
use tokio::sync::mpsc;
use libp2p::futures::StreamExt;
use libp2p::Transport as _;
use chrono::Utc;

/// Swarm 命令枚举
#[derive(Debug)]
pub enum SwarmCommand {
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

/// 运行 Swarm 事件循环
pub async fn run_swarm_event_loop(
    local_key: identity::Keypair,
    peer_id: PeerId,
    listen_port: u16,
    mut command_rx: mpsc::Receiver<SwarmCommand>,
    message_tx: mpsc::Sender<NetworkMessage>,
    relay_reservations: std::sync::Arc<tokio::sync::RwLock<Vec<RelayReservation>>>,
    nat_status: std::sync::Arc<tokio::sync::RwLock<NatStatus>>,
    direct_connections: std::sync::Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    // 已连接节点列表（新增）
    connected_peers: std::sync::Arc<tokio::sync::RwLock<Vec<ConnectedPeer>>>,
    // 订阅的 topic 列表（新增）
    subscribed_topics: std::sync::Arc<tokio::sync::RwLock<Vec<String>>>,
    topic_handlers: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Box<dyn crate::transport::MessageHandler>>>>,
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
    let dcutr = libp2p_dcutr::Behaviour::new(local_key.public().to_peer_id());

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

    // 监听所有接口 - 使用配置的端口
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", listen_port).parse()
        .unwrap_or_else(|_| "/ip4/0.0.0.0/tcp/0".parse().unwrap());
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
                handle_swarm_command(&mut swarm, cmd, subscribed_topics.clone()).await;
            }
            event = swarm.select_next_some() => {
                handle_swarm_event(
                    event,
                    message_tx.clone(),
                    nat_status.clone(),
                    direct_connections.clone(),
                    relay_reservations.clone(),
                    connected_peers.clone(),
                    subscribed_topics.clone(),
                    peer_id.clone(),
                    topic_handlers.clone()
                ).await;
            }
        }
    }
}

/// 处理 Swarm 命令
async fn handle_swarm_command(
    swarm: &mut Swarm<AgentoraBehaviour>,
    cmd: SwarmCommand,
    subscribed_topics: std::sync::Arc<tokio::sync::RwLock<Vec<String>>>,
) {
    match cmd {
        SwarmCommand::Publish { topic, data } => {
            let topic = gossipsub::IdentTopic::new(&topic);
            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                tracing::error!("发布失败：{:?}", e);
            }
        }
        SwarmCommand::Subscribe { topic } => {
            let gossip_topic = gossipsub::IdentTopic::new(&topic);
            if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&gossip_topic) {
                tracing::error!("订阅失败：{:?}", e);
            } else {
                // 订阅成功，添加到列表
                let topic_name = topic.clone();
                let mut topics = subscribed_topics.write().await;
                if !topics.contains(&topic_name) {
                    topics.push(topic_name);
                }
                tracing::info!("订阅成功：{}", topic);
            }
        }
        SwarmCommand::Unsubscribe { topic } => {
            let gossip_topic = gossipsub::IdentTopic::new(&topic);
            if !swarm.behaviour_mut().gossipsub.unsubscribe(&gossip_topic) {
                tracing::error!("退订失败：topic={}", topic);
            } else {
                // 退订成功，从列表移除
                let mut topics = subscribed_topics.write().await;
                topics.retain(|t| t != &topic);
                tracing::info!("退订成功：{}", topic);
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

/// 处理 Swarm 事件
pub async fn handle_swarm_event(
    event: SwarmEvent<AgentoraBehaviourEvent>,
    message_tx: mpsc::Sender<NetworkMessage>,
    nat_status: std::sync::Arc<tokio::sync::RwLock<NatStatus>>,
    direct_connections: std::sync::Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    relay_reservations: std::sync::Arc<tokio::sync::RwLock<Vec<RelayReservation>>>,
    // 已连接节点列表（新增）
    connected_peers: std::sync::Arc<tokio::sync::RwLock<Vec<ConnectedPeer>>>,
    // 订阅的 topic 列表（新增）
    subscribed_topics: std::sync::Arc<tokio::sync::RwLock<Vec<String>>>,
    // 本地 PeerId（用于过滤本地订阅事件）
    local_peer_id: PeerId,
    topic_handlers: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Box<dyn crate::transport::MessageHandler>>>>,
) {
    use gossipsub::Event as GossipsubEvent;
    use kad::{Event as KademliaEvent, QueryResult};

    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            tracing::info!("新的监听地址：{}", address);
        }
        SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
            tracing::info!("连接到 peer: {} ({:?})", peer_id, endpoint);

            // 创建 ConnectedPeer 并添加到列表
            let connected_peer = ConnectedPeer {
                peer_id: peer_id.to_string(),
                agent_version: String::new(), // 待 Identify 协议更新
                connection_type: ConnectionType::Direct, // 默认直连，后续可能更新
                connected_at: Utc::now().to_rfc3339(),
                is_relay_server: false, // 待 Identify 更新
                listen_addr: Some(endpoint.get_remote_address().to_string()),
            };

            // 写入共享状态
            let mut peers = connected_peers.write().await;
            // 检查是否已存在（避免重复添加）
            if !peers.iter().any(|p| p.peer_id == peer_id.to_string()) {
                peers.push(connected_peer);
            }
        }
        SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
            tracing::info!("与 peer {} 连接关闭：{:?}", peer_id, cause);

            // 从共享状态移除
            let mut peers = connected_peers.write().await;
            peers.retain(|p| p.peer_id != peer_id.to_string());
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

                                // 调用所有已注册的 handler（广播模式）
                                let handlers = topic_handlers.read().await;
                                for (_topic, handler) in handlers.iter() {
                                    handler.handle(network_msg.clone()).await;
                                }

                                // 同时发送到消息通道供上层消费（向后兼容）
                                if let Err(e) = message_tx.try_send(network_msg) {
                                    tracing::warn!("消息通道发送失败：{:?}", e);
                                }
                            }
                        }
                        GossipsubEvent::Subscribed { peer_id, topic } => {
                            // 如果是本地订阅（peer_id == local_peer_id 的 libp2p PeerId）
                            // 注意：这里的 peer_id 是 libp2p::PeerId，需要与 local_peer_id 比较
                            tracing::debug!("Peer {} 订阅 topic: {}", peer_id, topic);
                            // 本地订阅已在 handle_swarm_command 中跟踪
                        }
                        GossipsubEvent::Unsubscribed { peer_id, topic } => {
                            tracing::debug!("Peer {} 退订 topic: {}", peer_id, topic);
                            // 本地退订已在 handle_swarm_command 中跟踪
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
                            // Task 2.1: 写入 relay_reservations
                            let relay_addr_str = relay_peer_id.to_string();
                            let mut reservations = relay_reservations.write().await;
                            // 检查是否已存在
                            if let existing @ Some(_) = reservations.iter_mut().find(|r| r.relay_peer_id == relay_addr_str) {
                                if let Some(r) = existing {
                                    r.active = true;
                                }
                            } else {
                                reservations.push(RelayReservation {
                                    relay_peer_id: relay_addr_str,
                                    relay_addr: relay_peer_id.to_string(),
                                    listen_addr: String::new(),
                                    active: true,
                                });
                            }
                        }
                        relay::client::Event::OutboundCircuitEstablished { relay_peer_id, limit } => {
                            tracing::info!("通过中继建立出站电路连接：{} (limit={:?})", relay_peer_id, limit);
                            // Task 2.2: 更新 reservation 的 active 状态
                            let relay_addr_str = relay_peer_id.to_string();
                            let mut reservations = relay_reservations.write().await;
                            if let Some(r) = reservations.iter_mut().find(|r| r.relay_peer_id == relay_addr_str) {
                                r.active = true;
                            }
                        }
                        relay::client::Event::InboundCircuitEstablished { src_peer_id, limit } => {
                            tracing::info!("通过中继建立入站电路连接：{} (limit={:?})", src_peer_id, limit);
                            // Task 2.2: 更新 reservation 的 active 状态
                            let src_addr_str = src_peer_id.to_string();
                            let mut reservations = relay_reservations.write().await;
                            if let Some(r) = reservations.iter_mut().find(|r| r.relay_peer_id == src_addr_str) {
                                r.active = true;
                            }
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

                            // 更新 connected_peers 中的 agent_version
                            let mut peers = connected_peers.write().await;
                            if let Some(peer) = peers.iter_mut().find(|p| p.peer_id == peer_id.to_string()) {
                                peer.agent_version = info.agent_version.clone();
                                // 判断是否为中继服务器
                                peer.is_relay_server = info.agent_version.contains("relay")
                                    || info.agent_version.contains("libp2p-relay");
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }
}