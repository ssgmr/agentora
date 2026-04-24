//! 模拟编排结构体
//!
//! 封装完整的模拟生命周期管理，提供统一 API 给 Bridge 调用

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::{World, WorldSeed, WorldSnapshot, AgentId, DecisionPipeline};
use crate::agent::inventory::{InventoryConfig, init_inventory_config};
use agentora_ai::{LlmProvider, LlmConfig};

use super::config::SimConfig;
#[cfg(feature = "p2p")]
use super::config::SimMode;
use super::delta::Delta;
use super::agent_loop::NarrativeEvent;
use super::p2p_handler::P2PMessageHandler;

#[cfg(feature = "p2p")]
use agentora_network::{Libp2pTransport, Transport, NetworkMessage, AgentDeltaMessage, NarrativeMessage};
#[cfg(feature = "p2p")]
use agentora_network::gossip::RegionTopicManager;

/// 模拟编排结构体
///
/// 封装 World、Pipeline、Agent循环、Tick循环、Snapshot循环
/// 提供 start/pause/resume/inject_preference 等公开 API
pub struct Simulation {
    /// 共享世界状态
    world: Arc<Mutex<World>>,
    /// 决策管道
    pipeline: Arc<DecisionPipeline>,
    /// 模拟配置
    config: SimConfig,
    /// 暂停状态
    is_paused: Arc<AtomicBool>,
    /// 运行状态
    is_running: AtomicBool,
    /// Snapshot 广播通道
    snapshot_tx: Sender<WorldSnapshot>,
    /// Delta 广播通道
    delta_tx: Sender<Delta>,
    /// Narrative 广播通道
    narrative_tx: Sender<NarrativeEvent>,
    /// Agent task handles
    agent_handles: Vec<JoinHandle<()>>,
    /// Tick task handle
    tick_handle: Option<JoinHandle<()>>,
    /// Snapshot task handle
    snapshot_handle: Option<JoinHandle<()>>,
    /// P2P 消息处理器（P2P 模式下启用）
    p2p_handler: Option<P2PMessageHandler>,
    /// 本地 peer ID（P2P 模式）
    local_peer_id: Option<String>,
    /// libp2p 传输层（P2P 模式）
    #[cfg(feature = "p2p")]
    transport: Option<Libp2pTransport>,
    /// 区域 topic 管理器（P2P 模式）
    #[cfg(feature = "p2p")]
    region_topic_manager: Option<RegionTopicManager>,
    /// P2P 网络消息循环 handle（P2P 模式）
    #[cfg(feature = "p2p")]
    p2p_network_handle: Option<JoinHandle<()>>,
}

impl Simulation {
    /// 创建模拟实例
    ///
    /// # Arguments
    /// - `config` — 模拟配置（Agent数量、决策间隔等）
    /// - `seed` — 世界种子（地图尺寸、地形比例等）
    /// - `llm_provider` — LLM Provider（可选，无则使用规则引擎兜底）
    /// - `llm_config` — LLM 配置（记忆系统参数）
    /// - `snapshot_tx` — Snapshot 发送通道
    /// - `delta_tx` — Delta 发送通道
    /// - `narrative_tx` — Narrative 发送通道
    pub fn new(
        config: SimConfig,
        seed: WorldSeed,
        llm_provider: Option<Box<dyn LlmProvider>>,
        llm_config: &LlmConfig,
        snapshot_tx: Sender<WorldSnapshot>,
        delta_tx: Sender<Delta>,
        narrative_tx: Sender<NarrativeEvent>,
    ) -> Self {
        // 初始化背包配置
        init_inventory_config(InventoryConfig {
            max_slots: config.inventory_max_slots,
            max_stack_size: config.inventory_max_stack_size,
            warehouse_limit_multiplier: config.inventory_warehouse_multiplier,
        });

        // 创建世界
        let mut world = World::new(&seed);
        // 应用运行时配置
        world.trade_timeout_ticks = config.trade_timeout_ticks;
        // 设置运行模式
        world.set_sim_mode(&config.mode);

        // 创建决策管道
        let pipeline = if let Some(provider) = llm_provider {
            DecisionPipeline::from_config(&llm_config.memory)
                .with_llm_provider(provider)
                .with_llm_params(llm_config.decision.max_tokens, llm_config.decision.temperature)
        } else {
            DecisionPipeline::from_config(&llm_config.memory)
                .with_llm_params(llm_config.decision.max_tokens, llm_config.decision.temperature)
        };

        tracing::info!(
            "[Simulation] 创建成功 [agents={} npc={} world_size={}x{} mode={:?}]",
            config.initial_agent_count,
            config.npc_count,
            seed.map_size[0],
            seed.map_size[1],
            config.mode
        );

        Self {
            world: Arc::new(Mutex::new(world)),
            pipeline: Arc::new(pipeline),
            config,
            is_paused: Arc::new(AtomicBool::new(false)),
            is_running: AtomicBool::new(false),
            snapshot_tx,
            delta_tx,
            narrative_tx,
            agent_handles: Vec::new(),
            tick_handle: None,
            snapshot_handle: None,
            p2p_handler: None,
            local_peer_id: None,
            #[cfg(feature = "p2p")]
            transport: None,
            #[cfg(feature = "p2p")]
            region_topic_manager: None,
            #[cfg(feature = "p2p")]
            p2p_network_handle: None,
        }
    }

    /// 创建带 P2P 支持的模拟实例
    ///
    /// # Arguments
    /// - 同上
    /// - `local_peer_id` — 本地 peer ID（用于过滤回环）
    pub fn with_p2p(
        config: SimConfig,
        seed: WorldSeed,
        llm_provider: Option<Box<dyn LlmProvider>>,
        llm_config: &LlmConfig,
        snapshot_tx: Sender<WorldSnapshot>,
        delta_tx: Sender<Delta>,
        narrative_tx: Sender<NarrativeEvent>,
        local_peer_id: String,
    ) -> Self {
        // 先 clone 通道，因为后面会 move
        let delta_tx_for_network = delta_tx.clone();
        let delta_tx_for_p2p = delta_tx.clone();
        let narrative_tx_for_network = narrative_tx.clone();
        let local_peer_id_for_log = local_peer_id.clone();

        let mut sim = Self::new(
            config,
            seed,
            llm_provider,
            llm_config,
            snapshot_tx,
            delta_tx,
            narrative_tx,
        );

        // 创建 P2P 消息处理器
        sim.p2p_handler = Some(P2PMessageHandler::new(
            local_peer_id.clone(),
            delta_tx_for_p2p,
            300, // shadow_timeout_ticks
        ));
        sim.local_peer_id = Some(local_peer_id);

        #[cfg(feature = "p2p")]
        {
            // 创建 libp2p 传输层
            match Libp2pTransport::new() {
                Ok(mut transport) => {
                    // 注册 region topic 消息处理器
                    let delta_tx_for_handler = delta_tx_for_network.clone();
                    let narrative_tx_for_handler = narrative_tx_for_network.clone();
                    let peer_id = sim.local_peer_id.clone().unwrap_or_default();

                    // 将 transport 的消息接收器取出，在网络循环中消费
                    let message_rx = transport.take_message_receiver();

                    // 创建区域 topic 管理器
                    let topic_manager = RegionTopicManager::new();

                    // 启动网络消息循环（在独立 task 中消费网络消息）
                    let network_handle = tokio::spawn(async move {
                        Self::run_p2p_network_loop(
                            message_rx,
                            delta_tx_for_handler,
                            narrative_tx_for_handler,
                            &peer_id,
                        ).await;
                    });

                    sim.transport = Some(transport);
                    sim.region_topic_manager = Some(topic_manager);
                    sim.p2p_network_handle = Some(network_handle);

                    tracing::info!("[Simulation] libp2p 传输层已创建，等待连接...");
                }
                Err(e) => {
                    tracing::error!("[Simulation] 创建 libp2p 传输层失败: {:?}", e);
                }
            }
        }

        tracing::info!("[Simulation] P2P 模式启用，local_peer_id={}", local_peer_id_for_log);

        sim
    }

    /// 启动模拟（异步版本，在 tokio runtime 内调用）
    ///
    /// 创建并 spawn 所有 Agent 决策循环、Tick 循环、Snapshot 循环
    /// P2P 模式下只 spawn local_agent_ids 对应的 Agent
    pub async fn start(&mut self) {
        if self.is_running.load(Ordering::SeqCst) {
            tracing::warn!("[Simulation] 模拟已在运行");
            return;
        }

        tracing::info!("[Simulation] 启动模拟...");
        self.is_running.store(true, Ordering::SeqCst);

        // 获取需要运行的 Agent ID（根据 SimMode）
        let agent_ids: Vec<AgentId>;
        {
            let world = self.world.lock().await;
            match &self.config.mode {
                super::config::SimMode::Centralized => {
                    // 集中式模式：所有 Agent
                    agent_ids = world.agents.keys().cloned().collect();
                }
                super::config::SimMode::P2P { local_agent_ids, .. } => {
                    // P2P 模式：只运行指定的 Agent
                    agent_ids = local_agent_ids.iter()
                        .map(|id| AgentId::new(id.clone()))
                        .filter(|id| world.agents.contains_key(id))
                        .collect();
                }
            }
        }

        // Spawn Agent 决策循环
        for agent_id in &agent_ids {
            let handle = self.spawn_agent_loop(agent_id.clone(), false);
            self.agent_handles.push(handle);
        }

        // 创建 NPC Agent（异步）
        // P2P 模式下不创建 NPC
        let npc_ids: Vec<AgentId>;
        match &self.config.mode {
            super::config::SimMode::Centralized => {
                npc_ids = self.create_npc_agents().await;
            }
            super::config::SimMode::P2P { .. } => {
                npc_ids = Vec::new();
            }
        }
        for npc_id in &npc_ids {
            let handle = self.spawn_agent_loop(npc_id.clone(), true);
            self.agent_handles.push(handle);
        }

        // 发送初始 snapshot（异步）
        {
            let world = self.world.lock().await;
            let initial_snapshot = world.snapshot();
            let _ = self.snapshot_tx.send(initial_snapshot);
            tracing::info!("[Simulation] 已发送初始 snapshot");
        }

        // Spawn Snapshot 循环
        self.snapshot_handle = Some(self.spawn_snapshot_loop());

        // Spawn Tick 循环（根据 SimMode 选择不同的 tick 方法）
        self.tick_handle = Some(self.spawn_tick_loop());

        // P2P 模式：连接种子节点并订阅 topic
        #[cfg(feature = "p2p")]
        {
            if let SimMode::P2P { .. } = &self.config.mode {
                self.init_p2p_network().await;
            }
        }

        tracing::info!(
            "[Simulation] 模拟已启动 [{} Agent（{} LLM + {} NPC）]",
            agent_ids.len() + npc_ids.len(),
            agent_ids.len(),
            npc_ids.len()
        );
    }

    /// 获取 Snapshot sender（供外部 clone）
    pub fn snapshot_sender(&self) -> Sender<WorldSnapshot> {
        self.snapshot_tx.clone()
    }

    /// 获取 Delta sender（供外部 clone）
    pub fn delta_sender(&self) -> Sender<Delta> {
        self.delta_tx.clone()
    }

    /// 获取 Narrative sender（供外部 clone）
    pub fn narrative_sender(&self) -> Sender<NarrativeEvent> {
        self.narrative_tx.clone()
    }

    /// 暂停模拟
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        tracing::info!("[Simulation] 模拟已暂停");
    }

    /// 恢复模拟
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        tracing::info!("[Simulation] 模拟已恢复");
    }

    /// 切换暂停状态
    pub fn toggle_pause(&self) {
        let current = self.is_paused.load(Ordering::SeqCst);
        self.is_paused.store(!current, Ordering::SeqCst);
        tracing::info!("[Simulation] 暂停状态 = {}", !current);
    }

    /// 获取暂停状态
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// 注入偏好（外部引导 Agent）
    pub async fn inject_preference(&self, agent_id: String, key: String, boost: f32, duration_ticks: u32) {
        let aid = AgentId::new(agent_id.clone());
        let mut world = self.world.lock().await;
        if let Some(agent) = world.agents.get_mut(&aid) {
            agent.inject_preference(&key, boost, duration_ticks);
            tracing::info!(
                "[Simulation] 注入偏好成功: {:?} key={} boost={} duration={} ticks",
                aid, key, boost, duration_ticks
            );
        } else {
            tracing::warn!("[Simulation] 注入偏好失败: Agent {:?} 不存在", aid);
        }
    }

    /// 设置 tick 间隔
    pub async fn set_tick_interval(&self, seconds: f32) {
        let mut world = self.world.lock().await;
        world.tick_interval = seconds as u32;
        tracing::info!("[Simulation] tick_interval 已设置为 {}s", seconds);
    }

    // ===== 内部方法 =====

    /// Spawn Agent 决策循环
    fn spawn_agent_loop(&self, agent_id: AgentId, is_npc: bool) -> JoinHandle<()> {
        let world = self.world.clone();
        let pipeline = self.pipeline.clone();
        let delta_tx = self.delta_tx.clone();
        let narrative_tx = self.narrative_tx.clone();
        let is_paused = self.is_paused.clone();
        let interval_secs = if is_npc {
            self.config.npc_decision_interval_secs
        } else {
            self.config.player_decision_interval_secs
        };
        let vision_radius = self.config.vision_radius;

        tokio::spawn(async move {
            super::agent_loop::run_agent_loop(
                world,
                agent_id,
                pipeline,
                delta_tx,
                narrative_tx,
                is_npc,
                interval_secs as u32,
                vision_radius,
                is_paused,
            ).await;
        })
    }

    /// 创建 NPC Agent（异步）
    async fn create_npc_agents(&self) -> Vec<AgentId> {
        super::npc::create_npc_agents(&self.world, &self.config).await
    }

    /// Spawn Snapshot 循环
    fn spawn_snapshot_loop(&self) -> JoinHandle<()> {
        let world = self.world.clone();
        let snapshot_tx = self.snapshot_tx.clone();
        let is_paused = self.is_paused.clone();

        tokio::spawn(async move {
            super::snapshot_loop::run_snapshot_loop(snapshot_tx, world, is_paused).await;
        })
    }

    /// Spawn Tick 循环
    fn spawn_tick_loop(&self) -> JoinHandle<()> {
        let world = self.world.clone();
        let is_paused = self.is_paused.clone();
        let tick_interval_secs = self.config.tick_interval_secs;
        let is_p2p_mode = matches!(self.config.mode, super::config::SimMode::P2P { .. });

        tokio::spawn(async move {
            super::tick_loop::run_tick_loop_with_mode(world, is_paused, tick_interval_secs, is_p2p_mode).await;
        })
    }

    /// 处理远程 Delta（P2P 模式）
    ///
    /// 从 P2PMessageHandler 处理接收到的远程 Delta
    pub async fn handle_remote_delta(&mut self, envelope: &super::DeltaEnvelope) {
        if let Some(ref mut handler) = self.p2p_handler {
            // 使用 handler 处理（包含回环过滤）
            let current_tick;
            {
                let world = self.world.lock().await;
                current_tick = world.tick;
            }
            handler.handle(envelope, current_tick);

            // 同步更新 World 的 remote_agents
            {
                let mut world = self.world.lock().await;
                world.apply_remote_delta(envelope, current_tick);
            }
        }
    }

    /// 获取 P2P handler（供外部消费网络消息）
    pub fn p2p_handler(&self) -> Option<&P2PMessageHandler> {
        self.p2p_handler.as_ref()
    }

    // ===== P2P 网络集成 =====

    /// 初始化 P2P 网络：连接种子节点、订阅 topic
    #[cfg(feature = "p2p")]
    async fn init_p2p_network(&mut self) {
        let transport = match self.transport.as_ref() {
            Some(t) => t,
            None => {
                tracing::warn!("[P2P] 传输层未创建，跳过网络初始化");
                return;
            }
        };

        // 连接种子节点
        if let Some(ref seed_peer) = self.config.seed_peer {
            tracing::info!("[P2P] 连接种子节点: {}", seed_peer);
            if let Err(e) = transport.connect_to_seed(seed_peer).await {
                tracing::warn!("[P2P] 连接种子节点失败: {:?}", e);
            } else {
                tracing::info!("[P2P] 种子节点连接成功");
            }
        } else {
            tracing::info!("[P2P] 未配置种子节点，等待对等点发现");
        }

        // 订阅世界事件 topic
        if let Some(topic_mgr) = self.region_topic_manager.as_mut() {
            if let Err(e) = topic_mgr.subscribe_world_events(transport).await {
                tracing::warn!("[P2P] 订阅世界事件 topic 失败: {:?}", e);
            } else {
                tracing::info!("[P2P] 已订阅世界事件 topic");
            }
        }

        // 订阅 region_0 topic（初始区域）
        if let Some(_topic_mgr) = self.region_topic_manager.as_mut() {
            let region_topic = agentora_network::gossip::RegionTopicManager::topic_name(0);
            let region_topic_ref: &str = &region_topic;
            // 注意：这里需要一个临时的 handler，实际消息由网络循环处理
            use agentora_network::NullMessageHandler;
            if let Err(e) = transport.subscribe(region_topic_ref, Box::new(NullMessageHandler)).await {
                tracing::warn!("[P2P] 订阅 region_0 topic 失败: {:?}", e);
            } else {
                tracing::info!("[P2P] 已订阅 region_0 topic");
            }
        }

        // 发布本地 peer 信息
        if let Some(ref peer_id) = self.local_peer_id {
            let peer_info = NetworkMessage::PeerInfo {
                peer_id: peer_id.clone(),
                position: (0, 0), // 初始位置
            };
            // 通过 region_0 发布 peer 信息
            let region_topic = agentora_network::gossip::RegionTopicManager::topic_name(0);
            if let Err(e) = transport.publish(&region_topic, &peer_info.to_bytes()).await {
                tracing::warn!("[P2P] 发布 peer 信息失败: {:?}", e);
            }
        }

        tracing::info!("[P2P] 网络初始化完成");
    }

    /// P2P 网络消息循环
    ///
    /// 在独立 task 中运行，消费 libp2p 消息并分发给 P2PMessageHandler
    #[cfg(feature = "p2p")]
    async fn run_p2p_network_loop(
        mut message_rx: Option<tokio::sync::mpsc::Receiver<NetworkMessage>>,
        delta_tx: Sender<Delta>,
        narrative_tx: Sender<NarrativeEvent>,
        local_peer_id: &str,
    ) {
        use serde_json;
        use crate::simulation::DeltaEnvelope;

        tracing::info!("[P2P-Network] 网络消息循环已启动 [peer={}]", local_peer_id);

        let mut rx = match message_rx.take() {
            Some(rx) => rx,
            None => {
                tracing::error!("[P2P-Network] 消息接收器不可用，网络循环退出");
                return;
            }
        };

        loop {
            match rx.recv().await {
                Some(msg) => {
                    match msg {
                        NetworkMessage::AgentDelta(delta_msg) => {
                            // 过滤本地回环
                            if delta_msg.source_peer_id == local_peer_id {
                                tracing::trace!("[P2P-Network] 过滤本地回环 delta");
                                continue;
                            }

                            // 解析 delta JSON 并构建 DeltaEnvelope
                            if let Ok(delta) = serde_json::from_value::<Delta>(delta_msg.delta_json.clone()) {
                                let envelope = DeltaEnvelope::from_remote(
                                    delta,
                                    delta_msg.source_peer_id.clone(),
                                    delta_msg.tick,
                                );

                                // 转发到本地 delta 通道（触发渲染更新）
                                if let Err(e) = delta_tx.send(envelope.delta.clone()) {
                                    tracing::error!("[P2P-Network] delta 转发失败: {:?}", e);
                                }

                                tracing::debug!("[P2P-Network] 收到远程 delta: {} from peer={}",
                                    envelope.delta.event_type(), delta_msg.source_peer_id);
                            } else {
                                tracing::warn!("[P2P-Network] delta JSON 解析失败: {:?}", delta_msg.delta_json);
                            }
                        }
                        NetworkMessage::Narrative(narrative_msg) => {
                            // 过滤本地回环
                            if narrative_msg.source_peer_id == local_peer_id {
                                tracing::trace!("[P2P-Network] 过滤本地回环 narrative");
                                continue;
                            }

                            // 解析 narrative JSON 并构建 NarrativeEvent
                            if let Ok(event) = serde_json::from_value::<NarrativeEvent>(narrative_msg.narrative_json.clone()) {
                                // 转发到本地 narrative 通道
                                if let Err(e) = narrative_tx.send(event) {
                                    tracing::error!("[P2P-Network] narrative 转发失败: {:?}", e);
                                }

                                tracing::debug!("[P2P-Network] 收到远程 narrative from peer={}",
                                    narrative_msg.source_peer_id);
                            } else {
                                tracing::warn!("[P2P-Network] narrative JSON 解析失败: {:?}", narrative_msg.narrative_json);
                            }
                        }
                        _ => {
                            // 其他消息类型暂不处理
                            tracing::trace!("[P2P-Network] 收到未处理消息类型，跳过");
                        }
                    }
                }
                None => {
                    tracing::warn!("[P2P-Network] 消息通道已关闭，网络循环退出");
                    break;
                }
            }
        }
    }

    /// 通过 P2P 发布 Delta
    #[cfg(feature = "p2p")]
    pub async fn publish_delta_p2p(&self, delta: &Delta, tick: u64, region_id: u32) {
        let transport = match self.transport.as_ref() {
            Some(t) => t,
            None => return,
        };

        let peer_id = match &self.local_peer_id {
            Some(id) => id.clone(),
            None => return,
        };

        let delta_msg = NetworkMessage::AgentDelta(AgentDeltaMessage {
            delta_json: delta.for_broadcast(),
            source_peer_id: peer_id,
            tick,
        });

        let topic = agentora_network::gossip::RegionTopicManager::topic_name(region_id);
        if let Err(e) = transport.publish(&topic, &delta_msg.to_bytes()).await {
            tracing::warn!("[P2P] delta 发布失败 [topic={}]: {:?}", topic, e);
        }
    }

    /// 通过 P2P 发布 Narrative
    #[cfg(feature = "p2p")]
    pub async fn publish_narrative_p2p(&self, event: &NarrativeEvent, tick: u64, channel: &str) {
        let transport = match self.transport.as_ref() {
            Some(t) => t,
            None => return,
        };

        let peer_id = match &self.local_peer_id {
            Some(id) => id.clone(),
            None => return,
        };

        // 将 NarrativeEvent 转换为可序列化的字典
        let narrative_dict = serde_json::json!({
            "tick": event.tick,
            "agent_id": event.agent_id,
            "agent_name": event.agent_name,
            "event_type": event.event_type,
            "description": event.description,
            "color_code": event.color_code,
        });

        let narrative_msg = NetworkMessage::Narrative(NarrativeMessage {
            narrative_json: narrative_dict,
            source_peer_id: peer_id,
            tick,
            channel: channel.to_string(),
        });

        // 根据频道选择 topic
        let topic = match channel {
            "world" => agentora_network::gossip::RegionTopicManager::world_topic_name().to_string(),
            "nearby" => agentora_network::gossip::RegionTopicManager::topic_name(0), // 初始区域
            _ => return, // local 频道不广播
        };

        if let Err(e) = transport.publish(&topic, &narrative_msg.to_bytes()).await {
            tracing::warn!("[P2P] narrative 发布失败 [topic={}]: {:?}", topic, e);
        } else {
            tracing::debug!("[P2P] narrative 已发布 [topic={} channel={}]", topic, channel);
        }
    }
}