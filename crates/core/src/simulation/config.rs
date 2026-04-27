//! 模拟配置
//!
//! 从 config/sim.toml 加载模拟参数

use serde::Deserialize;

/// 模拟模式
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SimMode {
    /// 集中式模式：所有 Agent 在本地运行
    Centralized,
    /// P2P 模式：每节点运行自己的 Agent，远程 Agent 通过影子状态同步
    P2P {
        /// P2P 区域大小（格子），用于区域订阅
        #[serde(default = "default_region_size")]
        region_size: u32,
    },
}

fn default_region_size() -> u32 { 32 }

impl Default for SimMode {
    fn default() -> Self {
        SimMode::Centralized
    }
}

/// 模拟配置 — 从 config/sim.toml 加载，支持热改
#[derive(Debug, Clone, Deserialize)]
pub struct SimConfigFile {
    simulation: Option<SimSection>,
    inventory: Option<InvSection>,
    p2p: Option<P2PSection>,
}

#[derive(Debug, Clone, Deserialize)]
struct SimSection {
    initial_agent_count: Option<usize>,
    npc_count: Option<usize>,
    tick_interval_secs: Option<u64>,
    npc_decision_interval_secs: Option<u64>,
    player_decision_interval_secs: Option<u64>,
    vision_radius: Option<u32>,
    trade_timeout_ticks: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct InvSection {
    max_slots: Option<usize>,
    max_stack_size: Option<u32>,
    warehouse_limit_multiplier: Option<u32>,
}

/// P2P 配置段
#[derive(Debug, Clone, Deserialize)]
struct P2PSection {
    mode: Option<String>,
    region_size: Option<u32>,
    port: Option<u16>,
    seed_peer: Option<String>,
}

/// 模拟配置（运行时使用的扁平结构）
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// 运行模式（集中式/P2P）
    pub mode: SimMode,
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
    /// 交易超时 tick 数（超过此时间自动取消交易，解冻资源）
    pub trade_timeout_ticks: u64,
    /// 背包格子数量（不同资源类型的种类上限）
    pub inventory_max_slots: usize,
    /// 单个资源堆叠上限
    pub inventory_max_stack_size: u32,
    /// 仓库附近堆叠上限倍率
    pub inventory_warehouse_multiplier: u32,
    /// P2P 端口（仅 P2P 模式）
    pub p2p_port: u16,
    /// 种子节点地址（仅 P2P 模式）
    pub seed_peer: Option<String>,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            mode: SimMode::default(),
            initial_agent_count: 3,
            npc_count: 3,
            tick_interval_secs: 5,
            npc_decision_interval_secs: 1,
            player_decision_interval_secs: 2,
            vision_radius: 10,
            trade_timeout_ticks: 50,
            inventory_max_slots: 20,
            inventory_max_stack_size: 20,
            inventory_warehouse_multiplier: 2,
            p2p_port: 4001,
            seed_peer: None,
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
                            if let Some(v) = sim.trade_timeout_ticks { cfg.trade_timeout_ticks = v; }
                        }
                        if let Some(inv) = file.inventory {
                            if let Some(v) = inv.max_slots { cfg.inventory_max_slots = v; }
                            if let Some(v) = inv.max_stack_size { cfg.inventory_max_stack_size = v; }
                            if let Some(v) = inv.warehouse_limit_multiplier { cfg.inventory_warehouse_multiplier = v; }
                        }
                        // 解析 P2P 配置
                        if let Some(p2p) = file.p2p {
                            if let Some(mode_str) = p2p.mode {
                                if mode_str == "p2p" {
                                    let region_size = p2p.region_size.unwrap_or(32);
                                    cfg.mode = SimMode::P2P { region_size };
                                }
                            }
                            if let Some(v) = p2p.port { cfg.p2p_port = v; }
                            if let Some(v) = p2p.seed_peer {
                                if !v.is_empty() {
                                    cfg.seed_peer = Some(v);
                                }
                            }
                        }
                        tracing::info!("[SimConfig] 配置加载成功 [mode={} agents={} npc={} tick_interval={}s npc_interval={}s player_interval={}s vision_radius={} trade_timeout={} inv_slots={} inv_stack={} warehouse_mult={} p2p_port={} seed_peer={:?}]",
                            if matches!(cfg.mode, SimMode::P2P { .. }) { "P2P" } else { "Centralized" },
                            cfg.initial_agent_count, cfg.npc_count, cfg.tick_interval_secs, cfg.npc_decision_interval_secs, cfg.player_decision_interval_secs, cfg.vision_radius, cfg.trade_timeout_ticks,
                            cfg.inventory_max_slots, cfg.inventory_max_stack_size, cfg.inventory_warehouse_multiplier,
                            cfg.p2p_port, cfg.seed_peer);
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