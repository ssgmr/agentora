//! 世界模型：256×256 地图、地形、区域、资源、环境压力

pub mod map;
pub mod region;
pub mod resource;
pub mod pressure;
pub mod structure;
pub mod generator;
pub mod actions;
pub mod snapshot;
pub mod feedback;
pub mod tick;
pub mod milestones;
pub mod legacy;
pub mod vision;
pub mod types;
pub mod action_result;

// 重导出辅助类型
pub use types::{MilestoneType, Milestone, TradeStatus, PendingTrade, DialogueLog, DialogueMessage};
pub use action_result::{ActionResultSchema, FieldChange, ActionSuggestion};

use crate::seed::WorldSeed;
use crate::agent::Agent;
use crate::agent::inventory::get_config as get_inventory_config;
use crate::agent::ShadowAgent;
use crate::types::{AgentId, Position, ActionType, Action, TerrainType, StructureType};
use crate::world::legacy::Legacy;
use crate::strategy::decay::{decay_all_strategies, check_deprecation, auto_delete_deprecated};
use crate::strategy::create::{should_create_strategy, create_strategy, scan_strategy_content};
use crate::snapshot::{NarrativeEvent, NarrativeChannel, AgentSource};
use crate::decision::SparkType;
use crate::simulation::{Delta, DeltaEnvelope, SimMode};
use std::collections::HashMap;

/// 世界状态
pub struct World {
    pub tick: u64,
    pub tick_interval: u32, // 秒
    pub map: map::CellGrid,
    pub regions: HashMap<u32, region::Region>,
    pub resources: HashMap<Position, resource::ResourceNode>,
    pub structures: HashMap<Position, structure::Structure>,
    /// 本地 Agent（本 peer 负责决策的 Agent）
    pub agents: HashMap<AgentId, Agent>,
    /// 远程 Agent 影子状态（P2P 模式下，其他 peer 负责决策的 Agent）
    pub remote_agents: HashMap<AgentId, ShadowAgent>,
    /// 本地 Agent ID 集合（P2P 模式下用于区分 local/remote）
    pub local_agent_ids: Option<std::collections::HashSet<AgentId>>,
    pub pressure_pool: Vec<pressure::PressureEvent>,
    pub legacies: Vec<Legacy>,
    /// 当前 tick 各 Agent 的动作
    pub current_actions: HashMap<AgentId, String>,
    /// 当前 tick 的叙事事件
    pub tick_events: Vec<NarrativeEvent>,
    /// 待处理的交易
    pub pending_trades: Vec<PendingTrade>,
    /// 对话日志
    pub dialogue_logs: Vec<DialogueLog>,
    /// 位置到 Agent ID 的反向索引，用于空间查询（包含 local + remote）
    pub agent_positions: HashMap<Position, Vec<AgentId>>,
    /// 下次压力事件触发 tick
    pub next_pressure_tick: u64,
    /// 资源产出乘数（如干旱时 Water → 0.5）
    pub pressure_multiplier: HashMap<String, f32>,
    /// 文明里程碑
    pub milestones: Vec<Milestone>,
    /// 累计交易次数
    pub total_trades: u32,
    /// 累计攻击次数
    pub total_attacks: u32,
    /// 累计遗产交互次数
    pub total_legacy_interacts: u32,
    /// 交易超时 tick 数（超过此时间自动取消交易，解冻资源）
    pub trade_timeout_ticks: u64,
}

impl World {
    pub fn new(seed: &WorldSeed) -> Self {
        let mut world = Self {
            tick: 0,
            tick_interval: 5, // 默认 5 秒
            map: map::CellGrid::new(seed.map_size[0], seed.map_size[1]),
            regions: HashMap::new(),
            resources: HashMap::new(),
            structures: HashMap::new(),
            agents: HashMap::new(),
            remote_agents: HashMap::new(),
            local_agent_ids: None, // 集中式模式下为 None
            pressure_pool: Vec::new(),
            legacies: Vec::new(),
            current_actions: HashMap::new(),
            tick_events: Vec::new(),
            pending_trades: Vec::new(),
            dialogue_logs: Vec::new(),
            agent_positions: HashMap::new(),
            next_pressure_tick: {
                use rand::Rng;
                rand::thread_rng().gen_range(40..80)
            },
            pressure_multiplier: HashMap::new(),
            milestones: Vec::new(),
            total_trades: 0,
            total_attacks: 0,
            total_legacy_interacts: 0,
            trade_timeout_ticks: 50, // 默认值，可由 Simulation 配置覆盖
        };

        // 生成地形
        Self::generate_terrain(&mut world.map, &seed);

        // 生成区域
        Self::generate_regions(&mut world.regions, seed);

        // 生成资源节点
        Self::generate_resources(&mut world.map, &mut world.resources, seed);

        // 生成初始 Agent（P2P 模式下可能跳过）
        if !seed.skip_initial_agents {
            let map_size = world.map.size();
            Self::generate_agents(&mut world, map_size, seed);
        }

        world
    }

    /// 插入 Agent 并初始化位置索引
    pub fn insert_agent_at(&mut self, agent_id: AgentId, agent: Agent) {
        let pos = agent.position;
        self.agent_positions.entry(pos).or_default().push(agent_id.clone());
        self.agents.insert(agent_id, agent);
    }

    /// 获取指定位置的地形类型
    pub fn terrain_at(&self, pos: Position) -> TerrainType {
        self.map.get_terrain(pos)
    }

    /// 获取当前 tick
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// 记录叙事事件
    fn record_event(&mut self, agent_id: &AgentId, agent_name: &str, event_type: &str, description: &str, color_code: &str) {
        self.tick_events.push(NarrativeEvent {
            tick: self.tick,
            agent_id: agent_id.as_str().to_string(),
            agent_name: agent_name.to_string(),
            event_type: event_type.to_string(),
            description: description.to_string(),
            color_code: color_code.to_string(),
            channel: NarrativeChannel::Local,
            agent_source: AgentSource::Local,
        });
    }

    /// 查找同一格的存活 Agent（排除自己）
    #[allow(dead_code)]
    fn find_alive_at(&self, agent_id: &AgentId) -> Vec<AgentId> {
        let agent = match self.agents.get(agent_id) {
            Some(a) => a,
            None => return vec![],
        };
        let pos = agent.position;
        self.agents
            .values()
            .filter(|a| a.is_alive && a.id != *agent_id && a.position == pos)
            .map(|a| a.id.clone())
            .collect()
    }

    /// 查找待处理交易
    #[allow(dead_code)]
    fn find_pending_trade(&self, proposer_id: &AgentId, acceptor_id: &AgentId) -> Option<usize> {
        self.pending_trades.iter().position(|t| {
            (t.proposer_id == *proposer_id && t.acceptor_id == *acceptor_id)
                || (t.proposer_id == *acceptor_id && t.acceptor_id == *proposer_id)
        })
    }

    /// 计算有效库存上限（受 Warehouse 影响）
    pub fn effective_inventory_limit_for(&self, agent_position: Position) -> usize {
        let config = get_inventory_config();
        let base: u32 = config.max_stack_size;
        let multiplier = config.warehouse_limit_multiplier;
        for (_, structure) in &self.structures {
            if structure.structure_type == StructureType::Warehouse {
                if agent_position.manhattan_distance(&structure.position) <= 1 {
                    return (base * multiplier) as usize; // 仓库附近上限翻倍
                }
            }
        }
        base as usize
    }

    /// 推进 tick
    pub fn advance_tick(&mut self) {
        self.tick += 1;

        // 生存消耗 tick：satiety/hydration 衰减，耗尽掉血
        self.survival_consumption_tick();

        // 建筑效果 tick
        self.structure_effects_tick();

        // 更新所有存活 Agent 的临时偏好
        for (_, agent) in self.agents.iter_mut() {
            if agent.is_alive {
                agent.tick_preferences();
            }
        }

        // 环境压力 tick
        self.pressure_tick();

        // 3.2 检查 Agent 死亡并产生遗产
        self.check_agent_death();

        // 遗产衰减
        self.decay_legacies();

        // 交易超时检查
        self.check_trade_timeout();

        // 策略衰减（每 50 tick）
        if self.tick % 50 == 0 {
            for (_, agent) in self.agents.iter_mut() {
                let _ = decay_all_strategies(&agent.strategies, self.tick as u32);
                let _ = check_deprecation(&agent.strategies);
                let _ = auto_delete_deprecated(&agent.strategies, self.tick as u32);
            }
        }

        // 里程碑检查
        self.check_milestones();
    }

    /// 应用动作到世界（路由模式：校验 → 路由 → 统一处理结果）
    ///
    /// # 参数
    /// - `spark_type`: 当前决策情境的 SparkType，用于策略创建（传入 None 时使用 fallback 推断）
    pub fn apply_action(&mut self, agent_id: &AgentId, action: &Action, spark_type: Option<SparkType>) -> ActionResult {
        // 前置校验
        if !self.agents.contains_key(agent_id) {
            return ActionResult::InvalidAgent;
        }
        if !self.agents[agent_id].is_alive {
            return ActionResult::AgentDead;
        }

        // 记录旧位置
        let old_position = self.agents.get(agent_id).map(|a| a.position);

        // 记录当前动作的思考内容（reasoning，用于 UI 显示 Agent 的决策思路）
        self.current_actions.insert(agent_id.clone(), action.reasoning.clone());

        // 路由到具体 handler（通过 ActionExecutor）
        let result = self.execute_action(agent_id, action);

        // 失败时生成错误叙事
        if let ActionResult::Blocked(ref reason) = result {
            self.record_error_narrative(agent_id, &action.action_type, reason);
        }

        // ===== 统一生成反馈 =====
        let feedback = self.generate_action_feedback(&result, &action.action_type, old_position);
        self.agents.get_mut(agent_id).unwrap().last_action_result = Some(feedback);

        // 记录上一次动作类型（简短描述）
        self.agents.get_mut(agent_id).unwrap().last_action_type = Some(Self::action_type_to_short_desc(&action.action_type));

        // 经验值积累
        let xp_reward = match &action.action_type {
            ActionType::Gather { .. } => 15,
            ActionType::Build { .. } => 25,
            ActionType::TradeOffer { .. } => 10,
            ActionType::Attack { .. } => 20,
            ActionType::AllyPropose { .. } | ActionType::AllyAccept { .. } => 15,
            ActionType::InteractLegacy { .. } => 20,
            ActionType::MoveToward { .. } => 1,
            ActionType::Wait => 0,
            ActionType::Eat => 3,
            ActionType::Drink => 3,
            ActionType::Talk { .. } => 2,
            ActionType::TradeAccept { .. } => 10,
            ActionType::TradeReject { .. } => 0,
            ActionType::AllyReject { .. } => 0,
        };
        let leveled_up = if xp_reward > 0 {
            self.agents.get_mut(agent_id).unwrap().add_experience(xp_reward)
        } else {
            false
        };

        if leveled_up {
            let agent = self.agents.get(agent_id).unwrap();
            self.tick_events.push(NarrativeEvent {
                tick: self.tick,
                agent_id: agent_id.as_str().to_string(),
                agent_name: agent.name.clone(),
                event_type: "level_up".to_string(),
                description: format!("{} 升级到等级 {}！HP上限+10", agent.name, agent.level),
                color_code: "#FF6B35".to_string(),
                channel: NarrativeChannel::Local,
                agent_source: AgentSource::Local,
            });
        }

        // 策略创建触发检查
        let is_success = matches!(result, ActionResult::SuccessWithDetail(_) | ActionResult::AlreadyAtPosition(_));
        if is_success {
            let candidate_count = action.params.get("candidate_count")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(3);
            if should_create_strategy(is_success, candidate_count) {
                let agent = self.agents.get_mut(agent_id).unwrap();
                // 使用传入的 spark_type，确保与决策检索时使用相同的推断逻辑
                let spark_type = spark_type.unwrap_or_else(|| SparkType::Explore);
                let _ = scan_strategy_content(&action.reasoning);
                let _ = create_strategy(&agent.strategies, spark_type, self.tick as u32, &action.reasoning);
            }
        }

        // 维护 agent_positions 反向索引
        if let (Some(old_pos), Some(agent)) = (old_position, self.agents.get(agent_id)) {
            if old_pos != agent.position {
                if let Some(ids) = self.agent_positions.get_mut(&old_pos) {
                    ids.retain(|id| *id != *agent_id);
                    if ids.is_empty() {
                        self.agent_positions.remove(&old_pos);
                    }
                }
                self.agent_positions.entry(agent.position).or_default().push(agent_id.clone());
            }
        }

        result
    }

    // ===== P2P 模式相关方法 =====

    /// 设置运行模式
    pub fn set_sim_mode(&mut self, mode: &SimMode) {
        match mode {
            SimMode::Centralized => {
                // 集中式模式：所有 Agent 都是 local
                self.local_agent_ids = None;
            }
            SimMode::P2P { .. } => {
                // P2P 模式：动态创建的 Agent 自动加入 local_agent_ids
                // 初始化为空集合，generate_local_agent 会添加
                self.local_agent_ids = Some(std::collections::HashSet::new());
            }
        }
    }

    /// 判断 AgentId 是否为本地 Agent
    pub fn is_local_agent(&self, agent_id: &AgentId) -> bool {
        match &self.local_agent_ids {
            None => true, // 集中式模式下所有都是 local
            Some(ids) => ids.contains(agent_id),
        }
    }

    /// 获取所有 Agent（local + remote），用于渲染
    ///
    /// 返回 (local_agents, remote_agents) 元组
    pub fn all_agents(&self) -> (&HashMap<AgentId, Agent>, &HashMap<AgentId, ShadowAgent>) {
        (&self.agents, &self.remote_agents)
    }

    /// 应用远程 Delta 更新影子状态
    ///
    /// 过滤本地回环，只处理来自其他 peer 的 Delta
    pub fn apply_remote_delta(&mut self, envelope: &DeltaEnvelope, current_tick: u64) {
        // 过滤本地回环（如果 source_peer_id 为空，视为本地产生）
        if envelope.source_peer_id.is_none() {
            return;
        }

        match &envelope.delta {
            Delta::AgentStateChanged { agent_id, state, change_hint, .. } => {
                let id = AgentId::new(agent_id.clone());

                // 如果是本地 Agent，跳过（本地回环过滤）
                if self.is_local_agent(&id) {
                    tracing::trace!("[World] 跳过本地 Agent delta: {}", agent_id);
                    return;
                }

                // 创建或更新影子 Agent
                if let Some(shadow) = self.remote_agents.get_mut(&id) {
                    shadow.apply_delta(&envelope.delta);
                    shadow.last_seen_tick = current_tick;
                    tracing::trace!("[World] 更新远程影子 Agent: {}", agent_id);
                } else {
                    // 创建新影子
                    let new_shadow = ShadowAgent::from_state(
                        state,
                        &envelope.source_peer_id.clone().unwrap_or_default(),
                        current_tick
                    );
                    self.remote_agents.insert(id, new_shadow);
                    tracing::info!("[World] 创建远程影子 Agent: {}", agent_id);
                }
            }
            Delta::WorldEvent(_) => {
                // WorldEvent 不涉及 Agent 状态更新，暂不处理
                tracing::trace!("[World] 收到 WorldEvent，暂不处理影子状态");
            }
        }
    }

    /// 清理过期影子 Agent
    pub fn cleanup_expired_shadows(&mut self, current_tick: u64, timeout_ticks: u64) {
        let expired: Vec<AgentId> = self.remote_agents.iter()
            .filter(|(_, shadow)| shadow.is_expired(current_tick, timeout_ticks))
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired {
            self.remote_agents.remove(id);
            tracing::info!("[World] 清理过期影子 Agent: {}", id.as_str());
        }
    }

    /// 只对本地 Agent 执行生存消耗（P2P 模式）
    ///
    /// 集中式模式下行为与 advance_tick 相同
    pub fn advance_tick_local_only(&mut self) {
        self.tick += 1;

        // 生存消耗 tick：只对本地 Agent
        if let Some(ref local_ids) = self.local_agent_ids {
            // P2P 模式：只对 local_agents 执行生存消耗
            for agent_id in local_ids {
                if let Some(agent) = self.agents.get_mut(agent_id) {
                    if !agent.is_alive {
                        continue;
                    }
                    // 每 tick 衰减 1 点
                    agent.satiety = agent.satiety.saturating_sub(1);
                    agent.hydration = agent.hydration.saturating_sub(1);

                    // 饱食度耗尽：HP -1/tick
                    if agent.satiety == 0 {
                        agent.health = agent.health.saturating_sub(1);
                    }
                    // 水分度耗尽：HP -1/tick
                    if agent.hydration == 0 {
                        agent.health = agent.health.saturating_sub(1);
                    }

                    // 检查死亡
                    if agent.health <= 0 {
                        agent.is_alive = false;
                        self.tick_events.push(NarrativeEvent {
                            tick: self.tick,
                            agent_id: agent_id.as_str().to_string(),
                            agent_name: agent.name.clone(),
                            event_type: "death".to_string(),
                            description: format!("{} 因饥饿或脱水而死", agent.name),
                            color_code: "#FF0000".to_string(),
                            channel: NarrativeChannel::World, // 死亡是世界频道
                            agent_source: AgentSource::Local,
                        });
                    }
                }
            }
        } else {
            // 集中式模式：原有行为
            self.survival_consumption_tick();
        }

        // 建筑效果 tick（所有 Agent 受益）
        self.structure_effects_tick();

        // 更新本地 Agent 的临时偏好
        for (_, agent) in self.agents.iter_mut() {
            if agent.is_alive {
                agent.tick_preferences();
            }
        }

        // 环境压力 tick
        self.pressure_tick();

        // 检查 Agent 死亡并产生遗产（只检查本地 Agent）
        self.check_agent_death();

        // 遗产衰减
        self.decay_legacies();

        // 交易超时检查
        self.check_trade_timeout();

        // 策略衰减（每 50 tick）
        if self.tick % 50 == 0 {
            for (_, agent) in self.agents.iter_mut() {
                let _ = decay_all_strategies(&agent.strategies, self.tick as u32);
                let _ = check_deprecation(&agent.strategies);
                let _ = auto_delete_deprecated(&agent.strategies, self.tick as u32);
            }
        }

        // 里程碑检查
        self.check_milestones();
    }

    /// 持久化世界状态
    pub async fn persist(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: 实现持久化
        Ok(())
    }
}

/// 动作执行结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    /// 成功，携带详细信息供反馈生成
    SuccessWithDetail(String),
    /// 失败，携带原因
    Blocked(String),
    /// Agent 不存在
    InvalidAgent,
    /// Agent 已死亡
    AgentDead,
    /// 超出边界
    OutOfBounds,
    /// 未实现
    NotImplemented,
    /// 已在目标位置（特殊成功情况）
    AlreadyAtPosition(String),
}
