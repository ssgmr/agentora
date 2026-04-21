//! 模拟配置
//!
//! 从 config/sim.toml 加载模拟参数

use serde::Deserialize;

/// 模拟配置 — 从 config/sim.toml 加载，支持热改
#[derive(Debug, Clone, Deserialize)]
pub struct SimConfigFile {
    simulation: Option<SimSection>,
    inventory: Option<InvSection>,
}

#[derive(Debug, Clone, Deserialize)]
struct SimSection {
    initial_agent_count: Option<usize>,
    npc_count: Option<usize>,
    tick_interval_secs: Option<u64>,
    npc_decision_interval_secs: Option<u64>,
    player_decision_interval_secs: Option<u64>,
    vision_radius: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct InvSection {
    max_slots: Option<usize>,
    max_stack_size: Option<u32>,
    warehouse_limit_multiplier: Option<u32>,
}

/// 模拟配置（运行时使用的扁平结构）
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// LLM 驱动 Agent 数量（走完整决策管道，耗时较长）
    pub initial_agent_count: usize,
    /// NPC 数量（规则引擎快速决策，不阻塞）
    pub npc_count: usize,
    /// 世界 tick 间隔（秒），控制状态衰减和世界时间推进速度
    pub tick_interval_secs: u64,
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
            tick_interval_secs: 5,
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
    pub fn load(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                match toml::from_str::<SimConfigFile>(&content) {
                    Ok(file) => {
                        let mut cfg = Self::default();
                        if let Some(sim) = file.simulation {
                            if let Some(v) = sim.initial_agent_count { cfg.initial_agent_count = v; }
                            if let Some(v) = sim.npc_count { cfg.npc_count = v; }
                            if let Some(v) = sim.tick_interval_secs { cfg.tick_interval_secs = v; }
                            if let Some(v) = sim.npc_decision_interval_secs { cfg.npc_decision_interval_secs = v; }
                            if let Some(v) = sim.player_decision_interval_secs { cfg.player_decision_interval_secs = v; }
                            if let Some(v) = sim.vision_radius { cfg.vision_radius = v; }
                        }
                        if let Some(inv) = file.inventory {
                            if let Some(v) = inv.max_slots { cfg.inventory_max_slots = v; }
                            if let Some(v) = inv.max_stack_size { cfg.inventory_max_stack_size = v; }
                            if let Some(v) = inv.warehouse_limit_multiplier { cfg.inventory_warehouse_multiplier = v; }
                        }
                        tracing::info!("[SimConfig] 配置加载成功 [agents={} npc={} tick_interval={}s npc_interval={}s player_interval={}s vision_radius={} inv_slots={} inv_stack={} warehouse_mult={}]",
                            cfg.initial_agent_count, cfg.npc_count, cfg.tick_interval_secs, cfg.npc_decision_interval_secs, cfg.player_decision_interval_secs, cfg.vision_radius,
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