//! 世界模型：256×256 地图、地形、区域、资源、环境压力

pub mod map;
pub mod region;
pub mod resource;
pub mod pressure;
pub mod structure;
pub mod generator;
pub mod actions;

use crate::seed::WorldSeed;
use crate::agent::Agent;
use crate::types::{AgentId, Position, ActionType, Action, TerrainType, ResourceType};
use crate::legacy::Legacy;
use crate::strategy::decay::{decay_all_strategies, check_deprecation, auto_delete_deprecated};
use crate::strategy::create::{should_create_strategy, create_strategy, scan_strategy_content};
use crate::snapshot::NarrativeEvent;
use crate::strategy::motivation_link::{on_strategy_success, on_strategy_failure};
use crate::decision::SparkType;
use std::collections::HashMap;

/// 世界状态
pub struct World {
    pub tick: u64,
    pub tick_interval: u32, // 秒
    pub map: map::CellGrid,
    pub regions: HashMap<u32, region::Region>,
    pub resources: HashMap<Position, resource::ResourceNode>,
    pub structures: HashMap<Position, structure::Structure>,
    pub agents: HashMap<AgentId, Agent>,
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
    /// 位置到 Agent ID 的反向索引，用于空间查询
    pub agent_positions: HashMap<Position, Vec<AgentId>>,
}

// ===== 辅助类型 =====

/// 交易状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeStatus {
    Pending,
    Accepted,
    Rejected,
}

/// 待处理交易
#[derive(Debug, Clone)]
pub struct PendingTrade {
    pub proposer_id: AgentId,
    pub acceptor_id: AgentId,
    pub offer_resources: HashMap<String, u32>,
    pub want_resources: HashMap<String, u32>,
    pub status: TradeStatus,
    pub tick_created: u64,
}

/// 对话日志
#[derive(Debug, Clone)]
pub struct DialogueLog {
    pub agent_a: AgentId,
    pub agent_b: AgentId,
    pub messages: Vec<DialogueMessage>,
    pub tick_started: u64,
    pub is_active: bool,
}

/// 对话消息
#[derive(Debug, Clone)]
pub struct DialogueMessage {
    pub speaker_id: AgentId,
    pub content: String,
    pub tick: u64,
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
            pressure_pool: Vec::new(),
            legacies: Vec::new(),
            current_actions: HashMap::new(),
            tick_events: Vec::new(),
            pending_trades: Vec::new(),
            dialogue_logs: Vec::new(),
            agent_positions: HashMap::new(),
        };

        // 生成地形
        Self::generate_terrain(&mut world.map, &seed);

        // 生成区域
        Self::generate_regions(&mut world.regions, seed);

        // 生成资源节点
        Self::generate_resources(&mut world.map, &mut world.resources, seed);

        // 生成初始 Agent
        let map_size = world.map.size();
        Self::generate_agents(&mut world, map_size, seed);

        world
    }

    /// 插入 Agent 并初始化位置索引
    pub fn insert_agent_at(&mut self, agent_id: AgentId, agent: Agent) {
        let pos = agent.position;
        self.agent_positions.entry(pos).or_default().push(agent_id.clone());
        self.agents.insert(agent_id, agent);
    }

    /// 生成地形
    fn generate_terrain(map: &mut map::CellGrid, seed: &WorldSeed) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let (width, height) = map.size();

        for y in 0..height {
            for x in 0..width {
                let terrain = Self::random_terrain(&mut rng, &seed.terrain_ratio);
                map.set_terrain(Position::new(x, y), terrain);
            }
        }
    }

    /// 生成区域划分
    fn generate_regions(regions: &mut HashMap<u32, region::Region>, seed: &WorldSeed) {
        let (width, height) = (seed.map_size[0], seed.map_size[1]);
        let region_size = seed.region_size;

        let region_count_x = width / region_size;
        let region_count_y = height / region_size;

        for ry in 0..region_count_y {
            for rx in 0..region_count_x {
                let id = region::Region::position_to_region_id(rx * region_size, ry * region_size, region_size);
                let region = region::Region::new(
                    id,
                    rx * region_size + region_size / 2,
                    ry * region_size + region_size / 2,
                    region_size,
                );
                regions.insert(id, region);
            }
        }
    }

    /// 生成资源节点
    fn generate_resources(map: &map::CellGrid, resources: &mut HashMap<Position, resource::ResourceNode>, seed: &WorldSeed) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let (width, height) = map.size();

        let resource_count = (width * height * seed.resource_density as u32) as usize;
        let resource_types = [ResourceType::Iron, ResourceType::Food, ResourceType::Wood, ResourceType::Water, ResourceType::Stone];

        for _ in 0..resource_count {
            let x = rng.gen_range(0..width);
            let y = rng.gen_range(0..height);
            let pos = Position::new(x, y);

            // 只在可通行地形放置资源
            if map.get_terrain(pos).is_passable() {
                let resource_type = resource_types[rng.gen_range(0..resource_types.len())];
                let node = resource::ResourceNode::new(pos, resource_type, rng.gen_range(50..200));
                resources.insert(pos, node);
            }
        }
    }

    /// 生成初始 Agent
    fn generate_agents(world: &mut World, map_size: (u32, u32), seed: &WorldSeed) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let (width, height) = map_size;

        let templates: Vec<&[f32; 6]> = seed.motivation_templates.values().collect();
        let template_names: Vec<&str> = seed.motivation_templates.keys().map(|s| s.as_str()).collect();

        for i in 0..seed.initial_agents {
            // 找一个可通行位置（出生在地图中心附近，确保相机能看到）
            let mut pos;
            let cx = width / 2;
            let cy = height / 2;
            let spawn_radius = 16u32; // 中心 32x32 区域内出生
            loop {
                let x = rng.gen_range(cx.saturating_sub(spawn_radius)..(cx + spawn_radius).min(width));
                let y = rng.gen_range(cy.saturating_sub(spawn_radius)..(cy + spawn_radius).min(height));
                pos = Position::new(x, y);
                if world.map.get_terrain(pos).is_passable() {
                    break;
                }
            }

            let template_idx = rng.gen_range(0..templates.len().max(1));
            let name = format!("{}_{}", template_names.get(template_idx).unwrap_or(&"Agent"), i + 1);

            let mut agent = Agent::new(AgentId::new(uuid::Uuid::new_v4().to_string()), name, pos);

            // 应用动机模板
            if let Some(template) = templates.get(template_idx) {
                agent.motivation = crate::motivation::MotivationVector::from_array(**template);
            }

            world.insert_agent_at(agent.id.clone(), agent);
        }
    }

    /// 随机选择地形
    fn random_terrain(rng: &mut impl rand::Rng, ratios: &std::collections::HashMap<String, f32>) -> TerrainType {
        let total: f32 = ratios.values().sum();
        let roll = rng.gen::<f32>() * total;
        let mut accumulated = 0.0;

        for (name, ratio) in ratios {
            accumulated += ratio;
            if roll < accumulated {
                return Self::terrain_from_name(name);
            }
        }
        TerrainType::Plains
    }

    fn terrain_from_name(name: &str) -> TerrainType {
        match name {
            "plains" => TerrainType::Plains,
            "forest" => TerrainType::Forest,
            "mountain" => TerrainType::Mountain,
            "water" => TerrainType::Water,
            "desert" => TerrainType::Desert,
            _ => TerrainType::Plains,
        }
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
        });
    }

    /// 查找同一格的存活 Agent（排除自己）
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
    fn find_pending_trade(&self, proposer_id: &AgentId, acceptor_id: &AgentId) -> Option<usize> {
        self.pending_trades.iter().position(|t| {
            (t.proposer_id == *proposer_id && t.acceptor_id == *acceptor_id)
                || (t.proposer_id == *acceptor_id && t.acceptor_id == *proposer_id)
        })
    }

    /// 推进 tick
    pub fn advance_tick(&mut self) {
        self.tick += 1;

        // 动机惯性衰减（向中性值 0.5 收敛）
        for (_, agent) in self.agents.iter_mut() {
            agent.motivation.decay();
        }

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

        // 策略衰减（每 50 tick）
        if self.tick % 50 == 0 {
            for (_, agent) in self.agents.iter_mut() {
                let _ = decay_all_strategies(&agent.strategies, self.tick as u32);
                let _ = check_deprecation(&agent.strategies);
                let _ = auto_delete_deprecated(&agent.strategies, self.tick as u32);
            }
        }
    }

    /// 检查 Agent 死亡并产生遗产（任务 3.2）
    fn check_agent_death(&mut self) {
        let dead_agent_ids: Vec<AgentId> = self.agents
            .iter()
            .filter(|(_, agent)| agent.is_alive && (agent.age >= agent.max_age || agent.health == 0))
            .map(|(id, _)| id.clone())
            .collect();

        for agent_id in dead_agent_ids {
            let agent = self.agents.get(&agent_id).unwrap();
            if !agent.is_alive {
                continue;
            }

            let agent_name = agent.name.clone();
            let agent_pos = agent.position;

            // 资源散落：将背包资源散落在当前位置
            let scattered: Vec<(String, u32)> = agent.inventory.iter()
                .filter(|(_, v)| **v > 0)
                .map(|(k, v)| (k.clone(), *v))
                .collect();

            for (res_type, amount) in &scattered {
                if let Some(node) = self.resources.get_mut(&agent_pos) {
                    // 如果当前位置已有资源节点，增加数量
                    if format!("{:?}", node.resource_type) == *res_type {
                        node.current_amount += amount;
                    }
                } else {
                    // 创建新资源节点
                    let resource_type = match res_type.as_str() {
                        "iron" => ResourceType::Iron,
                        "food" => ResourceType::Food,
                        "wood" => ResourceType::Wood,
                        "water" => ResourceType::Water,
                        "stone" => ResourceType::Stone,
                        _ => ResourceType::Food,
                    };
                    let node = resource::ResourceNode::new(agent_pos, resource_type, *amount);
                    self.resources.insert(agent_pos, node);
                }
            }

            // 创建遗产
            let legacy = Legacy::from_agent(agent, self.tick);
            let legacy_event = crate::legacy::LegacyEvent::from_legacy(&legacy);

            // 添加到遗产列表
            self.legacies.push(legacy);

            // 标记 Agent 为死亡
            let agent = self.agents.get_mut(&agent_id).unwrap();
            agent.is_alive = false;

            // 清理死亡 Agent 的位置记录
            if let Some(ids) = self.agent_positions.get_mut(&agent_pos) {
                ids.retain(|id| *id != agent_id);
                if ids.is_empty() {
                    self.agent_positions.remove(&agent_pos);
                }
            }

            // 记录死亡事件
            let res_desc = if scattered.is_empty() {
                String::new()
            } else {
                format!("，留下: {}", scattered.iter().map(|(r, a)| format!("{}x{}", r, a)).collect::<Vec<_>>().join(", "))
            };
            self.record_event(&agent_id, &agent_name, "death", &format!("{} 已死亡{}{}", agent_name, res_desc, if !scattered.is_empty() { "，资源散落在地".to_string() } else { String::new() }), "#FF0000");

            tracing::info!("Agent {} 死亡，产生遗产 {}", agent_name, legacy_event.legacy_id);

            // 3.2 广播到"legacy"topic（简化实现，实际应通过网络层广播）
            // TODO: 调用网络层 broadcast_to_topic("legacy", legacy_event)
        }
    }

    /// 应用动作到世界（路由模式：校验 → 路由 → 统一处理结果）
    pub fn apply_action(&mut self, agent_id: &AgentId, action: &Action) -> ActionResult {
        // 前置校验
        if !self.agents.contains_key(agent_id) {
            return ActionResult::InvalidAgent;
        }
        if !self.agents[agent_id].is_alive {
            return ActionResult::AgentDead;
        }

        // 记录旧位置（用于维护 agent_positions 反向索引）
        let old_position = self.agents.get(agent_id).map(|a| a.position);

        // 记录当前动作
        self.current_actions.insert(agent_id.clone(), action.reasoning.clone());

        // 路由到具体 handler
        let result = match &action.action_type {
            ActionType::Move { direction } => self.handle_move(agent_id, *direction),
            ActionType::Gather { resource } => self.handle_gather(agent_id, *resource),
            ActionType::Wait => self.handle_wait(agent_id),
            ActionType::Build { structure } => self.handle_build(agent_id, *structure),
            ActionType::Attack { target_id } => self.handle_attack(agent_id, target_id.clone()),
            ActionType::Talk { message } => self.handle_talk(agent_id, message.clone()),
            ActionType::Explore { .. } => self.handle_explore(agent_id),
            ActionType::TradeOffer { offer, want, target_id } => self.handle_trade_offer(agent_id, offer.clone(), want.clone(), target_id.clone()),
            ActionType::TradeAccept { .. } => self.handle_trade_accept(agent_id),
            ActionType::TradeReject { .. } => self.handle_trade_reject(agent_id),
            ActionType::AllyPropose { target_id } => self.handle_ally_propose(agent_id, target_id.clone()),
            ActionType::AllyAccept { ally_id } => self.handle_ally_accept(agent_id, ally_id.clone()),
            ActionType::AllyReject { ally_id } => self.handle_ally_reject(agent_id, ally_id.clone()),
            ActionType::InteractLegacy { legacy_id, interaction } => {
                self.handle_legacy_interaction(agent_id, legacy_id, interaction)
            }
        };

        // 统一处理结果：失败时生成错误叙事
        if let ActionResult::Blocked(ref reason) = result {
            self.record_error_narrative(agent_id, &action.action_type, reason);
        }

        // 应用动机变化
        let agent = self.agents.get_mut(agent_id).unwrap();
        for (i, delta) in action.motivation_delta.iter().enumerate() {
            if i < 6 {
                let new_val = agent.motivation[i] + delta;
                agent.motivation[i] = new_val.clamp(0.0, 1.0);
            }
        }

        // 策略创建触发检查（任务 2.1-2.5）
        let is_success = matches!(result, ActionResult::Success);
        if is_success {
            let candidate_count = action.params.get("candidate_count")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(3);
            let motivation_alignment = action.params.get("motivation_alignment")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.8);

            if should_create_strategy(is_success, candidate_count, motivation_alignment) {
                let agent = self.agents.get_mut(agent_id).unwrap();
                let spark_type = SparkType::Explore;
                let _ = scan_strategy_content(&action.reasoning);
                let _ = create_strategy(
                    &agent.strategies,
                    spark_type,
                    self.tick as u32,
                    action.motivation_delta,
                    &action.reasoning,
                );
            }

            // 动机联动：策略成功
            let strategy_data = self.agents.get(agent_id).and_then(|agent| {
                agent.strategies.find_by_spark_type("explore").map(|s| s.clone())
            });
            if let Some(strategy) = strategy_data {
                let agent = self.agents.get_mut(agent_id).unwrap();
                on_strategy_success(&mut agent.motivation, &strategy);
            }
        } else {
            // 动机联动：策略失败
            let strategy_data = self.agents.get(agent_id).and_then(|agent| {
                agent.strategies.find_by_spark_type("explore").map(|s| s.clone())
            });
            if let Some(strategy) = strategy_data {
                let agent = self.agents.get_mut(agent_id).unwrap();
                on_strategy_failure(&mut agent.motivation, &strategy);
            }
        }

        // 统一维护 agent_positions 反向索引
        if let (Some(old_pos), Some(agent)) = (old_position, self.agents.get(agent_id)) {
            if old_pos != agent.position {
                if let Some(ids) = self.agent_positions.get_mut(&old_pos) {
                    ids.retain(|id| *id != *agent_id);
                    if ids.is_empty() {
                        self.agent_positions.remove(&old_pos);
                    }
                }
                self.agent_positions.entry(agent.position)
                    .or_default().push(agent_id.clone());
            }
        }

        result
    }

    /// 生成世界快照
    pub fn snapshot(&self) -> crate::snapshot::WorldSnapshot {
        use crate::snapshot::{WorldSnapshot, AgentSnapshot, CellChange, NarrativeEvent, LegacyEvent, PressureSnapshot};

        let agents = self.agents
            .values()
            .filter(|a| a.is_alive)
            .map(|agent| {
                let current_action = self.current_actions.get(&agent.id)
                    .map(|s| s.as_str())
                    .unwrap_or("等待")
                    .to_string();

                AgentSnapshot {
                    id: agent.id.as_str().to_string(),
                    name: agent.name.clone(),
                    position: (agent.position.x, agent.position.y),
                    motivation: agent.motivation.to_array(),
                    health: agent.health,
                    max_health: agent.max_health,
                    inventory_summary: agent.inventory.iter()
                        .map(|(k, v)| (k.clone(), *v))
                        .collect(),
                    current_action,
                    age: agent.age,
                    is_alive: agent.is_alive,
                }
            })
            .collect();

        // 从 tick_events 填充 events
        let events = self.tick_events.iter().map(|e| NarrativeEvent {
            tick: e.tick,
            agent_id: e.agent_id.clone(),
            agent_name: e.agent_name.clone(),
            event_type: e.event_type.clone(),
            description: e.description.clone(),
            color_code: e.color_code.clone(),
        }).collect();

        // 从 legacies 填充 legacies
        let legacies = self.legacies.iter().map(|l| LegacyEvent {
            id: l.id.clone(),
            position: (l.position.x, l.position.y),
            legacy_type: "agent_legacy".to_string(),
            original_agent_name: l.original_agent_name.clone(),
        }).collect();

        // 从 pressure_pool 填充 pressures
        let pressures = self.pressure_pool.iter().map(|p| PressureSnapshot {
            id: p.id.clone(),
            pressure_type: format!("{:?}", p.pressure_type),
            description: p.description.clone(),
            remaining_ticks: p.remaining_ticks,
        }).collect();

        // 从 structures 填充 map_changes
        let map_changes = self.structures.iter().map(|(pos, s)| CellChange {
            x: pos.x,
            y: pos.y,
            terrain: format!("{:?}", self.map.get_terrain(*pos)),
            structure: Some(s.structure_type.clone()).map(|s| format!("{:?}", s)),
            resource_type: None,
            resource_amount: None,
        }).collect();

        WorldSnapshot {
            tick: self.tick,
            agents,
            map_changes,
            events,
            legacies,
            pressures,
        }
    }

    /// 环境压力 tick
    fn pressure_tick(&mut self) {
        // 每 20-50 tick 生成一个压力事件
        if self.tick % 30 == 0 {
            // TODO: 生成压力事件
        }

        // 移除过期的压力事件
        self.pressure_pool.retain(|p| p.remaining_ticks > 0);

        // 减少剩余 tick
        for pressure in &mut self.pressure_pool.iter_mut() {
            pressure.remaining_ticks = pressure.remaining_ticks.saturating_sub(1);
        }
    }

    /// 遗产衰减
    fn decay_legacies(&mut self) {
        for legacy in &mut self.legacies {
            if legacy.is_decaying(self.tick) {
                legacy.decay_items();
            }
        }

        // 4.4 清理空遗迹（物品全部消失且超过 100 tick）
        self.legacies.retain(|legacy| {
            !legacy.items.is_empty() || (self.tick - legacy.created_tick) < 100
        });
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
    Success,
    InvalidAgent,
    AgentDead,
    Blocked(String),
    OutOfBounds,
    NotImplemented,
}
