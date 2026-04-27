//! 模拟运行器
//!
//! 在独立线程中运行 Simulation，处理命令

use std::sync::mpsc::{Sender, Receiver};
use agentora_network::Transport;
use agentora_core::simulation::{SimConfig, SimMode, Delta, Simulation};
use agentora_core::snapshot::NarrativeEvent;
use agentora_core::WorldSeed;
use agentora_ai::{LlmProvider, config::LlmConfig};

use crate::bridge::{SimCommand, P2PEvent};

/// 模拟入口函数（在独立线程中运行）
pub fn run_simulation_with_api(
    snapshot_tx: Sender<agentora_core::WorldSnapshot>,
    delta_tx: Sender<Delta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    p2p_event_tx: Sender<P2PEvent>,
    llm_provider: Option<Box<dyn LlmProvider>>,
    llm_config: LlmConfig,
    config_path: String,
) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_simulation_async_with_api(
            snapshot_tx, delta_tx, narrative_tx, cmd_rx, p2p_event_tx,
            llm_provider, llm_config, config_path
        ).await;
    });
}

/// 异步模拟主函数（使用 Simulation 结构体）
async fn run_simulation_async_with_api(
    snapshot_tx: Sender<agentora_core::WorldSnapshot>,
    delta_tx: Sender<Delta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    p2p_event_tx: Sender<P2PEvent>,
    llm_provider: Option<Box<dyn LlmProvider>>,
    llm_config: LlmConfig,
    config_path: String,
) {
    // 优先使用环境变量，否则使用传入的配置路径
    let actual_config_path = if let Ok(env_path) = std::env::var("AGENTORA_SIM_CONFIG") {
        tracing::info!("[Bridge] 使用环境变量配置: {}", env_path);
        env_path
    } else if config_path.is_empty() {
        "../config/sim.toml".to_string()
    } else {
        config_path
    };

    // 加载模拟配置
    tracing::info!("[Bridge] 加载配置: {}", actual_config_path);
    let sim_config = SimConfig::load(&actual_config_path);

    // 从配置文件加载世界种子
    let mut seed = WorldSeed::load("../worldseeds/default.toml")
        .unwrap_or_else(|e| {
            tracing::error!("加载世界种子失败: {}，使用默认配置", e);
            WorldSeed::default()
        });
    seed.initial_agents = sim_config.initial_agent_count as u32;

    // 根据 P2P 配置选择构造函数
    let is_p2p_mode = matches!(sim_config.mode, SimMode::P2P { .. });

    let mut simulation = if is_p2p_mode {
        // P2P 模式：创建带 P2P 支持的 Simulation
        let local_peer_id = format!("local_{}", sim_config.p2p_port);
        tracing::info!("[Bridge] P2P 模式启动 [peer_id={}]", local_peer_id);

        // 设置 Agent 名字前缀，让不同节点的 Agent 名字不同
        // 例如：端口4001的节点生成 "N4001_Agent"，端口4002生成 "N4002_Agent"
        seed.agent_name_prefix = format!("N{}_", sim_config.p2p_port);
        tracing::info!("[Bridge] Agent 名字前缀: {}", seed.agent_name_prefix);

        // P2P 模式：跳过 World::new() 中的 Agent 生成，由 Simulation.start() 动态创建
        seed.skip_initial_agents = true;

        let sim = Simulation::with_p2p(
            sim_config.clone(),
            seed,
            llm_provider,
            &llm_config,
            snapshot_tx,
            delta_tx,
            narrative_tx,
            local_peer_id.clone(),
        );

        // 通知 Bridge 初始化状态
        let _ = p2p_event_tx.send(P2PEvent::StatusChanged {
            nat_status: "initializing".to_string(),
            peer_count: 0,
            error: String::new(),
        });

        sim
    } else {
        // 中心化模式
        tracing::info!("[Bridge] 中心化模式启动");
        Simulation::new(
            sim_config,
            seed,
            llm_provider,
            &llm_config,
            snapshot_tx,
            delta_tx,
            narrative_tx,
        )
    };

    // 获取真实的 libp2p PeerId（如果有 transport）
    #[cfg(feature = "p2p")]
    let initial_peer_id = simulation.transport_ref()
        .map(|t| t.peer_id().0.clone())
        .unwrap_or_else(|| simulation.local_peer_id().unwrap_or("").to_string());

    #[cfg(not(feature = "p2p"))]
    let initial_peer_id = simulation.local_peer_id().unwrap_or("").to_string();

    // 启动模拟（异步）
    simulation.start().await;

    // 发送真实的 peer_id 到 Bridge
    if !initial_peer_id.is_empty() {
        tracing::info!("[Bridge] 发送真实 PeerId: {}", initial_peer_id);
        let _ = p2p_event_tx.send(P2PEvent::PeerIdReady { peer_id: initial_peer_id });
    }

    // 命令处理循环
    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                SimCommand::Pause => {
                    simulation.pause();
                }
                SimCommand::Start => {
                    simulation.resume();
                }
                SimCommand::SetTickInterval { seconds } => {
                    simulation.set_tick_interval(seconds).await;
                }
                SimCommand::InjectPreference { agent_id, key, boost, duration_ticks } => {
                    simulation.inject_preference(agent_id, key, boost, duration_ticks).await;
                }
                SimCommand::ConnectToSeed { addr } => {
                    tracing::info!("[Bridge] ConnectToSeed: {}", addr);
                    if let Some(transport) = simulation.transport_ref() {
                        if let Err(e) = transport.connect_to_seed(&addr).await {
                            tracing::warn!("[Bridge] 连接种子节点失败: {:?}", e);
                            let _ = p2p_event_tx.send(P2PEvent::StatusChanged {
                                nat_status: "error".to_string(),
                                peer_count: 0,
                                error: format!("连接失败: {:?}", e),
                            });
                        }
                    } else {
                        tracing::warn!("[Bridge] P2P 未启用，忽略 ConnectToSeed");
                    }
                }
                SimCommand::QueryPeerInfo { query_type, response_tx } => {
                    let result = match query_type.as_str() {
                        "peer_id" => {
                            // 返回真实的 libp2p PeerId（如果有 transport）
                            #[cfg(feature = "p2p")]
                            {
                                simulation.transport_ref()
                                    .map(|t| t.peer_id().0.clone())
                                    .unwrap_or_else(|| simulation.local_peer_id().unwrap_or("").to_string())
                            }
                            #[cfg(not(feature = "p2p"))]
                            {
                                simulation.local_peer_id().unwrap_or("").to_string()
                            }
                        }
                        "nat_status" => {
                            if let Some(transport) = simulation.transport_ref() {
                                let status = transport.get_nat_status().await;
                                match status {
                                    agentora_network::NatStatus::Public(ref addr) => {
                                        format!("{{\"status\": \"public\", \"address\": \"{}\"}}", addr)
                                    }
                                    agentora_network::NatStatus::Private => {
                                        "{\"status\": \"private\", \"address\": \"\"}".to_string()
                                    }
                                    agentora_network::NatStatus::Unknown => {
                                        "{\"status\": \"unknown\", \"address\": \"\"}".to_string()
                                    }
                                }
                            } else {
                                "{\"status\": \"disabled\", \"address\": \"\"}".to_string()
                            }
                        }
                        "peers" => {
                            if let Some(transport) = simulation.transport_ref() {
                                let connected_peers = transport.get_connected_peers().await;
                                serde_json::to_string(&connected_peers).unwrap_or_else(|_| "[]".to_string())
                            } else {
                                "[]".to_string()
                            }
                        }
                        "topics" => {
                            if let Some(transport) = simulation.transport_ref() {
                                let topics = transport.get_subscribed_topics().await;
                                serde_json::to_string(&topics).unwrap_or_else(|_| "[]".to_string())
                            } else {
                                "[]".to_string()
                            }
                        }
                        _ => "[]".to_string(),
                    };
                    let _ = response_tx.send(result);
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
