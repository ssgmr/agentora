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

/// 日志配置
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

impl LogConfig {
    fn load(path: &str) -> Self {
        let defaults = Self {
            console_enabled: true,
            console_level: "info".to_string(),
            file_enabled: true,
            file_level: "debug".to_string(),
            log_dir: "../logs".to_string(),
            rotation: "daily".to_string(),
            targets: std::collections::HashMap::new(),
        };

        match std::fs::read_to_string(path) {
            Ok(content) => {
                match toml::from_str::<toml::Value>(&content) {
                    Ok(table) => {
                        let mut cfg = defaults;
                        if let Some(log) = table.get("log") {
                            if let Some(v) = log.get("console_enabled").and_then(|v| v.as_bool()) {
                                cfg.console_enabled = v;
                            }
                            if let Some(v) = log.get("console_level").and_then(|v| v.as_str()) {
                                cfg.console_level = v.to_string();
                            }
                            if let Some(v) = log.get("file_enabled").and_then(|v| v.as_bool()) {
                                cfg.file_enabled = v;
                            }
                            if let Some(v) = log.get("file_level").and_then(|v| v.as_str()) {
                                cfg.file_level = v.to_string();
                            }
                            if let Some(v) = log.get("log_dir").and_then(|v| v.as_str()) {
                                cfg.log_dir = v.to_string();
                            }
                            if let Some(v) = log.get("rotation").and_then(|v| v.as_str()) {
                                cfg.rotation = v.to_string();
                            }
                            if let Some(targets) = log.get("targets").and_then(|v| v.as_table()) {
                                for (k, v) in targets {
                                    if let Some(level) = v.as_str() {
                                        cfg.targets.insert(k.clone(), level.to_string());
                                    }
                                }
                            }
                        }
                        cfg
                    }
                    Err(e) => {
                        eprintln!("[Logger] log.toml 解析失败 ({}), 使用默认配置", e);
                        defaults
                    }
                }
            }
            Err(e) => {
                eprintln!("[Logger] log.toml 未找到 ({}), 使用默认配置", e);
                defaults
            }
        }
    }
}

// 核心引擎类型
use agentora_core::{World, WorldSeed, Agent, Position, WorldSnapshot, AgentId, Action};
use agentora_core::snapshot::{AgentSnapshot, NarrativeEvent as CoreNarrativeEvent};
use agentora_core::decision::{DecisionPipeline, infer_state_mode};
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
    /// Agent 视野扫描半径（格子数）
    pub vision_radius: u32,
    /// 背包格子数量（不同资源类型的种类上限）
    pub inventory_max_slots: usize,
    /// 单个资源堆叠上限
    pub inventory_max_stack_size: u32,
    /// 仓库附近堆叠上限倍率
    pub inventory_warehouse_multiplier: u32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            initial_agent_count: 3,
            npc_count: 3,
            npc_decision_interval_secs: 1,
            player_decision_interval_secs: 2,
            vision_radius: 10,
            inventory_max_slots: 20,
            inventory_max_stack_size: 20,
            inventory_warehouse_multiplier: 2,
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
                            if let Some(v) = sim.get("vision_radius").and_then(|v| v.as_integer()) {
                                cfg.vision_radius = v as u32;
                            }
                        }
                        if let Some(inv) = table.get("inventory") {
                            if let Some(v) = inv.get("max_slots").and_then(|v| v.as_integer()) {
                                cfg.inventory_max_slots = v as usize;
                            }
                            if let Some(v) = inv.get("max_stack_size").and_then(|v| v.as_integer()) {
                                cfg.inventory_max_stack_size = v as u32;
                            }
                            if let Some(v) = inv.get("warehouse_limit_multiplier").and_then(|v| v.as_integer()) {
                                cfg.inventory_warehouse_multiplier = v as u32;
                            }
                        }
                        tracing::info!("[SimConfig] 配置加载成功 [agents={} npc={} npc_interval={}s player_interval={}s vision_radius={} inv_slots={} inv_stack={} warehouse_mult={}]",
                            cfg.initial_agent_count, cfg.npc_count, cfg.npc_decision_interval_secs, cfg.player_decision_interval_secs, cfg.vision_radius,
                            cfg.inventory_max_slots, cfg.inventory_max_stack_size, cfg.inventory_warehouse_multiplier);
                        cfg
                    }
                    Err(e) => {
                        tracing::warn!("[SimConfig] sim.toml 解析失败 ({}), 使用默认配置", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[SimConfig] sim.toml 未找到 ({}), 使用默认配置", e);
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
    InjectPreference {
        agent_id: String,
        key: String,
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

                // 发送 map_changes（资源和建筑信息）
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

                let mc_count = snapshot.map_changes.len();
                godot::global::print(&[format!("[SimulationBridge] physics_process: 发送 snapshot 含 {} 个 map_changes", mc_count).to_variant()]);
                self.base_mut().emit_signal("world_updated", &[snapshot_dict.to_variant()]);
                godot::global::print(&[Variant::from("[SimulationBridge] world_updated 信号已发出")]);
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
        dict.set("level", &(Variant::from(agent.level as i64)));
        dict.set("current_action", &agent.current_action.clone().to_variant());
        dict.set("action_result", &agent.action_result.clone().to_variant());
        let pos = Vector2::new(agent.position.0 as f32, agent.position.1 as f32);
        dict.set("position", &pos.to_variant());
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

        let (llm_provider, llm_config) = Self::create_llm_provider();

        std::thread::spawn(move || {
            run_simulation(snapshot_tx, delta_tx, narrative_tx, cmd_rx, llm_provider, llm_config);
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

async fn run_simulation_async(
    snapshot_tx: Sender<WorldSnapshot>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    llm_provider: Option<Box<dyn LlmProvider>>,
    llm_config: agentora_ai::config::LlmConfig,
) {
    // 加载模拟配置（相对于项目根目录，Godot 工作目录为 client/）
    let sim_config = SimConfig::load("../config/sim.toml");

    // 初始化背包配置
    agentora_core::agent::inventory::init_inventory_config(agentora_core::agent::inventory::InventoryConfig {
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
    // 覆盖 Agent 数量（从 sim.toml 读取）
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

    // 初始化 Agent 并 spawn 同步决策+执行 task
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
        let handle = tokio::spawn(async move {
            run_agent_loop(w, aid, p, delta, narrative, false, interval as u32, vision_r).await;
        });
        _agent_handles.push(handle);
    }

    // 创建 NPC Agent 并 spawn 同步决策+执行 task
    let npc_ids = create_npc_agents(&world_arc, &sim_config).await;
    for npc_id in &npc_ids {
        let w = world_arc.clone();
        let p = pipeline_arc.clone();
        let delta = delta_tx.clone();
        let narrative = narrative_tx.clone();
        let aid = npc_id.clone();
        let interval = sim_config.npc_decision_interval_secs;
        let vision_r = sim_config.vision_radius;
        let handle = tokio::spawn(async move {
            run_agent_loop(w, aid, p, delta, narrative, true, interval as u32, vision_r).await;
        });
        _agent_handles.push(handle);
    }

    let all_agent_count = agent_ids.len() + npc_ids.len();
    tracing::info!("世界已创建，{} 个 Agent（{} LLM + {} NPC）",
        all_agent_count, agent_ids.len(), npc_ids.len());

    // 立即发送初始 snapshot，让 Godot 能渲染资源、建筑等初始状态
    {
        let w = world_arc.lock().await;
        let initial_snapshot = w.snapshot();
        let map_changes_count = initial_snapshot.map_changes.len();
        let _ = snapshot_tx.send(initial_snapshot);
        tracing::info!("已发送初始 snapshot（含 {} 个 map_changes）", map_changes_count);
    }

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
                    tracing::info!("模拟暂停状态 = {}", is_paused);
                }
                SimCommand::Start => {
                    is_paused = false;
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
                        tracing::info!("注入偏好 {:?} key={} boost={} duration={}",
                            aid, key, boost, duration_ticks);
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

        let agent = Agent::new(
            AgentId::default(),
            name.clone(),
            Position::new(x, y),
        );

        let aid = agent.id.clone();
        world.insert_agent_at(aid.clone(), agent);
        ids.push(aid);
    }

    ids
}

/// Agent 同步决策+执行循环
/// 每个 Agent 独立 task，在同一个 task 内顺序完成：读取状态 → LLM 决策 → 应用动作 → 推送 delta
async fn run_agent_loop(
    world: Arc<Mutex<World>>,
    agent_id: AgentId,
    pipeline: Arc<DecisionPipeline>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    is_npc: bool,
    interval_secs: u32,
    vision_radius: u32,
) {
    tracing::info!("[AgentLoop] Agent {:?} 启动 (is_npc={}, interval={}s, vision_radius={})", agent_id, is_npc, interval_secs, vision_radius);

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs as u64));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        // 检查 Agent 是否存活
        let should_continue = {
            let w = world.lock().await;
            match w.agents.get(&agent_id) {
                Some(agent) => agent.is_alive,
                None => false,
            }
        };

        if !should_continue {
            tracing::warn!("[AgentLoop] Agent {:?} 已死亡或不存在，退出循环", agent_id);
            break;
        }

        // 构建 WorldState（锁内纯计算 + 锁外 I/O）
        let (agent_clone, world_state) = {
            let w = world.lock().await;

            let vision = scan_vision(&w, &agent_id, vision_radius);

            let agent = match w.agents.get(&agent_id) {
                Some(a) => a.clone(),
                None => break,
            };

            tracing::debug!("[AgentLoop] Agent {:?} vision: {} terrain, {} resources, {} agents, {} structures, {} legacies",
                agent_id, vision.terrain_at.len(), vision.resources_at.len(), vision.nearby_agents.len(), vision.nearby_structures.len(), vision.nearby_legacies.len());

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
                self_id: agent_id.clone(),
                existing_agents: w.agents.keys().cloned().collect(),
                resources_at: vision.resources_at,
                nearby_agents: vision.nearby_agents,
                nearby_structures: vision.nearby_structures,
                nearby_legacies: vision.nearby_legacies,
                active_pressures: w.pressure_pool.iter().map(|p| p.description.clone()).collect(),
                last_move_direction: agent.last_position.and_then(|last_pos| {
                    agentora_core::vision::calculate_direction(&last_pos, &agent.position)
                }),
                temp_preferences: agent.temp_preferences.iter()
                    .map(|p| (p.key.clone(), p.boost, p.remaining_ticks))
                    .collect(),
            };

            (agent, ws)
        };

        // 锁外 I/O：获取记忆摘要
        let memory_summary_opt = {
            let spark_type = infer_state_mode(&world_state);
            let summary = agent_clone.memory.get_summary(spark_type);
            if summary.is_empty() { None } else { Some(summary) }
        };

        tracing::debug!("[AgentLoop] Agent {:?} ({}) 开始决策{}", agent_id.as_str(), agent_clone.name,
            if is_npc { " (NPC 规则决策)" } else { "" });

        let (action, validation_failure): (Option<Action>, Option<String>) = if is_npc {
            // NPC：规则引擎生存兜底（不调用 LLM）
            use agentora_core::rule_engine::RuleEngine;
            let engine = RuleEngine::new();
            if let Some(candidate) = engine.survival_fallback(&world_state) {
                (Some(Action {
                    reasoning: candidate.reasoning,
                    action_type: candidate.action_type,
                    target: candidate.target,
                    params: candidate.params.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
                    build_type: None,
                    direction: None,
                }), None)
            } else {
                (Some(Action {
                    reasoning: "NPC 无明确目标，等待".to_string(),
                    action_type: agentora_core::types::ActionType::Wait,
                    target: None,
                    params: HashMap::new(),
                    build_type: None,
                    direction: None,
                }), None)
            }
        } else {
            // Player Agent：LLM 决策
            let _ = agent_clone.last_action_type.as_deref(); // 保留占位，待后续使用
            let action_feedback = agent_clone.last_action_result.as_deref();
            let start = std::time::Instant::now();
            let result = pipeline.execute(&agent_clone.id, &world_state, memory_summary_opt.as_deref(), action_feedback).await;
            let elapsed = start.elapsed().as_secs_f32();

            if result.error_info.is_some() {
                // 校验失败：不执行动作，记录反馈让 LLM 下回合修正
                let vf = result.validation_failure.clone();
                if let Some(ref msg) = vf {
                    tracing::warn!("[AgentLoop] Agent {:?} ({}) 决策被拒绝 (耗时 {:.1}s): {}",
                        agent_id.as_str(), agent_clone.name, elapsed, msg);
                    eprintln!("[决策被拒绝] {} (耗时 {:.1}s): {}", agent_clone.name, elapsed, msg);
                }
                (None, vf)
            } else {
                let candidate = result.selected_action.expect("决策成功但 selected_action 为 None");
                tracing::info!("[AgentLoop] Agent {:?} ({}) 决策完成 (耗时 {:.1}s): {:?}",
                    agent_id.as_str(), agent_clone.name, elapsed, candidate.action_type);
                eprintln!("[{}] {} (耗时 {:.1}s): {:?}", agent_clone.name, "决策完成", elapsed, candidate.action_type);
                eprintln!("[{}] reasoning: {}", agent_clone.name, candidate.reasoning);

                (Some(Action {
                    reasoning: candidate.reasoning,
                    action_type: candidate.action_type,
                    target: candidate.target,
                    params: candidate.params.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
                    build_type: None,
                    direction: None,
                }), None)
            }
        };

        // 应用动作并发送 delta/narrative（同一个锁内完成，确保位置一致性）
        if let Some(action) = action {
            let events = {
                let mut w = world.lock().await;
                w.advance_tick();
                w.apply_action(&agent_id, &action);

                // 记录到 Agent 记忆系统
                if let Some(_agent) = w.agents.get(&agent_id) {
                    let action_type_str = format!("{:?}", action.action_type);
                    let (emotion_tags, importance) = match action.action_type {
                        agentora_core::types::ActionType::MoveToward { .. } => (vec!["purposeful".to_string()], 0.3),
                        agentora_core::types::ActionType::Gather { .. } => (vec!["satisfied".to_string()], 0.4),
                        agentora_core::types::ActionType::Wait => (vec!["resting".to_string()], 0.1),
                        agentora_core::types::ActionType::Eat => (vec!["satisfied".to_string()], 0.3),
                        agentora_core::types::ActionType::Drink => (vec!["refreshed".to_string()], 0.3),
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
                let delta: Option<AgentDelta> = match w.agents.get(&agent_id) {
                    Some(agent) if agent.is_alive => {
                        Some(AgentDelta::AgentMoved {
                            id: agent.id.as_str().to_string(),
                            name: agent.name.clone(),
                            position: (agent.position.x, agent.position.y),
                            health: agent.health,
                            max_health: agent.max_health,
                            is_alive: true,
                            age: agent.age,
                        })
                    }
                    Some(agent) => {
                        Some(AgentDelta::AgentDied {
                            id: agent.id.as_str().to_string(),
                            name: agent.name.clone(),
                            position: (agent.position.x, agent.position.y),
                            age: agent.age,
                        })
                    }
                    None => None,
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
                        if let Some(event) = events.iter().find(|e| e.event_type == "trade") {
                            extra_deltas.push(AgentDelta::TradeCompleted {
                                from_id: agent_id.as_str().to_string(),
                                to_id: "unknown".to_string(),
                                items: event.description.clone(),
                            });
                        }
                    }
                    agentora_core::types::ActionType::AllyAccept { .. } => {
                        if events.iter().any(|e| e.event_type == "ally") {
                            extra_deltas.push(AgentDelta::AllianceFormed {
                                id1: agent_id.as_str().to_string(),
                                id2: "unknown".to_string(),
                            });
                        }
                    }
                    _ => {}
                }

                // 发送 delta
                if let Some(delta) = delta {
                    if let Err(e) = delta_tx.send(delta) {
                        tracing::error!("[AgentLoop] delta 发送失败: {:?}", e);
                    }
                    for extra in extra_deltas {
                        if let Err(e) = delta_tx.send(extra) {
                            tracing::error!("[AgentLoop] extra delta 发送失败: {:?}", e);
                        }
                    }
                }

                events
            };

            // 发送叙事事件（锁外）
            for event in events {
                tracing::info!("[Narrative] tick={} {}: {}", event.tick, event.event_type, event.description);
                let _ = narrative_tx.send(event);
            }
        } else if !is_npc {
            // Player Agent 决策被拒绝，写入 last_action_result 供下次决策使用
            if let Some(ref vf) = validation_failure {
                let mut w = world.lock().await;
                if let Some(agent) = w.agents.get_mut(&agent_id) {
                    agent.last_action_result = Some(format!("[错误] 上次决策被拒绝：{}", vf));
                }
                tracing::info!("[AgentLoop] Agent {:?} LLM 校验失败反馈已记录: {}", agent_id, vf);
            }
        }

        interval.tick().await;
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
                tracing::info!("[SimulationBridge] snapshot 生成成功，tick={}", snapshot.tick);
                if snapshot_tx.send(snapshot).is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::error!("[SimulationBridge] snapshot() panic: {:?}", e);
                break;
            }
        }
    }
}

struct AgentoraExtension;

#[gdextension(entry_symbol = agentora_bridge_init)]
unsafe impl ExtensionLibrary for AgentoraExtension {}
