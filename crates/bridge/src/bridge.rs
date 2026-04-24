//! SimulationBridge GDExtension 节点
//!
//! Godot 节点定义 + INode 实现 + GDExtension API

use godot::prelude::*;
use godot::classes::{Node, INode};
use std::sync::mpsc::{self, Sender, Receiver};

use agentora_core::simulation::Delta;
use agentora_core::snapshot::NarrativeEvent;
use agentora_core::WorldSnapshot;
use agentora_ai::{load_llm_config, OpenAiProvider, FallbackChain, LlmProvider};

use crate::logging::init_logging;
use crate::conversion::{delta_to_dict, snapshot_to_dict};
use crate::simulation_runner::run_simulation_with_api;

/// 模拟命令（控制模拟状态）
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
    delta_receiver: Option<Receiver<Delta>>,
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
                    let delta_dict = delta_to_dict(&delta);
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

                let snapshot_dict = snapshot_to_dict(&snapshot);
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

    #[func]
    fn start_simulation(&mut self) {
        if self.is_running {
            godot::global::print(&[Variant::from("SimulationBridge: 模拟已在运行")]);
            return;
        }
        godot::global::print(&[Variant::from("SimulationBridge: 启动模拟...")]);

        let (snapshot_tx, snapshot_rx) = mpsc::channel::<WorldSnapshot>();
        let (delta_tx, delta_rx) = mpsc::channel::<Delta>();
        let (narrative_tx, narrative_rx) = mpsc::channel::<NarrativeEvent>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<SimCommand>();

        self.snapshot_receiver = Some(snapshot_rx);
        self.delta_receiver = Some(delta_rx);
        self.narrative_receiver = Some(narrative_rx);
        self.command_sender = Some(cmd_tx);
        self.is_running = true;
        self.is_paused = false;

        let (llm_provider, llm_config) = Self::create_llm_provider();

        // 使用 simulation_runner 模块运行模拟
        std::thread::spawn(move || {
            run_simulation_with_api(snapshot_tx, delta_tx, narrative_tx, cmd_rx, llm_provider, llm_config);
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
                dict.set("reasoning", &agent.reasoning.clone().unwrap_or_default().to_variant());
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