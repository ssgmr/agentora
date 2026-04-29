//! 模拟运行器
//!
//! 在独立线程中运行 Simulation，处理命令

use std::sync::mpsc::{Sender, Receiver};
use agentora_network::Transport;
use agentora_core::simulation::{SimConfig, SimMode, Delta, Simulation};
use agentora_core::snapshot::NarrativeEvent;
use agentora_core::WorldSeed;
use agentora_ai::{LlmProvider, config::LlmConfig, OpenAiProvider, FallbackChain};

use crate::bridge::{SimCommand, P2PEvent};
use crate::user_config::UserConfig;

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
    run_simulation_with_api_and_user_config(
        snapshot_tx, delta_tx, narrative_tx, cmd_rx, p2p_event_tx,
        llm_provider, llm_config, config_path, None
    );
}

/// 模拟入口函数（带 UserConfig）
pub fn run_simulation_with_api_and_user_config(
    snapshot_tx: Sender<agentora_core::WorldSnapshot>,
    delta_tx: Sender<Delta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    p2p_event_tx: Sender<P2PEvent>,
    llm_provider: Option<Box<dyn LlmProvider>>,
    llm_config: LlmConfig,
    config_path: String,
    user_config: Option<UserConfig>,
) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_simulation_async_with_api(
            snapshot_tx, delta_tx, narrative_tx, cmd_rx, p2p_event_tx,
            llm_provider, llm_config, config_path, user_config
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
    user_config: Option<UserConfig>,
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

    // ===== 应用 UserConfig =====
    let final_llm_provider = if let Some(config) = &user_config {
        // 合并 Agent 配置到 WorldSeed
        seed.merge_user_config(
            config.agent.name.clone(),
            config.agent.custom_prompt.clone(),
            config.agent.icon_id.clone(),
            config.agent.custom_icon_path.clone(),
            config.p2p.mode.clone(),
            config.p2p.seed_address.clone(),
        );

        tracing::info!(
            "[Bridge] UserConfig 已应用: agent_name={}, p2p_mode={}",
            config.agent.name, config.p2p.mode
        );

        // 根据 LLM mode 选择 Provider
        match config.llm.mode.as_str() {
            "local" => {
                // 本地推理模式（需要 feature）
                #[cfg(feature = "p2p")]
                {
                    use agentora_ai::LlamaProvider;
                    let model_path = config.llm.local_model_path.clone();
                    if model_path.is_empty() {
                        tracing::warn!("[Bridge] local 模式未配置模型路径，使用规则引擎");
                        None
                    } else {
                        match LlamaProvider::new(model_path) {
                            Ok(provider) => Some(Box::new(provider) as Box<dyn LlmProvider>),
                            Err(e) => {
                                tracing::error!("[Bridge] LlamaProvider 初始化失败: {}", e);
                                None
                            }
                        }
                    }
                }
                #[cfg(not(feature = "p2p"))]
                {
                    tracing::warn!("[Bridge] local 模式需要启用 feature，使用传入的 Provider");
                    llm_provider
                }
            }
            "remote" => {
                // 远程 API 模式
                let endpoint = config.llm.api_endpoint.clone();
                let token = config.llm.api_token.clone();
                let model = config.llm.model_name.clone();

                if endpoint.is_empty() {
                    tracing::warn!("[Bridge] remote 模式未配置 endpoint，使用传入的 Provider");
                    llm_provider
                } else {
                    tracing::info!("[Bridge] 创建远程 Provider: endpoint={}, model={}", endpoint, model);
                    let provider = OpenAiProvider::new(endpoint, token, model)
                        .with_timeout(llm_config.primary.timeout_seconds);
                    Some(Box::new(FallbackChain::new(vec![Box::new(provider)])) as Box<dyn LlmProvider>)
                }
            }
            "rule_only" => {
                // 仅规则引擎模式
                tracing::info!("[Bridge] rule_only 模式，不使用 LLM Provider");
                None
            }
            _ => {
                tracing::warn!("[Bridge] 未知的 LLM mode: {}，使用传入的 Provider", config.llm.mode);
                llm_provider
            }
        }
    } else {
        tracing::info!("[Bridge] 无 UserConfig，使用默认配置");
        llm_provider
    };

    // 根据 P2P mode 修改 sim_config
    let final_sim_config = if let Some(config) = &user_config {
        let mut cfg = sim_config.clone();
        match config.p2p.mode.as_str() {
            "single" => {
                cfg.mode = SimMode::Centralized;
            }
            "create" | "join" => {
                cfg.mode = SimMode::P2P { region_size: 16 };
                // seed_peers 已在 seed.merge_user_config 中设置
            }
            _ => {}
        }
        cfg
    } else {
        sim_config
    };

    // 根据 P2P 配置选择构造函数
    let is_p2p_mode = matches!(final_sim_config.mode, SimMode::P2P { .. });

    let mut simulation = if is_p2p_mode {
        // P2P 模式：创建带 P2P 支持的 Simulation
        let local_peer_id = format!("local_{}", final_sim_config.p2p_port);
        tracing::info!("[Bridge] P2P 模式启动 [peer_id={}]", local_peer_id);

        // 设置 Agent 名字前缀，让不同节点的 Agent 名字不同
        // 例如：端口4001的节点生成 "N4001_Agent"，端口4002生成 "N4002_Agent"
        seed.agent_name_prefix = format!("N{}_", final_sim_config.p2p_port);
        tracing::info!("[Bridge] Agent 名字前缀: {}", seed.agent_name_prefix);

        // P2P 模式：跳过 World::new() 中的 Agent 生成，由 Simulation.start() 动态创建
        seed.skip_initial_agents = true;

        let sim = Simulation::with_p2p(
            final_sim_config.clone(),
            seed,
            final_llm_provider,
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
            final_sim_config,
            seed,
            final_llm_provider,
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
