//! Agentora Godot GDExtension 桥接
//!
//! 薄桥接层：SimulationBridge 节点定义 + 类型转换 + 信号发射

use godot::prelude::*;
use godot::init::ExtensionLibrary;
use godot::classes::{Node, INode};
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use std::sync::Once;

// 日志初始化器（只执行一次）
static LOG_INIT: Once = Once::new();
static mut LOG_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;

fn init_logging() {
    godot::global::print(&[Variant::from("[Logger] init_logging called")]);
    LOG_INIT.call_once(|| {
        if let Err(e) = try_init_logging() {
            eprintln!("[Logger] 初始化失败: {}", e);
        }
    });
}

fn try_init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tracing_subscriber::{fmt, EnvFilter, Layer, prelude::*};
    use tracing_subscriber::fmt::time::LocalTime;
    use time::macros::format_description;

    // 加载日志配置
    let log_cfg = LogConfig::load("../config/log.toml");

    // 日志目录：从配置文件读取，相对于当前工作目录
    let log_dir = std::path::Path::new(&log_cfg.log_dir);
    std::fs::create_dir_all(log_dir)?;

    // 构建 EnvFilter
    let mut filter_str = log_cfg.file_level.clone();
    for (target, level) in &log_cfg.targets {
        filter_str.push_str(&format!(",{}={}", target, level));
    }

    let console_filter = EnvFilter::try_new(if log_cfg.console_enabled {
        &log_cfg.console_level
    } else {
        "off"
    }).unwrap_or_else(|_| EnvFilter::new("info"));
    let file_filter = EnvFilter::try_new(if log_cfg.file_enabled {
        &filter_str
    } else {
        "off"
    }).unwrap_or_else(|_| EnvFilter::new("debug"));

    let rotation = match log_cfg.rotation.as_str() {
        "hourly" => tracing_appender::rolling::Rotation::HOURLY,
        "never" => tracing_appender::rolling::Rotation::NEVER,
        _ => tracing_appender::rolling::Rotation::DAILY,
    };
    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        rotation,
        log_dir,
        "agentora.log",
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let time_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]");
    let local_timer = LocalTime::new(time_format);

    let console_layer = fmt::layer()
        .with_target(false)
        .with_ansi(false)
        .with_timer(local_timer.clone())
        .with_writer(std::io::stdout)
        .with_filter(console_filter);
    let file_layer = fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_timer(local_timer)
        .with_writer(non_blocking)
        .with_filter(file_filter);

    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    // 保持 guard 存活
    unsafe { LOG_GUARD = Some(guard); }

    godot::global::print(&[Variant::from(format!(
        "[Logger] 日志已初始化 [{}] → {}",
        log_cfg.file_level, log_cfg.log_dir
    ))]);

    Ok(())
}

/// 日志配置（简化版）
#[derive(Debug, Clone, serde::Deserialize)]
struct LogConfigFile {
    log: Option<LogSection>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct LogSection {
    console_enabled: Option<bool>,
    console_level: Option<String>,
    file_enabled: Option<bool>,
    file_level: Option<String>,
    log_dir: Option<String>,
    rotation: Option<String>,
    targets: Option<std::collections::HashMap<String, String>>,
}

/// 日志配置（运行时使用的扁平结构）
#[derive(Debug, Clone)]
struct LogConfig {
    console_enabled: bool,
    console_level: String,
    file_enabled: bool,
    file_level: String,
    log_dir: String,
    rotation: String,
    targets: std::collections::HashMap<String, String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            console_enabled: true,
            console_level: "info".to_string(),
            file_enabled: true,
            file_level: "debug".to_string(),
            log_dir: "../logs".to_string(),
            rotation: "daily".to_string(),
            targets: std::collections::HashMap::new(),
        }
    }
}

impl LogConfig {
    fn load(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                match toml::from_str::<LogConfigFile>(&content) {
                    Ok(file) => {
                        let mut cfg = Self::default();
                        if let Some(log) = file.log {
                            if let Some(v) = log.console_enabled { cfg.console_enabled = v; }
                            if let Some(v) = log.console_level { cfg.console_level = v; }
                            if let Some(v) = log.file_enabled { cfg.file_enabled = v; }
                            if let Some(v) = log.file_level { cfg.file_level = v; }
                            if let Some(v) = log.log_dir { cfg.log_dir = v; }
                            if let Some(v) = log.rotation { cfg.rotation = v; }
                            if let Some(v) = log.targets { cfg.targets = v; }
                        }
                        cfg
                    }
                    Err(e) => {
                        eprintln!("[Logger] log.toml 解析失败 ({}), 使用默认配置", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                eprintln!("[Logger] log.toml 未找到 ({}), 使用默认配置", e);
                Self::default()
            }
        }
    }
}

// 从 core::simulation 导入类型
use agentora_core::simulation::{SimConfig, AgentDelta};
use agentora_core::simulation::agent_loop::NarrativeEvent;
use agentora_core::{World, WorldSeed, WorldSnapshot, AgentId, DecisionPipeline};
use agentora_core::snapshot::AgentSnapshot;
use agentora_core::agent::inventory::{InventoryConfig, init_inventory_config};
use agentora_ai::{load_llm_config, OpenAiProvider, FallbackChain, LlmProvider};

/// 模拟命令
#[derive(Debug, Clone)]
pub enum SimCommand {
    Start,
    Pause,
    SetTickInterval { seconds: f32 },
    InjectPreference {
        agent_id: String,
        key: String,
        boost: f32,
        duration_ticks: u32,
    },
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
        init_logging();
        tracing::info!("SimulationBridge: 初始化完成");
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

                // 发送 map_changes
                let mut map_changes_array: Array<Variant> = Array::new();
                for change in &snapshot.map_changes {
                    let mut change_dict: Dictionary<GString, Variant> = Dictionary::new();
                    change_dict.set("x", &(Variant::from(change.x as i64)));
                    change_dict.set("y", &(Variant::from(change.y as i64)));
                    change_dict.set("terrain", &change.terrain.to_variant());
                    if let Some(structure) = &change.structure {
                        change_dict.set("structure", &structure.to_variant());
                    }
                    if let Some(resource_type) = &change.resource_type {
                        change_dict.set("resource_type", &resource_type.to_variant());
                    }
                    if let Some(resource_amount) = &change.resource_amount {
                        change_dict.set("resource_amount", &(Variant::from(*resource_amount as i64)));
                    }
                    map_changes_array.push(&change_dict.to_variant());
                }
                snapshot_dict.set("map_changes", &map_changes_array.to_variant());

                // 发送地形网格数据
                if let (Some(grid), Some(w), Some(h)) = (&snapshot.terrain_grid, &snapshot.terrain_width, &snapshot.terrain_height) {
                    let grid_packed = PackedByteArray::from(grid.as_slice());
                    snapshot_dict.set("terrain_grid", &grid_packed.to_variant());
                    snapshot_dict.set("terrain_width", &(Variant::from(*w as i64)));
                    snapshot_dict.set("terrain_height", &(Variant::from(*h as i64)));
                }

                let mc_count = snapshot.map_changes.len();
                let terrain_info = if snapshot.terrain_grid.is_some() {
                    format!("含 terrain_grid {}x{}", snapshot.terrain_width.unwrap_or(0), snapshot.terrain_height.unwrap_or(0))
                } else {
                    "无 terrain_grid".to_string()
                };
                tracing::debug!("[SimulationBridge] physics_process: 发送 snapshot 含 {} 个 map_changes, {}", mc_count, terrain_info);
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
            AgentDelta::AgentMoved { id, name, position, health, max_health, is_alive, age } => {
                dict.set("type", &"agent_moved".to_variant());
                dict.set("id", &id.to_variant());
                dict.set("name", &name.to_variant());
                let pos = Vector2::new(position.0 as f32, position.1 as f32);
                dict.set("position", &pos.to_variant());
                dict.set("health", &(Variant::from(*health as i64)));
                dict.set("max_health", &(Variant::from(*max_health as i64)));
                dict.set("is_alive", &is_alive.to_variant());
                dict.set("age", &(Variant::from(*age as i64)));
            }
            AgentDelta::AgentDied { id, name, position, age } => {
                dict.set("type", &"agent_died".to_variant());
                dict.set("id", &id.to_variant());
                dict.set("name", &name.to_variant());
                let pos = Vector2::new(position.0 as f32, position.1 as f32);
                dict.set("position", &pos.to_variant());
                dict.set("age", &(Variant::from(*age as i64)));
            }
            AgentDelta::AgentSpawned { id, name, position, health, max_health } => {
                dict.set("type", &"agent_spawned".to_variant());
                dict.set("id", &id.to_variant());
                dict.set("name", &name.to_variant());
                let pos = Vector2::new(position.0 as f32, position.1 as f32);
                dict.set("position", &pos.to_variant());
                dict.set("health", &(Variant::from(*health as i64)));
                dict.set("max_health", &(Variant::from(*max_health as i64)));
            }
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
        dict.set("level", &(Variant::from(agent.level as i64)));
        dict.set("current_action", &agent.current_action.clone().to_variant());
        dict.set("action_result", &agent.action_result.clone().to_variant());
        let pos = Vector2::new(agent.position.0 as f32, agent.position.1 as f32);
        dict.set("position", &pos.to_variant());
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

        let (llm_provider, llm_config) = Self::create_llm_provider();

        std::thread::spawn(move || {
            run_simulation(snapshot_tx, delta_tx, narrative_tx, cmd_rx, llm_provider, llm_config);
        });

        godot::global::print(&[Variant::from("SimulationBridge: 模拟已启动（事件驱动模式）")]);
    }

    #[func]
    fn start(&mut self) {
        self.start_simulation();
    }

    #[func]
    fn pause(&mut self) {
        self.toggle_pause();
    }

    fn create_llm_provider() -> (Option<Box<dyn LlmProvider>>, agentora_ai::config::LlmConfig) {
        let config_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/llm.toml");
        match load_llm_config(config_path) {
            Ok(config) => {
                let model = config.primary.model.clone();
                let full_config = config.clone();
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
                (Some(Box::new(fallback)), full_config)
            }
            Err(e) => {
                godot::global::print(&[Variant::from(format!(
                    "SimulationBridge: LLM 配置加载失败: {}，将使用规则引擎兜底",
                    e
                ))]);
                (None, agentora_ai::config::LlmConfig::default())
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
    fn inject_preference(&self, agent_id: String, key: String, boost: f32, duration: i32) {
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::InjectPreference {
                agent_id,
                key,
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
                dict.set("satiety", &(Variant::from(agent.satiety as i64)));
                dict.set("hydration", &(Variant::from(agent.hydration as i64)));
                dict.set("is_alive", &agent.is_alive.to_variant());
                dict.set("age", &(Variant::from(agent.age as i64)));
                dict.set("level", &(Variant::from(agent.level as i64)));
                dict.set("current_action", &agent.current_action.clone().to_variant());
                dict.set("action_result", &agent.action_result.clone().to_variant());
                dict.set("reasoning", &agent.reasoning.clone().to_variant());
                let pos = Vector2::new(agent.position.0 as f32, agent.position.1 as f32);
                dict.set("position", &pos.to_variant());
                let mut inv_dict: Dictionary<GString, Variant> = Dictionary::new();
                for (k, v) in &agent.inventory_summary {
                    inv_dict.set(k.as_str(), &Variant::from(*v as i64));
                }
                dict.set("inventory_summary", &inv_dict.to_variant());
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

/// 模拟入口函数（调用 core::simulation 模块）
fn run_simulation(
    snapshot_tx: Sender<WorldSnapshot>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    llm_provider: Option<Box<dyn LlmProvider>>,
    llm_config: agentora_ai::config::LlmConfig,
) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_simulation_async(snapshot_tx, delta_tx, narrative_tx, cmd_rx, llm_provider, llm_config).await;
    });
}

/// 异步模拟主函数（使用 core::simulation 组件）
async fn run_simulation_async(
    snapshot_tx: Sender<WorldSnapshot>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    llm_provider: Option<Box<dyn LlmProvider>>,
    llm_config: agentora_ai::config::LlmConfig,
) {
    // 加载模拟配置
    let sim_config = SimConfig::load("../config/sim.toml");

    // 初始化背包配置
    init_inventory_config(InventoryConfig {
        max_slots: sim_config.inventory_max_slots,
        max_stack_size: sim_config.inventory_max_stack_size,
        warehouse_limit_multiplier: sim_config.inventory_warehouse_multiplier,
    });

    // 从配置文件加载世界种子
    let mut seed = WorldSeed::load("../worldseeds/default.toml")
        .unwrap_or_else(|e| {
            tracing::error!("加载世界种子失败: {}，使用默认配置", e);
            WorldSeed::default()
        });
    seed.initial_agents = sim_config.initial_agent_count as u32;

    let world = World::new(&seed);

    let pipeline = if let Some(provider) = llm_provider {
        DecisionPipeline::from_config(&llm_config.memory)
            .with_llm_provider(provider)
            .with_llm_params(llm_config.decision.max_tokens, llm_config.decision.temperature)
    } else {
        DecisionPipeline::from_config(&llm_config.memory)
            .with_llm_params(llm_config.decision.max_tokens, llm_config.decision.temperature)
    };

    // 共享 World（Arc + Mutex）
    let world_arc = Arc::new(Mutex::new(world));
    let pipeline_arc = Arc::new(pipeline);

    // 共享暂停状态
    let is_paused = Arc::new(AtomicBool::new(false));

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
        let delta = delta_tx.clone();
        let narrative = narrative_tx.clone();
        let aid = agent_id.clone();
        let interval = sim_config.player_decision_interval_secs;
        let vision_r = sim_config.vision_radius;
        let pause_state = is_paused.clone();
        let handle = tokio::spawn(async move {
            agentora_core::simulation::agent_loop::run_agent_loop(
                w, aid, p, delta, narrative, false, interval as u32, vision_r, pause_state
            ).await;
        });
        _agent_handles.push(handle);
    }

    // 创建 NPC Agent
    let npc_ids = agentora_core::simulation::npc::create_npc_agents(&world_arc, &sim_config).await;
    for npc_id in &npc_ids {
        let w = world_arc.clone();
        let p = pipeline_arc.clone();
        let delta = delta_tx.clone();
        let narrative = narrative_tx.clone();
        let aid = npc_id.clone();
        let interval = sim_config.npc_decision_interval_secs;
        let vision_r = sim_config.vision_radius;
        let pause_state = is_paused.clone();
        let handle = tokio::spawn(async move {
            agentora_core::simulation::agent_loop::run_agent_loop(
                w, aid, p, delta, narrative, true, interval as u32, vision_r, pause_state
            ).await;
        });
        _agent_handles.push(handle);
    }

    let all_agent_count = agent_ids.len() + npc_ids.len();
    tracing::info!("世界已创建，{} 个 Agent（{} LLM + {} NPC）",
        all_agent_count, agent_ids.len(), npc_ids.len());

    // 立即发送初始 snapshot
    {
        let w = world_arc.lock().await;
        let initial_snapshot = w.snapshot();
        let map_changes_count = initial_snapshot.map_changes.len();
        let _ = snapshot_tx.send(initial_snapshot);
        tracing::info!("已发送初始 snapshot（含 {} 个 map_changes）", map_changes_count);
    }

    // 定期 snapshot 兜底循环
    let w_snap = world_arc.clone();
    let pause_snap = is_paused.clone();
    let _snap_handle = tokio::spawn(async move {
        agentora_core::simulation::snapshot_loop::run_snapshot_loop(snapshot_tx, w_snap, pause_snap).await;
    });

    // 世界时间推进循环
    let w_tick = world_arc.clone();
    let pause_tick = is_paused.clone();
    let tick_interval_secs = sim_config.tick_interval_secs;
    let _tick_handle = tokio::spawn(async move {
        agentora_core::simulation::tick_loop::run_tick_loop(w_tick, pause_tick, tick_interval_secs).await;
    });

    // 命令处理循环
    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                SimCommand::Pause => {
                    let current = is_paused.load(Ordering::SeqCst);
                    is_paused.store(!current, Ordering::SeqCst);
                    tracing::info!("模拟暂停状态 = {}", !current);
                }
                SimCommand::Start => {
                    is_paused.store(false, Ordering::SeqCst);
                    tracing::info!("模拟恢复运行");
                }
                SimCommand::SetTickInterval { seconds } => {
                    let mut world = world_arc.lock().await;
                    world.tick_interval = seconds as u32;
                }
                SimCommand::InjectPreference { agent_id, key, boost, duration_ticks } => {
                    let aid = AgentId::new(agent_id.clone());
                    let mut world = world_arc.lock().await;
                    if let Some(agent) = world.agents.get_mut(&aid) {
                        agent.inject_preference(&key, boost, duration_ticks);
                        tracing::info!(
                            "✅ 注入偏好成功: {:?} key={} boost={} duration={} ticks",
                            aid, key, boost, duration_ticks
                        );
                    } else {
                        tracing::warn!("❌ 注入偏好失败: Agent {:?} 不存在", aid);
                    }
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

struct AgentoraExtension;

#[gdextension(entry_symbol = agentora_bridge_init)]
unsafe impl ExtensionLibrary for AgentoraExtension {}