//! 世界模型：256×256 地图、地形、区域、资源、环境压力

pub mod map;
pub mod region;
pub mod resource;
pub mod pressure;
pub mod structure;
pub mod generator;
pub mod actions;

use crate::seed::WorldSeed;
use crate::agent::{Agent, RelationType};
use crate::agent::inventory::get_config as get_inventory_config;
use crate::types::{AgentId, Position, ActionType, Action, TerrainType, ResourceType, StructureType, PersonalitySeed};
use crate::legacy::Legacy;
use crate::strategy::decay::{decay_all_strategies, check_deprecation, auto_delete_deprecated};
use crate::strategy::create::{should_create_strategy, create_strategy, scan_strategy_content};
use crate::snapshot::NarrativeEvent;
use crate::decision::SparkType;
use serde::{Deserialize, Serialize};
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
}

// ===== 辅助类型 =====

/// 里程碑类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MilestoneType {
    FirstCamp,           // 第一座营地
    FirstTrade,          // 贸易萌芽
    FirstFence,          // 领地意识
    FirstAttack,         // 冲突爆发
    FirstLegacyInteract, // 首次传承
    CityState,           // 城邦雏形
    GoldenAge,           // 文明黄金期
}

/// 文明里程碑
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub name: String,
    pub display_name: String,
    pub achieved_tick: u64,
}

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
            next_pressure_tick: {
                use rand::Rng;
                rand::thread_rng().gen_range(40..80)
            },
            pressure_multiplier: HashMap::new(),
            milestones: Vec::new(),
            total_trades: 0,
            total_attacks: 0,
            total_legacy_interacts: 0,
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

        let resource_count = ((width * height) as f32 * seed.resource_density) as usize;
        tracing::debug!("generate_resources: map={width}x{height}, density={}, target={}", seed.resource_density, resource_count);
        let resource_types = [ResourceType::Iron, ResourceType::Food, ResourceType::Wood, ResourceType::Water, ResourceType::Stone];

        let mut placed = 0u32;
        for _ in 0..resource_count {
            let x = rng.gen_range(0..width);
            let y = rng.gen_range(0..height);
            let pos = Position::new(x, y);

            // 只在可通行地形放置资源
            if map.get_terrain(pos).is_passable() {
                let resource_type = resource_types[rng.gen_range(0..resource_types.len())];
                let node = resource::ResourceNode::new(pos, resource_type, rng.gen_range(50..200));
                resources.insert(pos, node);
                placed += 1;
            }
        }
        tracing::debug!("generate_resources 完成: 放置了 {} 个资源节点", placed);
    }

    /// 生成初始 Agent
    fn generate_agents(world: &mut World, map_size: (u32, u32), seed: &WorldSeed) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let (width, height) = map_size;

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

            let name = format!("Agent_{}", i + 1);

            let mut agent = Agent::new(AgentId::new(uuid::Uuid::new_v4().to_string()), name, pos);

            // 任务 2.4：根据性格配置设置 Agent 性格
            let template = seed.agent_personalities.select_template();
            agent.personality = PersonalitySeed::from_template(template);

            tracing::debug!(
                "Agent {} 创建：性格 {} (open={}, agree={}, neuro={})",
                agent.name,
                agent.personality.description,
                agent.personality.openness,
                agent.personality.agreeableness,
                agent.personality.neuroticism
            );

            world.insert_agent_at(agent.id.clone(), agent);
        }
    }

    /// 随机选择地形
    fn random_terrain(rng: &mut impl rand::Rng, ratios: &std::collections::BTreeMap<String, f32>) -> TerrainType {
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

        // 记录旧位置
        let old_position = self.agents.get(agent_id).map(|a| a.position);

        // 记录当前动作的思考内容（reasoning，用于 UI 显示 Agent 的决策思路）
        self.current_actions.insert(agent_id.clone(), action.reasoning.clone());

        // 路由到具体 handler
        let result = match &action.action_type {
            ActionType::MoveToward { target } => self.handle_move_toward(agent_id, *target),
            ActionType::Gather { resource } => self.handle_gather(agent_id, *resource),
            ActionType::Wait => self.handle_wait(agent_id),
            ActionType::Eat => self.handle_eat(agent_id),
            ActionType::Drink => self.handle_drink(agent_id),
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
            ActionType::Explore { .. } => 5,
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
                let spark_type = SparkType::Explore;
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

    /// 统一生成动作反馈（从 ActionResult 提取信息）
    fn generate_action_feedback(&self, result: &ActionResult, action_type: &ActionType, old_position: Option<Position>) -> String {
        match result {
            ActionResult::SuccessWithDetail(detail) => {
                // 解析 detail 格式，生成人类可读的反馈
                self.parse_success_detail(detail, action_type, old_position)
            }
            ActionResult::AlreadyAtPosition(msg) => msg.clone(),
            ActionResult::Blocked(reason) => {
                format!("{} 失败：{}", self.action_type_name(action_type), reason)
            }
            ActionResult::OutOfBounds => format!("{} 失败：超出地图边界", self.action_type_name(action_type)),
            ActionResult::AgentDead => format!("{} 失败：Agent 已死亡", self.action_type_name(action_type)),
            ActionResult::InvalidAgent => format!("{} 失败：Agent 不存在", self.action_type_name(action_type)),
            ActionResult::NotImplemented => format!("{} 失败：未实现", self.action_type_name(action_type)),
        }
    }

    /// 将动作类型转换为简短描述（用于 UI 显示）
    fn action_type_to_short_desc(action_type: &ActionType) -> String {
        match action_type {
            ActionType::MoveToward { target } => {
                format!("移动→({},{})", target.x, target.y)
            }
            ActionType::Gather { resource } => format!("采集 {}", resource.as_str()),
            ActionType::Eat => "进食".to_string(),
            ActionType::Drink => "饮水".to_string(),
            ActionType::Build { structure } => {
                let struct_name = match structure {
                    crate::types::StructureType::Camp => "营地",
                    crate::types::StructureType::Fence => "围栏",
                    crate::types::StructureType::Warehouse => "仓库",
                };
                format!("建造 {}", struct_name)
            }
            ActionType::Attack { target_id } => format!("攻击 {}", target_id.as_str()),
            ActionType::Talk { .. } => "对话".to_string(),
            ActionType::Explore { .. } => "探索".to_string(),
            ActionType::TradeOffer { .. } => "交易".to_string(),
            ActionType::TradeAccept { .. } => "接受交易".to_string(),
            ActionType::TradeReject { .. } => "拒绝交易".to_string(),
            ActionType::AllyPropose { .. } => "结盟".to_string(),
            ActionType::AllyAccept { .. } => "接受结盟".to_string(),
            ActionType::AllyReject { .. } => "拒绝结盟".to_string(),
            ActionType::InteractLegacy { .. } => "互动遗产".to_string(),
            ActionType::Wait => "等待".to_string(),
        }
    }

    /// 解析成功详情，生成人类可读反馈
    fn parse_success_detail(&self, detail: &str, action_type: &ActionType, _old_position: Option<Position>) -> String {
        // detail 格式: "动作类型:具体数据"
        // 如 "move:121,113→(131,142)" 或 "gather:waterx2,remain:184"

        if detail.starts_with("move:") {
            // 格式: move:old_x,old_y→(new_x,new_y)
            let parts = detail.strip_prefix("move:").unwrap_or("");
            if let Some((old, new)) = parts.split_once("→") {
                let old_coords: Vec<&str> = old.split(',').collect();
                let new_coords: Vec<&str> = new.trim_matches(|c| c == '(' || c == ')').split(',').collect();
                if old_coords.len() == 2 && new_coords.len() == 2 {
                    if let (Ok(ox), Ok(oy), Ok(nx), Ok(ny)) = (
                        old_coords[0].parse::<i32>(),
                        old_coords[1].parse::<i32>(),
                        new_coords[0].parse::<i32>(),
                        new_coords[1].parse::<i32>(),
                    ) {
                        // 直接计算方向名称（不使用 direction_description）
                        let dx = nx - ox;
                        let dy = ny - oy;
                        let dir_name = match (dx.cmp(&0), dy.cmp(&0)) {
                            (std::cmp::Ordering::Greater, std::cmp::Ordering::Less) => "东北",
                            (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => "东南",
                            (std::cmp::Ordering::Greater, std::cmp::Ordering::Equal) => "东",
                            (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => "西北",
                            (std::cmp::Ordering::Less, std::cmp::Ordering::Greater) => "西南",
                            (std::cmp::Ordering::Less, std::cmp::Ordering::Equal) => "西",
                            (std::cmp::Ordering::Equal, std::cmp::Ordering::Greater) => "南",
                            (std::cmp::Ordering::Equal, std::cmp::Ordering::Less) => "北",
                            (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal) => "原地",
                        };
                        return format!("向{}移动到 ({}, {})", dir_name, nx, ny);
                    }
                }
            }
            return format!("移动成功");
        }

        if detail.starts_with("gather:") {
            // 新格式: gather:resource x amount,node_remain: count,inv: old→new
            let parts = detail.strip_prefix("gather:").unwrap_or("");

            // 尝试解析新格式
            if parts.contains(",node_remain:") && parts.contains(",inv:") {
                // 格式: resource x amount,node_remain: count,inv: old→new
                if let Some((gather_part, rest)) = parts.split_once(",node_remain:") {
                    let resource_amount: Vec<&str> = gather_part.split('x').collect();
                    if resource_amount.len() == 2 {
                        let resource = resource_amount[0].trim();
                        if let Ok(amount) = resource_amount[1].trim().parse::<u32>() {
                            if let Some((remain_part, inv_part)) = rest.split_once(",inv:") {
                                if let Ok(node_remain) = remain_part.trim().parse::<u32>() {
                                    let inv_parts: Vec<&str> = inv_part.trim().split('→').collect();
                                    if inv_parts.len() == 2 {
                                        if let (Ok(old_inv), Ok(new_inv)) =
                                            (inv_parts[0].parse::<u32>(), inv_parts[1].parse::<u32>()) {
                                            return format!(
                                                "Gather成功：获得 {} x{}。当前位置 {} 资源剩余 x{}。背包 {} 从 x{} 增至 x{}",
                                                resource, amount, resource, node_remain, resource, old_inv, new_inv);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 旧格式兼容: gather:resource x amount,remain: count
            if let Some((gather_part, remain_part)) = parts.split_once(",remain:") {
                let resource_amount: Vec<&str> = gather_part.split('x').collect();
                if resource_amount.len() == 2 {
                    let resource = resource_amount[0];
                    if let Ok(amount) = resource_amount[1].parse::<u32>() {
                        if let Ok(remain) = remain_part.parse::<u32>() {
                            return format!("采集了 {} 个 {}，剩余 {}", amount, resource, remain);
                        }
                    }
                }
            }
            return format!("采集成功");
        }

        if detail.starts_with("eat:") {
            // 新格式: eat:satiety+gain(before→after),food_remain=count
            let parts = detail.strip_prefix("eat:").unwrap_or("");

            if parts.contains("satiety+") && parts.contains(",food_remain=") {
                if let Some((satiety_part, remain_part)) = parts.split_once(",food_remain=") {
                    // 解析 satiety+gain(before→after)
                    if let Some(gain_part) = satiety_part.strip_prefix("satiety+") {
                        let satiety_parts: Vec<&str> = gain_part.split('→').collect();
                        if satiety_parts.len() == 2 {
                            if let (Ok(before), Ok(after)) = (satiety_parts[0].parse::<u32>(), satiety_parts[1].parse::<u32>()) {
                                if let Ok(food_remain) = remain_part.trim().parse::<u32>() {
                                    return format!(
                                        "Eat成功：消耗 food x1，饱食度+{}（从{}增至{}）。背包 food 剩余 x{}",
                                        after - before, before, after, food_remain);
                                }
                            }
                        }
                    }
                }
            }

            // 旧格式兼容: eat:satiety=XX/100
            if let Some(satiety_str) = parts.strip_prefix("satiety=") {
                return format!("进食成功，饱食度恢复至 {}", satiety_str);
            }
            return format!("进食成功");
        }

        if detail.starts_with("drink:") {
            // 新格式: drink:hydration+gain(before→after),water_remain=count
            let parts = detail.strip_prefix("drink:").unwrap_or("");

            if parts.contains("hydration+") && parts.contains(",water_remain=") {
                if let Some((hydration_part, remain_part)) = parts.split_once(",water_remain=") {
                    // 解析 hydration+gain(before→after)
                    if let Some(gain_part) = hydration_part.strip_prefix("hydration+") {
                        let hydration_parts: Vec<&str> = gain_part.split('→').collect();
                        if hydration_parts.len() == 2 {
                            if let (Ok(before), Ok(after)) = (hydration_parts[0].parse::<u32>(), hydration_parts[1].parse::<u32>()) {
                                if let Ok(water_remain) = remain_part.trim().parse::<u32>() {
                                    return format!(
                                        "Drink成功：消耗 water x1，水分度+{}（从{}增至{}）。背包 water 剩余 x{}",
                                        after - before, before, after, water_remain);
                                }
                            }
                        }
                    }
                }
            }

            // 旧格式兼容: drink:hydration=XX/100
            if let Some(hydration_str) = parts.strip_prefix("hydration=") {
                return format!("饮水成功，水分度恢复至 {}", hydration_str);
            }
            return format!("饮水成功");
        }

        if detail.starts_with("build:") {
            // 格式: build:StructureTypeat(x,y)
            let parts = detail.strip_prefix("build:").unwrap_or("");
            if let Some((struct_part, pos_part)) = parts.split_once("at") {
                let coords = pos_part.trim_matches(|c| c == '(' || c == ')');
                return format!("在 {} 建造了 {}", coords, struct_part);
            }
            return format!("建造成功");
        }

        if detail.starts_with("attack:") {
            // 格式: attack:target_namehit,damage=10 或 attack:target_namedefeated,damage=10
            let parts = detail.strip_prefix("attack:").unwrap_or("");
            if let Some((name_part, outcome)) = parts.split_once(",") {
                if outcome.starts_with("defeated") {
                    return format!("攻击 {} 并将其击败", name_part);
                } else if outcome.starts_with("hit") {
                    return format!("攻击 {}，造成 10 点伤害", name_part);
                }
            }
            return format!("攻击成功");
        }

        if detail.starts_with("explore:") {
            // 格式: explore:Nsteps,old_x,old_y→(new_x,new_y)
            let parts = detail.strip_prefix("explore:").unwrap_or("");
            if let Some((steps_part, _)) = parts.split_once("steps") {
                if let Ok(steps) = steps_part.parse::<u32>() {
                    return format!("探索了 {} 步", steps);
                }
            }
            return format!("探索成功");
        }

        if detail.starts_with("talk:") {
            let parts = detail.strip_prefix("talk:").unwrap_or("");
            if parts == "self" {
                return format!("自言自语");
            }
            return format!("与 {} 交流", parts);
        }

        if detail.starts_with("trade_offer:") {
            let target = detail.strip_prefix("trade_offer:").unwrap_or("");
            return format!("向 {} 发起交易请求", target);
        }

        if detail.starts_with("trade_accept:") {
            let parts = detail.strip_prefix("trade_accept:").unwrap_or("");
            return format!("与 {} 完成交易", parts.replace(" ↔ ", " 和 "));
        }

        if detail.starts_with("trade_reject:") {
            let proposer = detail.strip_prefix("trade_reject:").unwrap_or("");
            return format!("拒绝了 {} 的交易请求", proposer);
        }

        if detail.starts_with("ally_propose:") {
            let target = detail.strip_prefix("ally_propose:").unwrap_or("");
            return format!("向 {} 提议结盟", target);
        }

        if detail.starts_with("ally_accept:") {
            let parts = detail.strip_prefix("ally_accept:").unwrap_or("");
            return format!("与 {} 结成联盟", parts.replace(" ↔ ", " 和 "));
        }

        if detail.starts_with("ally_reject:") {
            let proposer = detail.strip_prefix("ally_reject:").unwrap_or("");
            return format!("拒绝了 {} 的结盟请求", proposer);
        }

        if detail.starts_with("legacy:") {
            let parts = detail.strip_prefix("legacy:").unwrap_or("");
            if parts == "worship" {
                return format!("祭拜遗产");
            }
            if parts == "explore" {
                return format!("探索遗产");
            }
            if parts.starts_with("pickup") {
                return format!("拾取了 {}", parts.strip_prefix("pickup ").unwrap_or("物品"));
            }
            return format!("遗产交互成功");
        }

        if detail == "wait" {
            return format!("等待了一回合");
        }

        // 兜底
        format!("{} 执行成功", self.action_type_name(action_type))
    }

    /// 生成世界快照
    pub fn snapshot(&self) -> crate::snapshot::WorldSnapshot {
        use crate::snapshot::{WorldSnapshot, AgentSnapshot, CellChange, NarrativeEvent, LegacyEvent, PressureSnapshot, MilestoneSnapshot};

        let agents = self.agents
            .values()
            .filter(|a| a.is_alive)
            .map(|agent| {
                // reasoning：从 current_actions 获取（存储的是 action.reasoning）
                let reasoning = self.current_actions.get(&agent.id)
                    .map(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();

                // current_action：从 agent.last_action_type 获取（已存储简短描述）
                let current_action = agent.last_action_type.clone()
                    .unwrap_or_else(|| if reasoning.is_empty() { "等待".to_string() } else { "思考中...".to_string() });

                let action_result = agent.last_action_result.as_deref().unwrap_or("").to_string();

                AgentSnapshot {
                    id: agent.id.as_str().to_string(),
                    name: agent.name.clone(),
                    position: (agent.position.x, agent.position.y),
                    health: agent.health,
                    max_health: agent.max_health,
                    satiety: agent.satiety,
                    hydration: agent.hydration,
                    inventory_summary: agent.inventory.iter()
                        .map(|(k, v)| (k.clone(), *v))
                        .collect(),
                    current_action,
                    action_result,
                    reasoning,
                    age: agent.age,
                    is_alive: agent.is_alive,
                    level: agent.level,
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

        // 从 structures 和 resources 填充 map_changes
        // 首先收集所有需要发送的位置
        let mut positions_to_send: std::collections::HashSet<Position> = std::collections::HashSet::new();

        // 收集建筑位置
        for pos in self.structures.keys() {
            positions_to_send.insert(*pos);
        }

        // 收集资源位置
        for (pos, node) in &self.resources {
            if !node.is_depleted && node.current_amount > 0 {
                positions_to_send.insert(*pos);
            }
        }

        let map_changes = positions_to_send.iter().map(|pos| {
            let terrain = format!("{:?}", self.map.get_terrain(*pos));
            let structure = self.structures.get(pos).map(|s| format!("{:?}", s.structure_type));
            let (resource_type, resource_amount) = self.resources.get(pos)
                .filter(|n| !n.is_depleted && n.current_amount > 0)
                .map(|n| (Some(n.resource_type.as_str().to_string()), Some(n.current_amount)))
                .unwrap_or((None, None));

            CellChange {
                x: pos.x,
                y: pos.y,
                terrain,
                structure,
                resource_type,
                resource_amount,
            }
        }).collect();

        // 构建地形网格数据（完整地形快照，用于Godot客户端渲染）
        let (width, height) = self.map.size();
        let terrain_grid: Vec<u8> = (0..height).flat_map(|y| {
            (0..width).map(|x| {
                self.map.get_terrain(Position::new(x, y)).to_index()
            }).collect::<Vec<_>>()
        }).collect();

        WorldSnapshot {
            tick: self.tick,
            agents,
            terrain_grid: Some(terrain_grid),
            terrain_width: Some(width),
            terrain_height: Some(height),
            map_changes,
            events,
            legacies,
            pressures,
            milestones: self.milestones.iter().map(|m| MilestoneSnapshot {
                name: m.name.clone(),
                display_name: m.display_name.clone(),
                achieved_tick: m.achieved_tick,
            }).collect(),
        }
    }

    /// 生存消耗 tick：饱食度和水分度衰减，耗尽时掉血
    /// 每 tick 衰减 1 点（tick 间隔由配置决定，默认 5 秒）
    fn survival_consumption_tick(&mut self) {
        for (_, agent) in self.agents.iter_mut() {
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
        }
    }

    /// 建筑效果 tick
    fn structure_effects_tick(&mut self) {
        use crate::types::StructureType;

        // Camp 回血效果：曼哈顿距离 ≤ 1 的存活 Agent HP +2
        let camp_positions: Vec<Position> = self.structures.iter()
            .filter(|(_, s)| s.structure_type == StructureType::Camp)
            .map(|(pos, _)| *pos)
            .collect();

        for camp_pos in &camp_positions {
            let mut healed_agents: Vec<(AgentId, u32)> = Vec::new();
            for (_, agent) in self.agents.iter() {
                if !agent.is_alive { continue; }
                if agent.position.manhattan_distance(camp_pos) <= 1 && agent.health < agent.max_health {
                    let restored = 2.min(agent.max_health - agent.health);
                    healed_agents.push((agent.id.clone(), restored));
                }
            }
            for (agent_id, hp_restored) in healed_agents {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.health = (agent.health + hp_restored).min(agent.max_health);
                }
            }
        }
    }

    /// 里程碑检查（将在 Task 5.3 完整实现）
    fn check_milestones(&mut self) {
        // 简单实现：检测关键里程碑
        let milestones_to_check = [
            (MilestoneType::FirstCamp, self.structures.values().any(|s| s.structure_type == StructureType::Camp)),
            (MilestoneType::FirstTrade, self.total_trades > 0),
            (MilestoneType::FirstFence, self.structures.values().any(|s| s.structure_type == StructureType::Fence)),
            (MilestoneType::FirstAttack, self.total_attacks > 0),
            (MilestoneType::FirstLegacyInteract, self.total_legacy_interacts > 0),
        ];

        for (milestone_type, condition) in &milestones_to_check {
            if *condition {
                let name = format!("{:?}", milestone_type).to_lowercase();
                let display_name = match milestone_type {
                    MilestoneType::FirstCamp => "第一座营地",
                    MilestoneType::FirstTrade => "贸易萌芽",
                    MilestoneType::FirstFence => "领地意识",
                    MilestoneType::FirstAttack => "冲突爆发",
                    MilestoneType::FirstLegacyInteract => "首次传承",
                    MilestoneType::CityState => "城邦雏形",
                    MilestoneType::GoldenAge => "文明黄金期",
                };
                // 检查是否已达成
                let already_achieved = self.milestones.iter().any(|m| m.name == name);
                if !already_achieved {
                    self.milestones.push(Milestone {
                        name: name.clone(),
                        display_name: display_name.to_string(),
                        achieved_tick: self.tick,
                    });
                    tracing::info!("里程碑达成: {} (tick {})", display_name, self.tick);

                    // 世界正反馈：根据里程碑类型产生世界变化
                    self.apply_milestone_feedback(milestone_type);

                    // 添加叙事事件
                    self.tick_events.push(NarrativeEvent {
                        tick: self.tick,
                        agent_id: "system".to_string(),
                        agent_name: "文明".to_string(),
                        event_type: "milestone".to_string(),
                        description: format!("🏆 达成里程碑：【{}】", display_name),
                        color_code: "#FFD700".to_string(),
                    });
                }
            }
        }

        // 城邦雏形：3+ 建筑 + 2+ 盟友对 + 有 Warehouse
        let structure_count = self.structures.len();
        let has_warehouse = self.structures.values().any(|s| s.structure_type == StructureType::Warehouse);
        let ally_count = self.agents.values()
            .flat_map(|a| a.relations.iter())
            .filter(|(_, r)| r.relation_type == RelationType::Ally)
            .count();
        if structure_count >= 3 && ally_count >= 2 && has_warehouse {
            let name = "citystate";
            if !self.milestones.iter().any(|m| m.name == name) {
                self.milestones.push(Milestone {
                    name: name.to_string(),
                    display_name: "城邦雏形".to_string(),
                    achieved_tick: self.tick,
                });
                tracing::info!("里程碑达成: 城邦雏形 (tick {})", self.tick);
                // 添加叙事事件
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "文明".to_string(),
                    event_type: "milestone".to_string(),
                    description: "🏛 达成里程碑：【城邦雏形】".to_string(),
                    color_code: "#FFD700".to_string(),
                });
            }
        }

        // 文明黄金期：前六个全部达成
        if self.milestones.len() >= 6 {
            let name = "goldenage";
            if !self.milestones.iter().any(|m| m.name == name) {
                self.milestones.push(Milestone {
                    name: name.to_string(),
                    display_name: "文明黄金期".to_string(),
                    achieved_tick: self.tick,
                });
                tracing::info!("里程碑达成: 文明黄金期 (tick {})", self.tick);
                // 添加叙事事件
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "文明".to_string(),
                    event_type: "milestone".to_string(),
                    description: "👑 达成里程碑：【文明黄金期】".to_string(),
                    color_code: "#FFD700".to_string(),
                });
            }
        }
    }

    /// 里程碑达成时的世界正反馈
    fn apply_milestone_feedback(&mut self, milestone_type: &MilestoneType) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let (map_w, map_h) = self.map.size();

        // 找到最近有动作的 Agent 位置作为反馈中心
        let center_pos = self.agents.values()
            .filter(|a| a.is_alive)
            .next()
            .map(|a| a.position)
            .unwrap_or(Position::new(128, 128));

        match milestone_type {
            MilestoneType::FirstCamp => {
                // 首次建造营地 → 周围生成额外食物和水源（营地带来繁荣）
                for _ in 0..5 {
                    let offset_x = rng.gen_range(-3..=3) as i32;
                    let offset_y = rng.gen_range(-3..=3) as i32;
                    let px = (center_pos.x as i32 + offset_x).clamp(0, map_w as i32 - 1) as u32;
                    let py = (center_pos.y as i32 + offset_y).clamp(0, map_h as i32 - 1) as u32;
                    let pos = Position::new(px, py);
                    let res_type = if rng.gen_bool(0.5) { ResourceType::Food } else { ResourceType::Water };
                    let node = resource::ResourceNode::new(pos, res_type, rng.gen_range(3..=8));
                    self.resources.insert(pos, node);
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: "🌱 营地周围涌现出新的食物和水源！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            MilestoneType::FirstTrade => {
                // 首次交易 → 所有 Agent 获得少量额外资源（贸易繁荣）
                for agent in self.agents.values_mut() {
                    if agent.is_alive {
                        *agent.inventory.entry("food".to_string()).or_default() += 1;
                        *agent.inventory.entry("water".to_string()).or_default() += 1;
                    }
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: " 贸易带来繁荣，所有人获得额外补给！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            MilestoneType::FirstFence => {
                // 首次防御 → 周围生成木材（建设需要材料）
                for _ in 0..5 {
                    let offset_x = rng.gen_range(-3..=3) as i32;
                    let offset_y = rng.gen_range(-3..=3) as i32;
                    let px = (center_pos.x as i32 + offset_x).clamp(0, map_w as i32 - 1) as u32;
                    let py = (center_pos.y as i32 + offset_y).clamp(0, map_h as i32 - 1) as u32;
                    let pos = Position::new(px, py);
                    let node = resource::ResourceNode::new(pos, ResourceType::Wood, rng.gen_range(3..=8));
                    self.resources.insert(pos, node);
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: "🪵 围栏周围发现了新的木材资源！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            MilestoneType::CityState => {
                // 城邦时代 → 大规模资源涌现 + 所有 Agent 恢复 HP
                for agent in self.agents.values_mut() {
                    if agent.is_alive {
                        agent.health = agent.max_health;
                        *agent.inventory.entry("food".to_string()).or_default() += 3;
                        *agent.inventory.entry("water".to_string()).or_default() += 3;
                    }
                }
                for _ in 0..10 {
                    let offset_x = rng.gen_range(-8..=8) as i32;
                    let offset_y = rng.gen_range(-8..=8) as i32;
                    let px = (center_pos.x as i32 + offset_x).clamp(0, map_w as i32 - 1) as u32;
                    let py = (center_pos.y as i32 + offset_y).clamp(0, map_h as i32 - 1) as u32;
                    let pos = Position::new(px, py);
                    let res_types = [ResourceType::Food, ResourceType::Water, ResourceType::Wood, ResourceType::Stone];
                    let res_type = res_types[rng.gen_range(0..res_types.len())];
                    let node = resource::ResourceNode::new(pos, res_type, rng.gen_range(5..=15));
                    self.resources.insert(pos, node);
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: "🏛 城邦崛起！资源涌现，所有人恢复健康！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            MilestoneType::GoldenAge => {
                // 黄金时代 → 所有 Agent 满 HP + 大量资源
                for agent in self.agents.values_mut() {
                    if agent.is_alive {
                        agent.health = agent.max_health;
                        agent.satiety = 100;
                        agent.hydration = 100;
                        *agent.inventory.entry("food".to_string()).or_default() += 5;
                        *agent.inventory.entry("water".to_string()).or_default() += 5;
                        *agent.inventory.entry("wood".to_string()).or_default() += 5;
                        *agent.inventory.entry("stone".to_string()).or_default() += 5;
                    }
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: "👑 黄金时代降临！所有人满状态，资源充沛！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            _ => {}
        }
    }

    /// 环境压力 tick
    fn pressure_tick(&mut self) {
        // 生成新压力事件
        if self.tick >= self.next_pressure_tick && self.pressure_pool.len() < 3 {
            use rand::Rng;
            let mut rng = rand::thread_rng();

            // 从三种压力事件中随机选择
            let event_variants = ["drought", "abundance", "plague"];
            let event_type = event_variants[rng.gen_range(0..event_variants.len())];

            let (description, duration) = match event_type {
                "drought" => ("干旱来袭，水源产出减半".to_string(), 30),
                "abundance" => ("丰饶时节，食物产出翻倍".to_string(), 20),
                "plague" => ("瘟疫蔓延，生命受到威胁".to_string(), 1),
                _ => unreachable!(),
            };

            let event = pressure::PressureEvent {
                id: uuid::Uuid::new_v4().to_string(),
                pressure_type: match event_type {
                    "drought" => pressure::PressureType::ResourceFluctuation,
                    "abundance" => pressure::PressureType::ResourceFluctuation,
                    "plague" => pressure::PressureType::ClimateEvent,
                    _ => unreachable!(),
                },
                affected_resource: Some(match event_type {
                    "drought" => "Water".to_string(),
                    "abundance" => "Food".to_string(),
                    _ => String::new(),
                }),
                description: description.clone(),
                duration_ticks: duration,
                remaining_ticks: duration,
                intensity: match event_type {
                    "drought" => 0.5,
                    "abundance" => 2.0,
                    "plague" => 1.0,
                    _ => 1.0,
                },
                affected_region: None,
                created_tick: self.tick,
            };

            // 应用立即效果
            match event_type {
                "drought" => {
                    self.pressure_multiplier.insert("water".to_string(), 0.5);
                }
                "abundance" => {
                    // 食物节点数量翻倍
                    for node in self.resources.values_mut() {
                        if node.resource_type == ResourceType::Food {
                            node.current_amount = (node.current_amount * 2).min(node.max_amount);
                        }
                    }
                }
                "plague" => {
                    // 随机 1-3 个 Agent HP -20
                    let mut alive_agents: Vec<AgentId> = self.agents.iter()
                        .filter(|(_, a)| a.is_alive)
                        .map(|(id, _)| id.clone())
                        .collect();
                    let plague_count = rng.gen_range(1..=3).min(alive_agents.len());
                    // 简单随机选择
                    for _ in 0..plague_count {
                        if alive_agents.is_empty() { break; }
                        let idx = rng.gen_range(0..alive_agents.len());
                        let target_id = alive_agents.remove(idx);
                        if let Some(agent) = self.agents.get_mut(&target_id) {
                            agent.health = agent.health.saturating_sub(20);
                        }
                    }
                }
                _ => {}
            }

            tracing::info!("压力事件生成: {} (持续{}tick)", description, duration);
            // 添加叙事事件
            self.tick_events.push(NarrativeEvent {
                tick: self.tick,
                agent_id: "system".to_string(),
                agent_name: "世界".to_string(),
                event_type: "pressure_start".to_string(),
                description: format!("⚠️ {}", description),
                color_code: "#FF9800".to_string(),
            });
            self.pressure_pool.push(event);
            self.next_pressure_tick = self.tick + rng.gen_range(40..80);
        } else if self.tick >= self.next_pressure_tick {
            // 已达上限，推迟
            self.next_pressure_tick = self.tick + 20;
        }

        // 推进现有事件
        for pressure in &mut self.pressure_pool.iter_mut() {
            pressure.advance();
        }

        // 移除过期事件并恢复效果
        let expired: Vec<pressure::PressureEvent> = self.pressure_pool.drain(..)
            .filter(|p| p.is_finished())
            .collect();
        for event in &expired {
            // 恢复持续效果
            if let Some(ref resource) = event.affected_resource {
                match resource.as_str() {
                    "Water" | "water" => {
                        self.pressure_multiplier.remove("water");
                    }
                    _ => {}
                }
            }
            tracing::info!("压力事件结束: {}", event.description);
            // 添加叙事事件
            self.tick_events.push(NarrativeEvent {
                tick: self.tick,
                agent_id: "system".to_string(),
                agent_name: "世界".to_string(),
                event_type: "pressure_end".to_string(),
                description: format!("✓ {} 已结束", event.description),
                color_code: "#8BC34A".to_string(),
            });
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
