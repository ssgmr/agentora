//! 模拟运行器
//!
//! 在独立线程中运行 Simulation，处理命令

use std::sync::mpsc::{Sender, Receiver};
use agentora_core::simulation::{SimConfig, AgentDelta, Simulation};
use agentora_core::simulation::agent_loop::NarrativeEvent;
use agentora_core::WorldSeed;
use agentora_ai::{LlmProvider, config::LlmConfig};

use crate::bridge::SimCommand;

/// 模拟入口函数（在独立线程中运行）
pub fn run_simulation_with_api(
    snapshot_tx: Sender<agentora_core::WorldSnapshot>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    llm_provider: Option<Box<dyn LlmProvider>>,
    llm_config: LlmConfig,
) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_simulation_async_with_api(snapshot_tx, delta_tx, narrative_tx, cmd_rx, llm_provider, llm_config).await;
    });
}

/// 异步模拟主函数（使用 Simulation 结构体）
async fn run_simulation_async_with_api(
    snapshot_tx: Sender<agentora_core::WorldSnapshot>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    cmd_rx: Receiver<SimCommand>,
    llm_provider: Option<Box<dyn LlmProvider>>,
    llm_config: LlmConfig,
) {
    // 加载模拟配置
    let sim_config = SimConfig::load("../config/sim.toml");

    // 从配置文件加载世界种子
    let mut seed = WorldSeed::load("../worldseeds/default.toml")
        .unwrap_or_else(|e| {
            tracing::error!("加载世界种子失败: {}，使用默认配置", e);
            WorldSeed::default()
        });
    seed.initial_agents = sim_config.initial_agent_count as u32;

    // 创建 Simulation 实例
    let mut simulation = Simulation::new(
        sim_config,
        seed,
        llm_provider,
        &llm_config,
        snapshot_tx,
        delta_tx,
        narrative_tx,
    );

    // 启动模拟（异步）
    simulation.start().await;

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
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}