//! Agentora Godot GDExtension 桥接
//!
//! Tokio 运行时管理、mpsc Channel 桥接、WorldSnapshot 序列化。

use godot::prelude::*;
use godot::init::ExtensionLibrary;
use godot::classes::{Node, INode};
use std::sync::mpsc::{self, Sender, Receiver};

// 核心引擎类型
use agentora_core::{World, WorldSeed, Agent, Position, WorldSnapshot, AgentId};
use agentora_core::decision::{DecisionPipeline, Spark};
use agentora_core::rule_engine::WorldState;
use std::collections::HashMap;

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

/// SimulationBridge GDExtension 节点
/// 负责启动模拟、桥接模拟线程与 Godot 主线程
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SimulationBridge {
    base: Base<Node>,
    command_sender: Option<Sender<SimCommand>>,
    snapshot_receiver: Option<Receiver<WorldSnapshot>>,
    current_tick: i64,
    is_paused: bool,
    is_running: bool,
}

#[godot_api]
impl INode for SimulationBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            command_sender: None,
            snapshot_receiver: None,
            current_tick: 0,
            is_paused: false,
            is_running: false,
        }
    }

    fn ready(&mut self) {
        godot::global::print(&[Variant::from("SimulationBridge: 初始化完成")]);
        self.start_simulation();
    }

    fn physics_process(&mut self, _delta: f64) {
        // poll 通道接收 WorldSnapshot
        if let Some(receiver) = &self.snapshot_receiver {
            if let Ok(snapshot) = receiver.try_recv() {
                self.current_tick = snapshot.tick as i64;
            }
        }
    }
}

#[godot_api]
impl SimulationBridge {
    /// 启动模拟
    #[func]
    fn start_simulation(&mut self) {
        godot::global::print(&[Variant::from("SimulationBridge: 启动模拟...")]);

        // 创建通道
        let (tx, rx) = mpsc::channel::<WorldSnapshot>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<SimCommand>();

        self.snapshot_receiver = Some(rx);
        self.command_sender = Some(cmd_tx);
        self.is_running = true;

        // 启动后台模拟线程（使用真实的核心引擎）
        std::thread::spawn(move || {
            run_simulation(tx, cmd_rx);
        });

        godot::global::print(&[Variant::from("SimulationBridge: 模拟已启动")]);
    }

    /// 获取当前 tick
    #[func]
    fn get_tick(&self) -> i64 {
        self.current_tick
    }

    /// 获取 Agent 数量
    #[func]
    fn get_agent_count(&self) -> i64 {
        5
    }

    /// 暂停/继续模拟
    #[func]
    fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        godot::global::print(&[Variant::from(format!("SimulationBridge: 暂停状态 = {}", self.is_paused))]);
    }

    /// 调整动机
    #[func]
    fn adjust_motivation(&self, agent_id: String, dimension: i32, value: f32) {
        godot::global::print(&[Variant::from(format!(
            "SimulationBridge: 调整动机 agent={} dim={} value={}",
            agent_id, dimension, value
        ))]);
    }

    /// 注入偏好
    #[func]
    fn inject_preference(&self, agent_id: String, dimension: i32, boost: f32, duration: i32) {
        godot::global::print(&[Variant::from(format!(
            "SimulationBridge: 注入偏好 agent={} dim={} boost={} duration={}",
            agent_id, dimension, boost, duration
        ))]);
    }

    /// 设置 tick 间隔
    #[func]
    fn set_tick_interval(&self, seconds: f32) {
        godot::global::print(&[Variant::from(format!("SimulationBridge: 设置 tick 间隔={}秒", seconds))]);
    }
}

/// 运行模拟循环
fn run_simulation(tx: Sender<WorldSnapshot>, cmd_rx: Receiver<SimCommand>) {
    // 创建 Tokio 运行时
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        run_simulation_async(tx, cmd_rx).await;
    });
}

/// 异步运行模拟循环
async fn run_simulation_async(tx: Sender<WorldSnapshot>, cmd_rx: Receiver<SimCommand>) {
    // 创建世界种子
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
        initial_agents: 5,
        motivation_templates: std::collections::HashMap::from([
            ("gatherer".to_string(), [0.8, 0.4, 0.3, 0.2, 0.3, 0.2]),
            ("trader".to_string(), [0.5, 0.8, 0.4, 0.3, 0.7, 0.3]),
        ]),
        spawn_strategy: "scattered".to_string(),
        seed_peers: vec![],
        pressure_config: agentora_core::seed::PressureConfig::default(),
    };

    // 创建世界
    let mut world = World::new(&seed);

    // 创建初始 Agent
    create_initial_agents(&mut world);

    godot::global::print(&[Variant::from(format!("SimulationBridge: 世界已创建，{} 个 Agent", world.agents.len()))]);

    // 创建决策管道
    let pipeline = DecisionPipeline::new();

    // 模拟循环
    loop {
        // 检查命令
        if let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                SimCommand::Pause => {
                    // 暂停逻辑
                }
                SimCommand::SetTickInterval { seconds } => {
                    world.tick_interval = seconds as u32;
                }
                _ => {}
            }
        }

        // 推进 tick
        world.advance_tick();

        // Agent 决策和执行
        for agent_id in world.agents.keys().cloned().collect::<Vec<_>>() {
            let action = agent_decision(&pipeline, &world, &agent_id).await;
            world.apply_action(&agent_id, &action);
        }

        // 生成快照并发送
        let snapshot = world.snapshot();
        if tx.send(snapshot).is_err() {
            break;
        }

        // 等待 tick 间隔
        tokio::time::sleep(std::time::Duration::from_secs(world.tick_interval as u64)).await;
    }
}

/// Agent 决策逻辑
async fn agent_decision(pipeline: &DecisionPipeline, world: &World, agent_id: &AgentId) -> agentora_core::Action {
    let agent = world.agents.get(agent_id).unwrap();

    // 构建世界状态快照
    let mut terrain_at = HashMap::new();
    let mut resources_at = HashMap::new();

    // 收集视野内的地形和资源（简化：只收集相邻格子）
    let vision_radius = 5u32;
    for dx in 0..=vision_radius {
        for dy in 0..=vision_radius {
            let nx = agent.position.x.saturating_add(dx);
            let ny = agent.position.y.saturating_add(dy);
            let pos = Position::new(nx, ny);

            // 跳过中心点（Agent 自己的位置）
            if pos == agent.position {
                continue;
            }

            let terrain = world.map.get_terrain(pos);
            terrain_at.insert(pos, terrain);

            if let Some(res) = world.resources.get(&pos) {
                resources_at.insert(pos, res.resource_type);
            }
        }
    }

    let world_state = WorldState {
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
        terrain_at,
        existing_agents: world.agents.keys().cloned().collect(),
        resources_at,
    };

    // 生成 Spark
    let satisfaction = [0.5; 6]; // 简化实现
    let spark = Spark::from_gap(&agent.motivation, &satisfaction);

    // 执行决策管道
    let result = pipeline.execute(&agent.id, &agent.motivation, &spark, &world_state).await;

    // 转换为 Action
    agentora_core::Action {
        reasoning: result.selected_action.reasoning,
        action_type: result.selected_action.action_type,
        target: result.selected_action.target,
        params: result.selected_action.params.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
        motivation_delta: result.selected_action.motivation_delta,
    }
}

/// 创建初始 Agent
fn create_initial_agents(world: &mut World) {
    let positions = [
        (128u32, 128u32),
        (120, 128),
        (136, 128),
        (128, 120),
        (128, 136),
    ];

    for (i, (x, y)) in positions.iter().enumerate() {
        let agent = Agent::new(
            AgentId::default(),
            format!("Agent {}", i),
            Position::new(*x, *y),
        );
        world.agents.insert(agent.id.clone(), agent);
    }
}

/// GDExtension 库定义
struct AgentoraExtension;

#[gdextension(entry_symbol = agentora_bridge_init)]
unsafe impl ExtensionLibrary for AgentoraExtension {}
