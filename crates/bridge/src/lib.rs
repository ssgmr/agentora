//! Agentora Godot GDExtension 桥接
//!
//! Tokio 运行时管理、mpsc Channel 桥接、WorldSnapshot 序列化、
//! Agent 独立心跳循环、增量事件推送。

use godot::prelude::*;
use godot::init::ExtensionLibrary;
use godot::classes::{Node, INode};
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::Arc;
use tokio::sync::Mutex;

// 核心引擎类型
use agentora_core::{World, WorldSeed, Agent, Position, WorldSnapshot, AgentId, Action};
use agentora_core::snapshot::{AgentSnapshot, NarrativeEvent as CoreNarrativeEvent};
use agentora_core::decision::{DecisionPipeline, Spark};
use agentora_core::rule_engine::WorldState;
use agentora_core::vision::scan_vision;
use agentora_core::memory::MemoryEvent;
use std::collections::HashMap;

// AI 类型
use agentora_ai::{load_llm_config, OpenAiProvider, FallbackChain, LlmProvider};

// ===== AgentDelta 增量事件 =====

/// Agent 增量事件类型
#[derive(Debug, Clone)]
pub enum AgentDelta {
    /// Agent 移动或状态变化
    AgentMoved {
        id: String,
        name: String,
        position: (u32, u32),
        health: u32,
        max_health: u32,
        is_alive: bool,
        age: u32,
        motivation: [f32; 6],
    },
    /// Agent 死亡
    AgentDied {
        id: String,
        name: String,
        position: (u32, u32),
        age: u32,
    },
    /// 新 Agent 诞生
    AgentSpawned {
        id: String,
        name: String,
        position: (u32, u32),
        health: u32,
        max_health: u32,
        motivation: [f32; 6],
    },

    // ===== Tier 2 新增 =====
    /// 建筑创建
    StructureCreated {
        x: u32,
        y: u32,
        structure_type: String,
        owner_id: String,
    },
    /// 建筑销毁
    StructureDestroyed {
        x: u32,
        y: u32,
        structure_type: String,
    },
    /// 资源变化
    ResourceChanged {
        x: u32,
        y: u32,
        resource_type: String,
        amount: u32,
    },
    /// 交易完成
    TradeCompleted {
        from_id: String,
        to_id: String,
        items: String,
    },
    /// 联盟建立
    AllianceFormed {
        id1: String,
        id2: String,
    },
    /// 联盟破裂
    AllianceBroken {
        id1: String,
        id2: String,
        reason: String,
    },

    // ===== Tier 2.5 新增：生存+建筑+压力+里程碑 =====
    /// 营地治愈
    HealedByCamp {
        agent_id: String,
        agent_name: String,
        hp_restored: u32,
    },
    /// 生存状态警告
    SurvivalWarning {
        agent_id: String,
        agent_name: String,
        satiety: u32,
        hydration: u32,
        hp: u32,
    },
    /// 里程碑达成
    MilestoneReached {
        name: String,
        display_name: String,
        tick: u64,
    },
    /// 压力事件开始
    PressureStarted {
        pressure_type: String,
        description: String,
        duration: u32,
    },
    /// 压力事件结束
    PressureEnded {
        pressure_type: String,
        description: String,
    },
}

// ===== 模拟配置 =====

/// 模拟配置 — 从 config/sim.toml 加载，支持热改
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// LLM 驱动 Agent 数量（走完整决策管道，耗时较长）
    pub initial_agent_count: usize,
    /// NPC 数量（规则引擎快速决策，不阻塞）
    pub npc_count: usize,
    /// NPC 决策间隔（秒）
    pub npc_decision_interval_secs: u64,
    /// 玩家 Agent 决策间隔（秒）
    pub player_decision_interval_secs: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            initial_agent_count: 3,
            npc_count: 3,
            npc_decision_interval_secs: 1,
            player_decision_interval_secs: 2,
        }
    }
}

impl SimConfig {
    /// 从 toml 文件加载配置，失败则使用默认值
    fn load(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                match toml::from_str::<toml::Value>(&content) {
                    Ok(table) => {
                        let mut cfg = Self::default();
                        if let Some(sim) = table.get("simulation") {
                            if let Some(v) = sim.get("initial_agent_count").and_then(|v| v.as_integer()) {
                                cfg.initial_agent_count = v as usize;
                            }
                            if let Some(v) = sim.get("npc_count").and_then(|v| v.as_integer()) {
                                cfg.npc_count = v as usize;
                            }
                            if let Some(v) = sim.get("npc_decision_interval_secs").and_then(|v| v.as_integer()) {
                                cfg.npc_decision_interval_secs = v as u64;
                            }
                            if let Some(v) = sim.get("player_decision_interval_secs").and_then(|v| v.as_integer()) {
                                cfg.player_decision_interval_secs = v as u64;
                            }
                        }
                        println!("[SimConfig] 配置加载成功 [agents={} npc={} npc_interval={}s player_interval={}s]",
                            cfg.initial_agent_count, cfg.npc_count, cfg.npc_decision_interval_secs, cfg.player_decision_interval_secs);
                        cfg
                    }
                    Err(e) => {
                        println!("[SimConfig] sim.toml 解析失败 ({}), 使用默认配置", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                println!("[SimConfig] sim.toml 未找到 ({}), 使用默认配置", e);
                Self::default()
            }
        }
    }
}

/// 模拟命令
#[derive(Debug, Clone)]
pub enum SimCommand {
    Start,
    Pause,
    SetTickInterval { seconds: f32 },
    AdjustMotivation {
        agent_id: String,
        dimension: usize,
        value: f32,
    },
    InjectPreference {
        agent_id: String,
        dimension: usize,
        boost: f32,
        duration_ticks: u32,
    },
}

/// 叙事事件（推送至 Godot 叙事流）
#[derive(Debug, Clone)]
pub struct NarrativeEvent {
    pub tick: u64,
    pub agent_id: String,
    pub agent_name: String,
    pub event_type: String,
    pub description: String,
    pub color_code: String,
}

/// SimulationBridge GDExtension 节点
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SimulationBridge {
    base: Base<Node>,
    command_sender: Option<Sender<SimCommand>>,
    snapshot_receiver: Option<Receiver<WorldSnapshot>>,
    delta_receiver: Option<Receiver<AgentDelta>>,
    narrative_receiver: Option<Receiver<NarrativeEvent>>,
    current_tick: i64,
    #[var]
    is_paused: bool,
    is_running: bool,
    last_snapshot: Option<WorldSnapshot>,
    #[var]
    selected_agent_id: GString,
}

#[godot_api]
impl INode for SimulationBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            command_sender: None,
            snapshot_receiver: None,
            delta_receiver: None,
            narrative_receiver: None,
            current_tick: 0,
            is_paused: false,
            is_running: false,
            last_snapshot: None,
            selected_agent_id: GString::new(),
        }
    }

    fn ready(&mut self) {
        godot::global::print(&[Variant::from("SimulationBridge: 初始化完成")]);
        self.start_simulation();
    }

    fn physics_process(&mut self, _delta: f64) {
        // 1. 优先处理 delta（实时）
        if let Some(receiver) = &self.delta_receiver {
            let mut processed = 0;
            let mut deltas = Vec::new();
            while let Ok(delta) = receiver.try_recv() {
                deltas.push(delta);
                processed += 1;
                if processed >= 100 { break; }
            }
            if !deltas.is_empty() {
                for delta in deltas {
                    let delta_dict = Self::delta_to_dict(&delta);
                    self.base_mut().emit_signal("agent_delta", &[delta_dict.to_variant()]);
                }
            }
        }

        // 2. 处理叙事事件
        if let Some(receiver) = &self.narrative_receiver {
            let mut events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
                if events.len() >= 50 { break; }
            }
            for event in events {
                let mut dict: Dictionary<GString, Variant> = Dictionary::new();
                dict.set("tick", &(Variant::from(event.tick as i64)));
                dict.set("agent_id", &event.agent_id.to_variant());
                dict.set("agent_name", &event.agent_name.to_variant());
                dict.set("event_type", &event.event_type.to_variant());
                dict.set("description", &event.description.to_variant());
                dict.set("color", &event.color_code.to_variant());
                self.base_mut().emit_signal("narrative_event", &[dict.to_variant()]);
            }
        }

        // 3. 再处理 snapshot（一致性校验）
        if let Some(receiver) = &self.snapshot_receiver {
            if let Ok(snapshot) = receiver.try_recv() {
                self.current_tick = snapshot.tick as i64;
                self.last_snapshot = Some(snapshot.clone());

                let mut snapshot_dict: Dictionary<GString, Variant> = Dictionary::new();
                snapshot_dict.set("tick", &(Variant::from(snapshot.tick as i64)));
                let mut agents_dict: Dictionary<GString, Variant> = Dictionary::new();
                for agent in &snapshot.agents {
                    let agent_data = Self::agent_to_dict(agent);
                    agents_dict.set(agent.id.as_str(), &agent_data);
                }
                snapshot_dict.set("agents", &agents_dict.to_variant());
                self.base_mut().emit_signal("world_updated", &[snapshot_dict.to_variant()]);
            }
        }
    }
}

#[godot_api]
impl SimulationBridge {
    #[signal]
    fn world_updated(snapshot: Variant);

    #[signal]
    fn agent_delta(delta: Variant);

    #[signal]
    fn agent_selected(agent_id: GString);

    #[signal]
    fn narrative_event(event: Variant);

    /// 将 AgentDelta 转为 GDScript Dictionary
    fn delta_to_dict(delta: &AgentDelta) -> Variant {
        let mut dict: Dictionary<GString, Variant> = Dictionary::new();
        match delta {
            AgentDelta::AgentMoved { id, name, position, health, max_health, is_alive, age, motivation } => {
                dict.set("type", &"agent_moved".to_variant());
                dict.set("id", &id.to_variant());
                dict.set("name", &name.to_variant());
                let pos = Vector2::new(position.0 as f32, position.1 as f32);
                dict.set("position", &pos.to_variant());
                dict.set("health", &(Variant::from(*health as i64)));
                dict.set("max_health", &(Variant::from(*max_health as i64)));
                dict.set("is_alive", &is_alive.to_variant());
                dict.set("age", &(Variant::from(*age as i64)));
                let mut mot_arr: Array<f32> = Array::new();
                for &v in motivation { mot_arr.push(v); }
                dict.set("motivation", &mot_arr.to_variant());
            }
            AgentDelta::AgentDied { id, name, position, age } => {
                dict.set("type", &"agent_died".to_variant());
                dict.set("id", &id.to_variant());
                dict.set("name", &name.to_variant());
                let pos = Vector2::new(position.0 as f32, position.1 as f32);
                dict.set("position", &pos.to_variant());
                dict.set("age", &(Variant::from(*age as i64)));
            }
            AgentDelta::AgentSpawned { id, name, position, health, max_health, motivation } => {
                dict.set("type", &"agent_spawned".to_variant());
                dict.set("id", &id.to_variant());
                dict.set("name", &name.to_variant());
                let pos = Vector2::new(position.0 as f32, position.1 as f32);
                dict.set("position", &pos.to_variant());
                dict.set("health", &(Variant::from(*health as i64)));
                dict.set("max_health", &(Variant::from(*max_health as i64)));
                let mut mot_arr: Array<f32> = Array::new();
                for &v in motivation { mot_arr.push(v); }
                dict.set("motivation", &mot_arr.to_variant());
            }

            // Tier 2 新增
            AgentDelta::StructureCreated { x, y, structure_type, owner_id } => {
                dict.set("type", &"structure_created".to_variant());
                dict.set("structure_type", &structure_type.to_variant());
                dict.set("owner_id", &owner_id.to_variant());
                let pos = Vector2::new(*x as f32, *y as f32);
                dict.set("position", &pos.to_variant());
            }
            AgentDelta::StructureDestroyed { x, y, structure_type } => {
                dict.set("type", &"structure_destroyed".to_variant());
                dict.set("structure_type", &structure_type.to_variant());
                let pos = Vector2::new(*x as f32, *y as f32);
                dict.set("position", &pos.to_variant());
            }
            AgentDelta::ResourceChanged { x, y, resource_type, amount } => {
                dict.set("type", &"resource_changed".to_variant());
                dict.set("resource_type", &resource_type.to_variant());
                dict.set("amount", &(Variant::from(*amount as i64)));
                let pos = Vector2::new(*x as f32, *y as f32);
                dict.set("position", &pos.to_variant());
            }
            AgentDelta::TradeCompleted { from_id, to_id, items } => {
                dict.set("type", &"trade_completed".to_variant());
                dict.set("from_id", &from_id.to_variant());
                dict.set("to_id", &to_id.to_variant());
                dict.set("items", &items.to_variant());
            }
            AgentDelta::AllianceFormed { id1, id2 } => {
                dict.set("type", &"alliance_formed".to_variant());
                dict.set("id1", &id1.to_variant());
                dict.set("id2", &id2.to_variant());
            }
            AgentDelta::AllianceBroken { id1, id2, reason } => {
                dict.set("type", &"alliance_broken".to_variant());
                dict.set("id1", &id1.to_variant());
                dict.set("id2", &id2.to_variant());
                dict.set("reason", &reason.to_variant());
            }

            // Tier 2.5 新增
            AgentDelta::HealedByCamp { agent_id, agent_name, hp_restored } => {
                dict.set("type", &"healed_by_camp".to_variant());
                dict.set("agent_id", &agent_id.to_variant());
                dict.set("agent_name", &agent_name.to_variant());
                dict.set("hp_restored", &(Variant::from(*hp_restored as i64)));
            }
            AgentDelta::SurvivalWarning { agent_id, agent_name, satiety, hydration, hp } => {
                dict.set("type", &"survival_warning".to_variant());
                dict.set("agent_id", &agent_id.to_variant());
                dict.set("agent_name", &agent_name.to_variant());
                dict.set("satiety", &(Variant::from(*satiety as i64)));
                dict.set("hydration", &(Variant::from(*hydration as i64)));
                dict.set("hp", &(Variant::from(*hp as i64)));
            }
            AgentDelta::MilestoneReached { name, display_name, tick } => {
                dict.set("type", &"milestone_reached".to_variant());
                dict.set("name", &name.to_variant());
                dict.set("display_name", &display_name.to_variant());
                dict.set("tick", &(Variant::from(*tick as i64)));
            }
            AgentDelta::PressureStarted { pressure_type, description, duration } => {
                dict.set("type", &"pressure_started".to_variant());
                dict.set("pressure_type", &pressure_type.to_variant());
                dict.set("description", &description.to_variant());
                dict.set("duration", &(Variant::from(*duration as i64)));
            }
            AgentDelta::PressureEnded { pressure_type, description } => {
                dict.set("type", &"pressure_ended".to_variant());
                dict.set("pressure_type", &pressure_type.to_variant());
                dict.set("description", &description.to_variant());
            }
        }
        dict.to_variant()
    }

    /// 将 AgentSnapshot 转为 GDScript Dictionary
    fn agent_to_dict(agent: &AgentSnapshot) -> Variant {
        let mut dict: Dictionary<GString, Variant> = Dictionary::new();
        dict.set("id", &agent.id.clone().to_variant());
        dict.set("name", &agent.name.clone().to_variant());
        dict.set("health", &(Variant::from(agent.health as i64)));
        dict.set("max_health", &(Variant::from(agent.max_health as i64)));
        dict.set("satiety", &(Variant::from(agent.satiety as i64)));
        dict.set("hydration", &(Variant::from(agent.hydration as i64)));
        dict.set("is_alive", &agent.is_alive.to_variant());
        dict.set("age", &(Variant::from(agent.age as i64)));
        dict.set("current_action", &agent.current_action.clone().to_variant());
        let pos = Vector2::new(agent.position.0 as f32, agent.position.1 as f32);
        dict.set("position", &pos.to_variant());
        let mut motivation_arr: Array<f32> = Array::new();
        for &v in &agent.motivation {
            motivation_arr.push(v);
        }
        dict.set("motivation", &motivation_arr.to_variant());
        // 背包摘要
        let mut inv_dict: Dictionary<GString, Variant> = Dictionary::new();
        for (k, v) in &agent.inventory_summary {
            inv_dict.set(k, &(Variant::from(*v as i64)));
        }
        dict.set("inventory_summary", &inv_dict.to_variant());
        dict.to_variant()
    }

    #[func]
    fn start_simulation(&mut self) {
        if self.is_running {
            godot::global::print(&[Variant::from("SimulationBridge: 模拟已在运行")]);
            return;
        }
        godot::global::print(&[Variant::from("SimulationBridge: 启动模拟...")]);

        let (snapshot_tx, snapshot_rx) = mpsc::channel::<WorldSnapshot>();
        let (delta_tx, delta_rx) = mpsc::channel::<AgentDelta>();
        let (narrative_tx, narrative_rx) = mpsc::channel::<NarrativeEvent>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<SimCommand>();

        self.snapshot_receiver = Some(snapshot_rx);
        self.delta_receiver = Some(delta_rx);
        self.narrative_receiver = Some(narrative_rx);
        self.command_sender = Some(cmd_tx);
        self.is_running = true;
        self.is_paused = false;

        let llm_provider = Self::create_llm_provider();

        std::thread::spawn(move || {
            run_simulation(snapshot_tx, delta_tx, narrative_tx, cmd_rx, llm_provider);
        });

        godot::global::print(&[Variant::from("SimulationBridge: 模拟已启动（事件驱动模式）")]);
    }

    /// GDScript 别名: start() -> start_simulation()
    #[func]
    fn start(&mut self) {
        self.start_simulation();
    }

    /// GDScript 别名: pause() -> toggle_pause()
    #[func]
    fn pause(&mut self) {
        self.toggle_pause();
    }

    fn create_llm_provider() -> Option<Box<dyn LlmProvider>> {
        let config_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/llm.toml");
        match load_llm_config(config_path) {
            Ok(config) => {
                let model = config.primary.model.clone();
                godot::global::print(&[Variant::from(format!(
                    "SimulationBridge: LLM 配置加载成功，model={}",
                    model
                ))]);

                let openai = OpenAiProvider::new(
                    config.primary.api_base,
                    config.primary.api_key,
                    config.primary.model,
                ).with_timeout(config.primary.timeout_seconds);

                let fallback = FallbackChain::new(vec![Box::new(openai)]);
                godot::global::print(&[Variant::from("SimulationBridge: LLM Provider 链已创建")]);
                Some(Box::new(fallback))
            }
            Err(e) => {
                godot::global::print(&[Variant::from(format!(
                    "SimulationBridge: LLM 配置加载失败: {}，将使用规则引擎兜底",
                    e
                ))]);
                None
            }
        }
    }

    #[func]
    fn get_tick(&self) -> i64 {
        self.current_tick
    }

    #[func]
    fn get_agent_count(&self) -> i64 {
        match &self.last_snapshot {
            Some(snapshot) => snapshot.agents.len() as i64,
            None => 5,
        }
    }

    #[func]
    fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        if let Some(tx) = &self.command_sender {
            let cmd = if self.is_paused {
                SimCommand::Pause
            } else {
                SimCommand::Start
            };
            let _ = tx.send(cmd);
        }
        godot::global::print(&[Variant::from(format!("SimulationBridge: 暂停状态 = {}", self.is_paused))]);
    }

    #[func]
    fn adjust_motivation(&self, agent_id: String, dimension: i32, value: f32) {
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::AdjustMotivation {
                agent_id,
                dimension: dimension as usize,
                value,
            });
        }
    }

    #[func]
    fn inject_preference(&self, agent_id: String, dimension: i32, boost: f32, duration: i32) {
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::InjectPreference {
                agent_id,
                dimension: dimension as usize,
                boost,
                duration_ticks: duration as u32,
            });
        }
    }

    #[func]
    fn set_tick_interval(&self, seconds: f32) {
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::SetTickInterval { seconds });
        }
    }

    #[func]
    fn get_agent_data(&self, agent_id: String) -> Variant {
        let mut dict: Dictionary<GString, Variant> = Dictionary::new();
        if let Some(snapshot) = &self.last_snapshot {
            if let Some(agent) = snapshot.agents.iter().find(|a| a.id == agent_id) {
                dict.set("id", &agent.id.clone().to_variant());
                dict.set("name", &agent.name.clone().to_variant());
                dict.set("health", &(Variant::from(agent.health as i64)));
                dict.set("max_health", &(Variant::from(agent.max_health as i64)));
                dict.set("is_alive", &agent.is_alive.to_variant());
                dict.set("age", &(Variant::from(agent.age as i64)));
                dict.set("current_action", &agent.current_action.clone().to_variant());
                let pos = Vector2::new(agent.position.0 as f32, agent.position.1 as f32);
                dict.set("position", &pos.to_variant());
                let mut motivation_arr: Array<f32> = Array::new();
                for &v in &agent.motivation {
                    motivation_arr.push(v);
                }
                dict.set("motivation", &motivation_arr.to_variant());
                let mut inv_dict: Dictionary<GString, Variant> = Dictionary::new();
                for (k, v) in &agent.inventory_summary {
                    inv_dict.set(k.as_str(), &Variant::from(*v as i64));
                }
                dict.set("inventory", &inv_dict.to_variant());
            }
        }
        dict.to_variant()
    }

    #[func]
    fn select_agent(&mut self, agent_id: GString) {
        self.selected_agent_id = agent_id.clone();
        self.base_mut().emit_signal("agent_selected", &[agent_id.to_variant()]);
    }
}

fn run_simulation(
    snapshot_tx: Sender<WorldSnapshot>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    llm_provider: Option<Box<dyn LlmProvider>>,
) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_simulation_async(snapshot_tx, delta_tx, narrative_tx, cmd_rx, llm_provider).await;
    });
}

async fn run_simulation_async(
    snapshot_tx: Sender<WorldSnapshot>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    llm_provider: Option<Box<dyn LlmProvider>>,
) {
    // 加载模拟配置（相对于项目根目录，Godot 工作目录为 client/）
    let sim_config = SimConfig::load("../config/sim.toml");

    let seed = WorldSeed {
        map_size: [256, 256],
        terrain_ratio: std::collections::HashMap::from([
            ("plains".to_string(), 0.5),
            ("forest".to_string(), 0.25),
            ("mountain".to_string(), 0.1),
            ("water".to_string(), 0.1),
            ("desert".to_string(), 0.05),
        ]),
        resource_density: 0.15,
        region_size: 16,
        initial_agents: sim_config.initial_agent_count as u32,
        motivation_templates: std::collections::HashMap::from([
            ("gatherer".to_string(), [0.8, 0.4, 0.3, 0.2, 0.3, 0.2]),
            ("trader".to_string(), [0.5, 0.8, 0.4, 0.3, 0.7, 0.3]),
        ]),
        spawn_strategy: "scattered".to_string(),
        seed_peers: vec![],
        pressure_config: agentora_core::seed::PressureConfig::default(),
    };

    let world = World::new(&seed);

    let pipeline = if let Some(provider) = llm_provider {
        DecisionPipeline::new().with_llm_provider(provider)
    } else {
        DecisionPipeline::new()
    };

    // 共享 World（Arc + Mutex），Agent 决策 task 和 Apply 循环共享访问
    let world_arc = Arc::new(Mutex::new(world));
    let pipeline_arc = Arc::new(pipeline);

    // World::new 已经通过 generate_agents 创建了初始 Agent

    // 动作通道：Agent 决策完成后发送 (AgentId, Action) 到 Apply 循环
    let (action_tx, action_rx) = tokio::sync::mpsc::channel::<(AgentId, Action)>(1024);

    // 初始化 Agent 并 spawn 决策 task
    let agent_ids: Vec<AgentId>;
    {
        let world = world_arc.lock().await;
        agent_ids = world.agents.keys().cloned().collect();
    }

    let mut _agent_handles = Vec::new();
    for agent_id in &agent_ids {
        let w = world_arc.clone();
        let p = pipeline_arc.clone();
        let tx = action_tx.clone();
        let aid = agent_id.clone();
        let interval = sim_config.player_decision_interval_secs;
        let handle = tokio::spawn(async move {
            run_agent_loop(w, aid, p, tx, false, interval as u32).await;
        });
        _agent_handles.push(handle);
    }

    // 创建 NPC Agent 并 spawn 决策 task
    let npc_ids = create_npc_agents(&world_arc, &sim_config).await;
    for npc_id in &npc_ids {
        let w = world_arc.clone();
        let p = pipeline_arc.clone();
        let tx = action_tx.clone();
        let aid = npc_id.clone();
        let interval = sim_config.npc_decision_interval_secs;
        let handle = tokio::spawn(async move {
            run_agent_loop(w, aid, p, tx, true, interval as u32).await;
        });
        _agent_handles.push(handle);
    }

    let all_agent_count = agent_ids.len() + npc_ids.len();
    println!("[SimulationBridge] 世界已创建，{} 个 Agent（{} LLM + {} NPC）",
        all_agent_count, agent_ids.len(), npc_ids.len());

    // Apply 循环：串行应用动作并发 delta + narrative
    let delta_tx_clone = delta_tx.clone();
    let narrative_tx_clone = narrative_tx.clone();
    let w_apply = world_arc.clone();
    let _apply_handle = tokio::spawn(async move {
        run_apply_loop(action_rx, w_apply, delta_tx_clone, narrative_tx_clone).await;
    });

    // 定期 snapshot 兜底循环
    let w_snap = world_arc.clone();
    let _snap_handle = tokio::spawn(async move {
        run_snapshot_loop(snapshot_tx, w_snap).await;
    });

    // 命令处理循环（无限循环，不会退出）
    let mut is_paused = false;
    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                SimCommand::Pause => {
                    is_paused = !is_paused;
                    println!("[模拟线程] 暂停状态 = {}", is_paused);
                }
                SimCommand::Start => {
                    is_paused = false;
                    println!("[模拟线程] 恢复运行");
                }
                SimCommand::SetTickInterval { seconds } => {
                    let mut world = world_arc.lock().await;
                    world.tick_interval = seconds as u32;
                }
                SimCommand::AdjustMotivation { agent_id, dimension, value } => {
                    let aid = AgentId::new(agent_id.clone());
                    let mut world = world_arc.lock().await;
                    if let Some(agent) = world.agents.get_mut(&aid) {
                        let current = agent.motivation.get(dimension);
                        let new_val = (current + (value - current) * 0.5).clamp(0.0, 1.0);
                        agent.motivation.set(dimension, new_val);
                        println!("[模拟线程] 调整 {:?} 动机[{}]={}", aid, dimension, value);
                    }
                }
                SimCommand::InjectPreference { agent_id, dimension, boost, duration_ticks } => {
                    let aid = AgentId::new(agent_id.clone());
                    let mut world = world_arc.lock().await;
                    if let Some(agent) = world.agents.get_mut(&aid) {
                        agent.inject_preference(dimension, boost, duration_ticks);
                        println!("[模拟线程] 注入偏好 {:?} dim={} boost={} duration={}",
                            aid, dimension, boost, duration_ticks);
                    }
                }
            }
        }

        if is_paused {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// NPC Agent 创建
async fn create_npc_agents(
    world_arc: &Arc<Mutex<World>>,
    config: &SimConfig,
) -> Vec<AgentId> {
    let mut ids = Vec::new();

    if config.npc_count == 0 {
        return ids;
    }

    let mut world = world_arc.lock().await;

    let npc_names = ["Explorer", "Miner", "Builder", "Trader", "Guard", "Scout", "Gatherer", "Hunter", "Farmer", "Nomad"];
    let templates = [
        [0.6, 0.3, 0.7, 0.4, 0.2, 0.3],  // Explorer
        [0.8, 0.2, 0.3, 0.2, 0.3, 0.2],  // Miner
        [0.5, 0.4, 0.5, 0.8, 0.3, 0.5],  // Builder
        [0.4, 0.8, 0.5, 0.3, 0.7, 0.3],  // Trader
        [0.6, 0.3, 0.4, 0.2, 0.8, 0.2],  // Guard
        [0.5, 0.3, 0.8, 0.3, 0.2, 0.4],  // Scout
        [0.8, 0.2, 0.3, 0.2, 0.2, 0.3],  // Gatherer
        [0.7, 0.3, 0.4, 0.3, 0.3, 0.2],  // Hunter
        [0.7, 0.4, 0.3, 0.2, 0.2, 0.5],  // Farmer
        [0.5, 0.5, 0.6, 0.3, 0.3, 0.3],  // Nomad
    ];

    // NPC spawn 位置（地图中心附近，确保相机能看到）
    let cx = 128u32;
    let cy = 128u32;
    let npc_positions = [
        (cx, cy), (cx + 5, cy), (cx - 5, cy), (cx, cy + 5), (cx, cy - 5),
        (cx + 10, cy + 10), (cx - 10, cy - 10), (cx + 10, cy - 10), (cx - 10, cy + 10), (cx + 15, cy),
    ];

    for i in 0..config.npc_count.min(npc_names.len()).min(npc_positions.len()) {
        let name = format!("[NPC]{}", npc_names[i]);
        let (mut x, mut y) = npc_positions[i];
        let template = templates[i];

        // 确保出生位置可通行，如果不可通行则找附近可通行位置
        let mut pos = Position::new(x, y);
        if !world.map.get_terrain(pos).is_passable() {
            // 在附近 5x5 范围内找可通行位置
            let mut found = false;
            for dx in 0..=5u32 {
                for dy in 0..=5u32 {
                    let nx = x.saturating_add(dx).min(255);
                    let ny = y.saturating_add(dy).min(255);
                    let trial = Position::new(nx, ny);
                    if world.map.get_terrain(trial).is_passable() {
                        x = nx;
                        y = ny;
                        found = true;
                        break;
                    }
                }
                if found { break; }
            }
        }

        let mut agent = Agent::new(
            AgentId::default(),
            name.clone(),
            Position::new(x, y),
        );
        agent.motivation = agentora_core::motivation::MotivationVector::from_array(template);

        let aid = agent.id.clone();
        world.insert_agent_at(aid.clone(), agent);
        ids.push(aid);
    }

    ids
}

/// Agent 独立决策循环
async fn run_agent_loop(
    world: Arc<Mutex<World>>,
    agent_id: AgentId,
    pipeline: Arc<DecisionPipeline>,
    action_tx: tokio::sync::mpsc::Sender<(AgentId, Action)>,
    is_npc: bool,
    interval_secs: u32,
) {
    println!("[AgentLoop] Agent {:?} 启动 (is_npc={}, interval={}s)", agent_id, is_npc, interval_secs);

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs as u64));
    // 跳过第一次立即触发
    interval.tick().await;

    loop {
        interval.tick().await;

        // 检查 Agent 是否存活
        let should_continue = {
            let w = world.lock().await;
            match w.agents.get(&agent_id) {
                Some(agent) => agent.is_alive,
                None => false,
            }
        };

        if !should_continue {
            println!("[AgentLoop] Agent {:?} 已死亡或不存在，退出循环", agent_id);
            break;
        }

        // 构建 WorldState（锁内纯计算 + 锁外 I/O）
        let (agent_clone, world_state) = {
            let w = world.lock().await;

            // scan_vision() → VisionScanResult  ← 纯计算，快
            let vision = scan_vision(&w, &agent_id, 5);

            // clone Agent 的必要字段
            let agent = match w.agents.get(&agent_id) {
                Some(a) => a.clone(),
                None => break,
            };

            // 将 VisionScanResult 映射到 WorldState
            let ws = WorldState {
                map_size: 256,
                agent_position: agent.position,
                agent_inventory: agent.inventory.iter().map(|(k, v)| {
                    let resource = match k.as_str() {
                        "iron" => agentora_core::types::ResourceType::Iron,
                        "food" => agentora_core::types::ResourceType::Food,
                        "wood" => agentora_core::types::ResourceType::Wood,
                        "water" => agentora_core::types::ResourceType::Water,
                        "stone" => agentora_core::types::ResourceType::Stone,
                        _ => agentora_core::types::ResourceType::Food,
                    };
                    (resource, *v)
                }).collect(),
                agent_satiety: agent.satiety,
                agent_hydration: agent.hydration,
                terrain_at: vision.terrain_at,
                existing_agents: w.agents.keys().cloned().collect(),
                resources_at: vision.resources_at,
                nearby_agents: vision.nearby_agents,
                active_pressures: w.pressure_pool.iter().map(|p| p.description.clone()).collect(),
            };

            (agent, ws)
        };

        // 锁外 I/O：获取记忆摘要
        let memory_summary_opt = {
            let effective = agent_clone.effective_motivation();
            let mot = agentora_core::motivation::MotivationVector::from_array(effective);
            let spark = Spark::from_gap(
                &mot,
                &[0.5; 6],
            );
            let spark_type = spark.spark_type;
            // clone 后的 Agent memory 已重连 SQLite，可以安全调用 get_summary
            let summary = agent_clone.memory.get_summary(spark_type);
            if summary.is_empty() { None } else { Some(summary) }
        };

        println!("[AgentLoop] Agent {:?} ({}) 开始决策{}", agent_id.as_str(), agent_clone.name,
            if is_npc { " (NPC 规则决策)" } else { "" });

        let action = if is_npc {
            // NPC：规则引擎决策（不调用 LLM）
            use agentora_core::rule_engine::RuleEngine;
            let effective = agent_clone.effective_motivation();
            let mot = agentora_core::motivation::MotivationVector::from_array(effective);
            let engine = RuleEngine::new();
            engine.rule_decision(&mot, &world_state)
        } else {
            // Player Agent：LLM 决策
            let effective = agent_clone.effective_motivation();
            let satisfaction = [0.5; 6];
            let spark = Spark::from_gap(
                &agentora_core::motivation::MotivationVector::from_array(effective),
                &satisfaction,
            );
            let effective_mot = agentora_core::motivation::MotivationVector::from_array(effective);

            let start = std::time::Instant::now();
            let result = pipeline.execute(&agent_clone.id, &effective_mot, &spark, &world_state, memory_summary_opt.as_deref()).await;
            let elapsed = start.elapsed().as_secs_f32();

            if result.error_info.is_some() {
                println!("[AgentLoop] Agent {:?} ({}) 决策完成 (耗时 {:.1}s, 兜底): {:?} | 原因: {:?}",
                    agent_id.as_str(), agent_clone.name, elapsed, result.selected_action.action_type, result.error_info);
            } else {
                println!("[AgentLoop] Agent {:?} ({}) 决策完成 (耗时 {:.1}s): {:?}",
                    agent_id.as_str(), agent_clone.name, elapsed, result.selected_action.action_type);
            }

            Action {
                reasoning: result.selected_action.reasoning,
                action_type: result.selected_action.action_type,
                target: result.selected_action.target,
                params: result.selected_action.params.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
                build_type: None,
                direction: None,
                motivation_delta: result.selected_action.motivation_delta,
            }
        };

        // 发送动作到 Apply 循环（如果通道满了，丢弃该动作）
        if let Err(e) = action_tx.try_send((agent_id.clone(), action)) {
            println!("[AgentLoop] Agent {:?} 动作发送失败: {:?}", agent_id, e);
        } else {
            println!("[AgentLoop] Agent {:?} ({}) 动作已发送", agent_id.as_str(), agent_clone.name);
        }
    }
}

/// Apply 循环：串行应用动作并发 delta + narrative
async fn run_apply_loop(
    mut action_rx: tokio::sync::mpsc::Receiver<(AgentId, Action)>,
    world: Arc<Mutex<World>>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
) {
    println!("[ApplyLoop] 启动");
    while let Some((agent_id, action)) = action_rx.recv().await {
        println!("[ApplyLoop] 收到 Agent {:?} 的动作: {:?}", agent_id, action.action_type);
        let (delta, events, extra_deltas) = {
            let mut w = world.lock().await;

            // 每应用一个动作前推进 world tick（确保世界状态更新）
            w.advance_tick();

            // 应用动作
            w.apply_action(&agent_id, &action);

            // 记录到 Agent 记忆系统（任务 4.2-4.3）
            if let Some(agent) = w.agents.get(&agent_id) {
                let action_type_str = format!("{:?}", action.action_type);
                // 根据 ActionType 自动标注 emotion_tags 和 importance
                let (emotion_tags, importance) = match action.action_type {
                    agentora_core::types::ActionType::Move { .. } => (vec!["neutral".to_string()], 0.2),
                    agentora_core::types::ActionType::MoveToward { .. } => (vec!["purposeful".to_string()], 0.3),  // 导航移动，有目的性
                    agentora_core::types::ActionType::Gather { .. } => (vec!["satisfied".to_string()], 0.4),
                    agentora_core::types::ActionType::Wait => (vec!["resting".to_string()], 0.1),
                    agentora_core::types::ActionType::Attack { .. } => (vec!["aggressive".to_string(), "angry".to_string()], 0.8),
                    agentora_core::types::ActionType::Talk { .. } => (vec!["social".to_string()], 0.5),
                    agentora_core::types::ActionType::Build { .. } => (vec!["creative".to_string()], 0.6),
                    agentora_core::types::ActionType::Explore { .. } => (vec!["curious".to_string()], 0.5),
                    agentora_core::types::ActionType::TradeOffer { .. } | agentora_core::types::ActionType::TradeAccept { .. } => (vec!["cooperative".to_string()], 0.6),
                    agentora_core::types::ActionType::AllyPropose { .. } | agentora_core::types::ActionType::AllyAccept { .. } => (vec!["trust".to_string(), "bonding".to_string()], 0.7),
                    agentora_core::types::ActionType::InteractLegacy { .. } => (vec!["reverent".to_string()], 0.7),
                    _ => (vec!["unknown".to_string()], 0.3),
                };

                let event = MemoryEvent {
                    tick: w.tick as u32,
                    event_type: action_type_str,
                    content: action.reasoning.clone(),
                    emotion_tags,
                    importance,
                };

                // 需要可变引用，重新获取 agent
                if let Some(agent_mut) = w.agents.get_mut(&agent_id) {
                    agent_mut.memory.record(&event);
                }
            }

            // 提取叙事事件
            let events: Vec<NarrativeEvent> = w.tick_events.drain(..).map(|e| NarrativeEvent {
                tick: e.tick,
                agent_id: e.agent_id,
                agent_name: e.agent_name,
                event_type: e.event_type,
                description: e.description,
                color_code: e.color_code,
            }).collect();

            // 构建 delta 事件
            let delta = match w.agents.get(&agent_id) {
                Some(agent) if agent.is_alive => {
                    let mot = agent.motivation.to_array();
                    println!("[ApplyLoop] Agent {:?} ({}) 应用成功 -> ({}, {})",
                        agent_id, agent.name, agent.position.x, agent.position.y);
                    AgentDelta::AgentMoved {
                        id: agent.id.as_str().to_string(),
                        name: agent.name.clone(),
                        position: (agent.position.x, agent.position.y),
                        health: agent.health,
                        max_health: agent.max_health,
                        is_alive: true,
                        age: agent.age,
                        motivation: mot,
                    }
                }
                Some(agent) => {
                    // Agent 死亡
                    println!("[ApplyLoop] Agent {:?} ({}) 死亡", agent_id, agent.name);
                    AgentDelta::AgentDied {
                        id: agent.id.as_str().to_string(),
                        name: agent.name.clone(),
                        position: (agent.position.x, agent.position.y),
                        age: agent.age,
                    }
                }
                None => {
                    println!("[ApplyLoop] Agent {:?} 不存在，跳过", agent_id);
                    AgentDelta::AgentDied {
                        id: agent_id.as_str().to_string(),
                        name: String::new(),
                        position: (0, 0),
                        age: 0,
                    }
                }
            };

            // Tier 2: 基于动作类型生成额外 delta
            let mut extra_deltas: Vec<AgentDelta> = Vec::new();
            match &action.action_type {
                agentora_core::types::ActionType::Build { structure } => {
                    if let Some(agent) = w.agents.get(&agent_id) {
                        extra_deltas.push(AgentDelta::StructureCreated {
                            x: agent.position.x,
                            y: agent.position.y,
                            structure_type: format!("{:?}", structure),
                            owner_id: agent_id.as_str().to_string(),
                        });
                    }
                }
                agentora_core::types::ActionType::Gather { resource } => {
                    if let Some(agent) = w.agents.get(&agent_id) {
                        if let Some(node) = w.resources.get(&agent.position) {
                            extra_deltas.push(AgentDelta::ResourceChanged {
                                x: agent.position.x,
                                y: agent.position.y,
                                resource_type: resource.as_str().to_string(),
                                amount: node.current_amount,
                            });
                        }
                    }
                }
                agentora_core::types::ActionType::TradeAccept { .. } => {
                    // 查找最近的已完成交易叙事
                    if let Some(event) = events.iter().find(|e| e.event_type == "trade") {
                        extra_deltas.push(AgentDelta::TradeCompleted {
                            from_id: agent_id.as_str().to_string(),
                            to_id: "unknown".to_string(),
                            items: event.description.clone(),
                        });
                    }
                }
                agentora_core::types::ActionType::AllyAccept { .. } => {
                    // 查找结盟叙事
                    if let Some(event) = events.iter().find(|e| e.event_type == "ally") {
                        extra_deltas.push(AgentDelta::AllianceFormed {
                            id1: agent_id.as_str().to_string(),
                            id2: "unknown".to_string(),
                        });
                    }
                }
                _ => {}
            }

            (delta, events, extra_deltas)
        };

        // 发送叙事事件
        for event in events {
            println!("[Narrative] tick={} {}: {}", event.tick, event.event_type, event.description);
            let _ = narrative_tx.send(event);
        }

        // 在锁外发送 delta，避免阻塞
        if let Err(e) = delta_tx.send(delta) {
            println!("[ApplyLoop] delta 发送失败: {:?}", e);
        }

        // Tier 2: 发送额外 delta 事件
        for extra_delta in extra_deltas {
            if let Err(e) = delta_tx.send(extra_delta) {
                println!("[ApplyLoop] extra delta 发送失败: {:?}", e);
            }
        }
    }
}

/// 定期 snapshot 兜底循环
async fn run_snapshot_loop(
    snapshot_tx: Sender<WorldSnapshot>,
    world: Arc<Mutex<World>>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    interval.tick().await; // 跳过第一次

    loop {
        interval.tick().await;

        let snapshot_opt = {
            let w = world.lock().await;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w.snapshot()))
        };

        match snapshot_opt {
            Ok(snapshot) => {
                println!("[SimulationBridge] snapshot 生成成功，tick={}", snapshot.tick);
                if snapshot_tx.send(snapshot).is_err() {
                    break;
                }
            }
            Err(e) => {
                eprintln!("[SimulationBridge] snapshot() panic: {:?}", e);
                break;
            }
        }
    }
}

struct AgentoraExtension;

#[gdextension(entry_symbol = agentora_bridge_init)]
unsafe impl ExtensionLibrary for AgentoraExtension {}
