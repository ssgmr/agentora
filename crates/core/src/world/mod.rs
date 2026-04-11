//! 世界模型：256×256 地图、地形、区域、资源、环境压力

pub mod map;
pub mod region;
pub mod resource;
pub mod pressure;
pub mod structure;
pub mod generator;

use crate::seed::WorldSeed;
use crate::agent::Agent;
use crate::types::{AgentId, Position, ActionType, Action, TerrainType, ResourceType};
use crate::legacy::Legacy;
use crate::strategy::decay::{decay_all_strategies, check_deprecation, auto_delete_deprecated};
use crate::strategy::create::{should_create_strategy, create_strategy, scan_strategy_content};
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
        };

        // 生成地形
        Self::generate_terrain(&mut world.map, &seed);

        // 生成区域
        Self::generate_regions(&mut world.regions, seed);

        // 生成资源节点
        Self::generate_resources(&mut world.map, &mut world.resources, seed);

        // 生成初始 Agent
        Self::generate_agents(&mut world.agents, &world.map, seed);

        world
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
    fn generate_agents(agents: &mut HashMap<AgentId, Agent>, map: &map::CellGrid, seed: &WorldSeed) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let (width, height) = map.size();

        let templates: Vec<&[f32; 6]> = seed.motivation_templates.values().collect();
        let template_names: Vec<&str> = seed.motivation_templates.keys().map(|s| s.as_str()).collect();

        for i in 0..seed.initial_agents {
            // 找一个可通行位置
            let mut pos;
            loop {
                let x = rng.gen_range(0..width);
                let y = rng.gen_range(0..height);
                pos = Position::new(x, y);
                if map.get_terrain(pos).is_passable() {
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

            agents.insert(agent.id.clone(), agent);
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

    /// 推进 tick
    pub fn advance_tick(&mut self) {
        self.tick += 1;

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

            // 创建遗产
            let legacy = Legacy::from_agent(agent, self.tick);
            let legacy_event = crate::legacy::LegacyEvent::from_legacy(&legacy);

            // 添加到遗产列表
            self.legacies.push(legacy);

            // 标记 Agent 为死亡
            let agent = self.agents.get_mut(&agent_id).unwrap();
            agent.is_alive = false;

            tracing::info!("Agent {} 死亡，产生遗产 {}", agent.name, legacy_event.legacy_id);

            // 3.2 广播到"legacy"topic（简化实现，实际应通过网络层广播）
            // TODO: 调用网络层 broadcast_to_topic("legacy", legacy_event)
        }
    }

    /// 应用动作到世界
    pub fn apply_action(&mut self, agent_id: &AgentId, action: &Action) -> ActionResult {
        // 检查 Agent 是否存在且存活
        let agent_check = self.agents.get(agent_id);
        if agent_check.is_none() {
            return ActionResult::InvalidAgent;
        }
        if !agent_check.unwrap().is_alive {
            return ActionResult::AgentDead;
        }

        // 记录当前动作
        self.current_actions.insert(agent_id.clone(), action.reasoning.clone());

        // 执行动作
        let result = match &action.action_type {
            ActionType::Move { direction } => {
                let agent = self.agents.get_mut(agent_id).unwrap();
                let (dx, dy) = direction.delta();
                let new_x = agent.position.x as i32 + dx;
                let new_y = agent.position.y as i32 + dy;

                if new_x >= 0 && new_y >= 0 {
                    let new_pos = Position::new(new_x as u32, new_y as u32);
                    if self.map.is_valid(new_pos) {
                        let terrain = self.map.get_terrain(new_pos);
                        if terrain.is_passable() {
                            agent.position = new_pos;
                            ActionResult::Success
                        } else {
                            ActionResult::Blocked
                        }
                    } else {
                        ActionResult::OutOfBounds
                    }
                } else {
                    ActionResult::OutOfBounds
                }
            }

            ActionType::Gather { resource } => {
                let agent = self.agents.get_mut(agent_id).unwrap();
                // 简化实现：直接添加资源到背包
                let resource_key = resource.as_str().to_string();
                let current = *agent.inventory.get(&resource_key).unwrap_or(&0);
                agent.inventory.insert(resource_key, current + 1);
                ActionResult::Success
            }

            ActionType::Wait => {
                ActionResult::Success
            }

            ActionType::InteractLegacy { legacy_id, interaction } => {
                // 2.4 交互合法性检查（必须在遗迹格）
                self.handle_legacy_interaction(agent_id, legacy_id, interaction)
            }

            _ => {
                // 其他动作类型暂不实现
                ActionResult::NotImplemented
            }
        };

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
            // 尝试从 params 提取候选数量和动机对齐度
            let candidate_count = action.params.get("candidate_count")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(3);
            let motivation_alignment = action.params.get("motivation_alignment")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.8);

            if should_create_strategy(is_success, candidate_count, motivation_alignment) {
                let agent = self.agents.get_mut(agent_id).unwrap();
                let spark_type = SparkType::Explore; // 默认使用 Explore，实际应从决策上下文获取
                let _ = scan_strategy_content(&action.reasoning);
                let _ = create_strategy(
                    &agent.strategies,
                    spark_type,
                    self.tick as u32,
                    action.motivation_delta,
                    &action.reasoning,
                );
            }

            // 动机联动：策略成功（任务 6.4）
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

        WorldSnapshot {
            tick: self.tick,
            agents,
            map_changes: vec![],
            events: vec![],
            legacies: vec![],
            pressures: vec![],
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

    /// 处理遗产交互（任务 2.1-2.4）
    fn handle_legacy_interaction(&mut self, agent_id: &AgentId, legacy_id: &str, interaction: &crate::types::LegacyInteraction) -> ActionResult {
        // 2.4 交互合法性检查：必须在遗迹格
        let agent = self.agents.get(agent_id).unwrap();
        let agent_pos = agent.position;

        // 查找遗产
        let legacy_index = self.legacies.iter().position(|l| l.id == legacy_id);
        if legacy_index.is_none() {
            return ActionResult::InvalidAgent; // 遗产不存在
        }
        let legacy = &self.legacies[legacy_index.unwrap()];

        // 检查 Agent 是否在遗迹位置
        if legacy.position != agent_pos {
            return ActionResult::Blocked; // 不在遗迹位置，无法交互
        }

        // agent 借用在此结束，因为 agent_pos 已复制

        match interaction {
            // 2.1 祭拜动作（认知/传承动机 +0.05）
            crate::types::LegacyInteraction::Worship => {
                let agent = self.agents.get_mut(agent_id).unwrap();
                // 认知动机（索引 2）+0.05
                agent.motivation[2] = (agent.motivation[2] + 0.05).clamp(0.0, 1.0);
                // 传承动机（索引 5）+0.05
                agent.motivation[5] = (agent.motivation[5] + 0.05).clamp(0.0, 1.0);
                ActionResult::Success
            }

            // 2.2 探索动作（认知动机 +0.1，获得回响日志）
            crate::types::LegacyInteraction::Explore => {
                let agent = self.agents.get_mut(agent_id).unwrap();
                // 认知动机 +0.1
                agent.motivation[2] = (agent.motivation[2] + 0.1).clamp(0.0, 1.0);
                // 回响日志已在 legacy.echo_log 中，Agent 可以通过 memory 系统记录
                ActionResult::Success
            }

            // 2.3 拾取动作（转移物品到背包）
            crate::types::LegacyInteraction::Pickup => {
                // 检查遗产是否有物品
                let legacy = &mut self.legacies[legacy_index.unwrap()];
                if legacy.items.is_empty() {
                    return ActionResult::Blocked; // 没有物品可拾取
                }

                // 转移第一个物品到 Agent 背包
                let mut items_to_transfer = Vec::new();
                for (item_name, amount) in &legacy.items {
                    if *amount > 0 {
                        items_to_transfer.push((item_name.clone(), *amount));
                        break; // 只拾取第一个物品
                    }
                }

                if items_to_transfer.is_empty() {
                    return ActionResult::Blocked;
                }

                let (item_name, amount) = items_to_transfer[0].clone();
                let agent = self.agents.get_mut(agent_id).unwrap();
                let current = *agent.inventory.get(&item_name).unwrap_or(&0);
                agent.inventory.insert(item_name.clone(), current + amount);

                // 从遗产中移除物品
                let legacy = &mut self.legacies[legacy_index.unwrap()];
                legacy.items.insert(item_name, amount - 1);

                ActionResult::Success
            }
        }
    }

    /// 持久化世界状态
    pub async fn persist(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: 实现持久化
        Ok(())
    }
}

/// 动作执行结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionResult {
    Success,
    InvalidAgent,
    AgentDead,
    Blocked,
    OutOfBounds,
    NotImplemented,
}
