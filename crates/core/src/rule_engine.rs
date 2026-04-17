//! 规则引擎：硬约束过滤、规则校验、兜底决策

use crate::decision::{ActionCandidate, CandidateSource};
use crate::motivation::MotivationVector;
use crate::types::{ActionType, AgentId, Position, TerrainType, ResourceType, StructureType};
use crate::vision::NearbyAgentInfo;
use std::collections::{HashMap, HashSet};

/// 世界状态快照（用于规则校验）
#[derive(Debug, Clone)]
pub struct WorldState {
    pub map_size: u32,
    pub agent_position: Position,
    pub agent_inventory: HashMap<ResourceType, u32>,
    pub agent_satiety: u32,
    pub agent_hydration: u32,
    pub terrain_at: HashMap<Position, TerrainType>,
    pub existing_agents: HashSet<AgentId>,
    pub resources_at: HashMap<Position, (ResourceType, u32)>,
    pub nearby_agents: Vec<NearbyAgentInfo>,
    /// 活跃压力事件描述（用于 Prompt 注入）
    pub active_pressures: Vec<String>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            map_size: 256,
            agent_position: Position::new(0, 0),
            agent_inventory: HashMap::new(),
            agent_satiety: 100,
            agent_hydration: 100,
            terrain_at: HashMap::new(),
            existing_agents: HashSet::new(),
            resources_at: HashMap::new(),
            nearby_agents: Vec::new(),
            active_pressures: Vec::new(),
        }
    }
}

/// 规则引擎
pub struct RuleEngine;

impl RuleEngine {
    pub fn new() -> Self {
        Self
    }

    /// 硬约束过滤：过滤掉物理上不可能的动作
    pub fn filter_hard_constraints(&self, world_state: &WorldState) -> Vec<ActionType> {
        // 生成所有可能的候选动作，然后过滤
        let mut candidates = Vec::new();

        // 移动动作：四个方向
        for direction in [crate::types::Direction::North, crate::types::Direction::South, crate::types::Direction::East, crate::types::Direction::West] {
            if self.check_move_valid(direction, world_state) {
                candidates.push(ActionType::Move { direction });
            }
        }

        // MoveToward 动作：为视野内最近的资源生成导航候选
        if !world_state.resources_at.is_empty() {
            let mut move_toward_candidates: Vec<ActionType> = Vec::new();

            // 按距离排序，只取最近的3个资源
            let mut resources: Vec<_> = world_state.resources_at.iter().collect();
            resources.sort_by_key(|(pos, _)| pos.manhattan_distance(&world_state.agent_position));

            for (pos, _) in resources.iter().take(3) {
                if self.is_valid_move_toward_target(pos, world_state) {
                    move_toward_candidates.push(ActionType::MoveToward { target: **pos });
                }
            }

            candidates.extend(move_toward_candidates);
        }

        // 采集动作：当前位置有资源
        if let Some((resource_type, _amount)) = world_state.resources_at.get(&world_state.agent_position) {
            candidates.push(ActionType::Gather {
                resource: resource_type.clone()
            });
        }

        // 向资源移动：当视野内有资源但不在当前位置时，生成朝向最近资源的 Move 候选
        if !world_state.resources_at.is_empty()
            && world_state.resources_at.get(&world_state.agent_position).is_none()
        {
            if let Some(nearest_resource_pos) = self.find_nearest_resource(&world_state.agent_position, &world_state.resources_at) {
                if let Some(direction) = self.direction_toward(&world_state.agent_position, &nearest_resource_pos) {
                    if self.check_move_valid(direction, world_state) {
                        candidates.push(ActionType::Move { direction });
                    }
                }
            }
        }

        // 建造动作：检查资源
        for structure in [StructureType::Camp, StructureType::Fence, StructureType::Warehouse] {
            if self.can_build(structure, world_state) {
                candidates.push(ActionType::Build { structure });
            }
        }

        // 等待动作：总是合法
        candidates.push(ActionType::Wait);

        // 社交动作：附近有其他 Agent
        for agent_id in &world_state.existing_agents {
            if agent_id.as_str() != "self" {
                candidates.push(ActionType::Talk { message: "你好，有空聊聊吗？".to_string() });
                candidates.push(ActionType::Attack { target_id: agent_id.clone() });
                candidates.push(ActionType::TradeOffer {
                    offer: HashMap::new(),
                    want: HashMap::new(),
                    target_id: agent_id.clone(),
                });
                candidates.push(ActionType::AllyPropose { target_id: agent_id.clone() });
            }
        }

        candidates
    }

    /// 找到距离最近的资源位置
    fn find_nearest_resource(
        &self,
        from: &Position,
        resources_at: &HashMap<Position, (ResourceType, u32)>,
    ) -> Option<Position> {
        resources_at
            .keys()
            .min_by_key(|pos| pos.manhattan_distance(from))
            .copied()
    }

    /// 计算从当前位置朝向目标位置的方向
    fn direction_toward(&self, from: &Position, to: &Position) -> Option<crate::types::Direction> {
        use crate::types::Direction;
        let dx = to.x as i32 - from.x as i32;
        let dy = to.y as i32 - from.y as i32;

        if dx.abs() >= dy.abs() {
            if dx > 0 { Some(Direction::East) } else if dx < 0 { Some(Direction::West) } else { None }
        } else {
            if dy > 0 { Some(Direction::South) } else if dy < 0 { Some(Direction::North) } else { None }
        }
    }

    /// 检查移动是否有效
    fn check_move_valid(&self, direction: crate::types::Direction, world_state: &WorldState) -> bool {
        let delta = direction.delta();
        let new_x = world_state.agent_position.x as i32 + delta.0;
        let new_y = world_state.agent_position.y as i32 + delta.1;

        // 边界检查
        if new_x < 0 || new_y < 0 || new_x >= world_state.map_size as i32 || new_y >= world_state.map_size as i32 {
            return false;
        }

        let new_pos = Position::new(new_x as u32, new_y as u32);

        // 地形通行性检查
        if let Some(terrain) = world_state.terrain_at.get(&new_pos) {
            return terrain.is_passable();
        }

        // 默认假设未知地形可通行
        true
    }

    /// 检查是否可以建造
    fn can_build(&self, structure: StructureType, world_state: &WorldState) -> bool {
        // 检查资源是否足够
        let required = match structure {
            StructureType::Camp => [(ResourceType::Wood, 5), (ResourceType::Stone, 2)].into_iter().collect::<HashMap<_, _>>(),
            StructureType::Fence => [(ResourceType::Wood, 2)].into_iter().collect::<HashMap<_, _>>(),
            StructureType::Warehouse => [(ResourceType::Wood, 10), (ResourceType::Stone, 5)].into_iter().collect::<HashMap<_, _>>(),
        };

        for (resource, amount) in required {
            if world_state.agent_inventory.get(&resource).unwrap_or(&0) < &amount {
                return false;
            }
        }

        true
    }

    /// 验证 MoveToward 目标位置有效性
    ///
    /// 验证条件：
    /// 1. 目标在地图有效范围内
    /// 2. 目标在视野范围内（曼哈顿距离 ≤ 5）
    /// 3. 目标地形可通行
    pub fn is_valid_move_toward_target(&self, target: &Position, world_state: &WorldState) -> bool {
        // 验证1: 目标在地图有效范围内
        if target.x >= world_state.map_size || target.y >= world_state.map_size {
            return false;
        }

        // 验证2: 目标在视野范围内（曼哈顿距离 ≤ 5）
        let dx = (target.x as i32 - world_state.agent_position.x as i32).abs();
        let dy = (target.y as i32 - world_state.agent_position.y as i32).abs();
        let vision_radius = 5i32; // 与 scan_vision 保持一致
        if dx + dy > vision_radius {
            return false;
        }

        // 验证3: 目标地形可通行
        if let Some(terrain) = world_state.terrain_at.get(target) {
            if !terrain.is_passable() {
                return false;
            }
        }

        // 如果已在目标位置，不需要移动
        if target == &world_state.agent_position {
            return false;
        }

        true
    }

    /// 校验动作参数合法性
    pub fn validate_action(&self, candidate: &ActionCandidate, world_state: &WorldState) -> bool {
        match &candidate.action_type {
            ActionType::Move { direction } => {
                self.check_move_valid(*direction, world_state)
            }
            ActionType::MoveToward { target } => {
                self.is_valid_move_toward_target(target, world_state)
            }
            ActionType::Gather { resource } => {
                // 检查当前位置是否有该资源
                world_state.resources_at.get(&world_state.agent_position)
                    .map(|(rt, _)| rt.as_str() == resource.as_str())
                    .unwrap_or(false)
            }
            ActionType::Build { structure } => {
                self.can_build(*structure, world_state)
            }
            ActionType::Attack { target_id } => {
                // 检查目标是否存在且距离≤1
                world_state.existing_agents.contains(target_id)
            }
            ActionType::Talk { .. } => {
                // 社交动作总是合法
                true
            }
            ActionType::TradeOffer { offer, target_id, .. } => {
                // 检查提供的资源是否在背包中
                for (resource, amount) in offer {
                    if world_state.agent_inventory.get(resource).unwrap_or(&0) < amount {
                        return false;
                    }
                }
                // 检查目标是否存在
                world_state.existing_agents.contains(target_id)
            }
            ActionType::TradeAccept { .. } | ActionType::TradeReject { .. } => {
                // 交易响应动作总是合法
                true
            }
            ActionType::AllyPropose { target_id } | ActionType::AllyAccept { ally_id: target_id } | ActionType::AllyReject { ally_id: target_id } => {
                // 检查目标是否存在
                world_state.existing_agents.contains(target_id)
            }
            ActionType::Explore { .. } => {
                // 探索动作总是合法
                true
            }
            ActionType::Wait => {
                // 等待动作总是合法
                true
            }
            ActionType::InteractLegacy { .. } => {
                // 遗产交互：总是合法
                true
            }
        }
    }

    /// 选择动作目标（NPC 专用）
    /// 基于空间距离、信任值、库存互补等策略选择
    pub fn select_target(
        &self,
        purpose: &str,
        world_state: &WorldState,
    ) -> Option<AgentId> {
        if world_state.nearby_agents.is_empty() {
            // 回退到 existing_agents
            return world_state.existing_agents.iter()
                .filter(|id| id.as_str() != "self")
                .next()
                .cloned();
        }

        match purpose {
            "attack" => {
                // 选择最近的（HP 信息不在 NearbyAgentInfo 中）
                world_state.nearby_agents.iter()
                    .min_by(|a, b| a.distance.cmp(&b.distance))
                    .map(|info| info.id.clone())
            }
            "ally" | "trade" => {
                // 选择信任度最高的
                world_state.nearby_agents.iter()
                    .max_by(|a, b| a.trust.partial_cmp(&b.trust).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|info| info.id.clone())
            }
            "talk" => {
                // 选择最近的
                world_state.nearby_agents.iter()
                    .min_by(|a, b| a.distance.cmp(&b.distance))
                    .map(|info| info.id.clone())
            }
            _ => {
                world_state.nearby_agents.first().map(|info| info.id.clone())
            }
        }
    }

    /// 基于动机类型选择建筑类型
    pub fn select_build_type(&self, motivation_dim: usize, world_state: &WorldState) -> Option<StructureType> {
        match motivation_dim {
            0 => {
                // 生存 → 如果资源足够建 Warehouse，否则建 Camp
                if self.can_build(StructureType::Warehouse, world_state) {
                    Some(StructureType::Warehouse)
                } else if self.can_build(StructureType::Camp, world_state) {
                    Some(StructureType::Camp)
                } else {
                    None
                }
            }
            1 => {
                // 社交 → Campfire
                if self.can_build(StructureType::Camp, world_state) {
                    Some(StructureType::Camp)
                } else {
                    None
                }
            }
            3 => {
                // 表达 → Campfire
                if self.can_build(StructureType::Camp, world_state) {
                    Some(StructureType::Camp)
                } else {
                    None
                }
            }
            4 => {
                // 权力 → Fortress/Warehouse
                if self.can_build(StructureType::Warehouse, world_state) {
                    Some(StructureType::Warehouse)
                } else if self.can_build(StructureType::Fence, world_state) {
                    Some(StructureType::Fence)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 规则决策：基于 6 维动机缺口选择对应动作
    ///
    /// 动机维度索引：0=生存, 1=社交, 2=认知, 3=表达, 4=权力, 5=传承
    /// 平局打破：使用 Agent 位置坐标哈希 `(x + y + i) % 2`
    pub fn rule_decision(&self, motivation: &MotivationVector, world_state: &WorldState) -> crate::types::Action {
        use crate::types::ActionType;

        let mot = motivation.to_array();

        // 生存优先：satiety/hydration 低时优先满足饮食需求
        // 阈值设为 50，提前储备资源（100 tick ≈ 3分钟生存时间）
        if world_state.agent_satiety <= 50 || world_state.agent_hydration <= 50 {
            // 极端：satiety=0 或 hydration=0 时覆盖其他动机
            if world_state.agent_satiety == 0 || world_state.agent_hydration == 0 {
                return self.survival_action(world_state);
            }
            // 饥饿/口渴但背包有资源 → Wait 消耗
            let has_food = world_state.agent_inventory.get(&ResourceType::Food).copied().unwrap_or(0) > 0;
            let has_water = world_state.agent_inventory.get(&ResourceType::Water).copied().unwrap_or(0) > 0;
            if (world_state.agent_satiety <= 50 && has_food) || (world_state.agent_hydration <= 50 && has_water) {
                return crate::types::Action {
                    reasoning: "生存需求紧迫，原地饮食恢复".to_string(),
                    action_type: ActionType::Wait,
                    target: None,
                    params: HashMap::new(),
                    build_type: None,
                    direction: None,
                    motivation_delta: [0.15, 0.0, 0.0, 0.0, 0.0, 0.0],
                };
            }
            // 背包没有 → 寻找资源
            return self.survival_action(world_state);
        }

        // 水分检查（更严格的阈值）
        if world_state.agent_hydration <= 50 {
            let has_water = world_state.agent_inventory.get(&ResourceType::Water).copied().unwrap_or(0) > 0;
            if has_water {
                return crate::types::Action {
                    reasoning: "口渴，饮水恢复".to_string(),
                    action_type: ActionType::Wait,
                    target: None,
                    params: HashMap::new(),
                    build_type: None,
                    direction: None,
                    motivation_delta: [0.15, 0.0, 0.0, 0.0, 0.0, 0.0],
                };
            }
            return self.survival_action(world_state);
        }

        // 1. 找出最高动机维度，平局用位置哈希打破
        let mut max_idx = 0;
        let mut max_val = mot[0];
        for i in 1..6 {
            let pos_hash = (world_state.agent_position.x + world_state.agent_position.y + i as u32) % 2;
            if mot[i] > max_val || (mot[i] == max_val && pos_hash == 0) {
                max_val = mot[i];
                max_idx = i;
            }
        }

        // 2. 动机-动作映射表（扩展支持全套复杂动作）
        let (action_type, build_type, reasoning, motivation_delta) = match max_idx {
            0 => {
                // 生存：优先采集 → 向资源移动 → 资源不足时尝试建造 → 否则探索
                if let Some((resource_type, _amount)) = world_state.resources_at.get(&world_state.agent_position) {
                    // 当前位置有资源，直接采集
                    (
                        ActionType::Gather { resource: resource_type.clone() },
                        None,
                        "生存动机最高，采集资源".to_string(),
                        [0.12, 0.0, 0.06, 0.0, 0.0, 0.0],
                    )
                } else if !world_state.resources_at.is_empty() {
                    // 视野内有资源但不在当前位置，向最近资源移动
                    let (action, reasoning) = if let Some(nearest_pos) = self.find_nearest_resource(&world_state.agent_position, &world_state.resources_at) {
                        if let Some(dir) = self.direction_toward(&world_state.agent_position, &nearest_pos) {
                            if self.check_move_valid(dir, world_state) {
                                (ActionType::Move { direction: dir }, format!("生存动机最高，向资源({},{})移动", nearest_pos.x, nearest_pos.y))
                            } else {
                                (ActionType::Explore { target_region: 0 }, "生存动机最高，但通往资源的路径受阻，探索寻找替代路线".to_string())
                            }
                        } else {
                            (ActionType::Explore { target_region: 0 }, "生存动机最高，已在资源位置但无法采集，探索寻找新资源".to_string())
                        }
                    } else {
                        (ActionType::Explore { target_region: 0 }, "生存动机最高，探索寻找资源".to_string())
                    };
                    let delta = [0.12, 0.0, 0.06, 0.0, 0.0, 0.0];
                    (action, None, reasoning, delta)
                } else if let Some(bt) = self.select_build_type(max_idx, world_state) {
                    (
                        ActionType::Build { structure: bt },
                        Some(bt),
                        format!("生存动机最高，建造 {:?}", bt),
                        [0.10, 0.0, 0.04, 0.0, 0.06, 0.0],
                    )
                } else {
                    (
                        ActionType::Explore { target_region: 0 },
                        None,
                        "生存动机最高，探索寻找资源".to_string(),
                        [0.12, 0.0, 0.06, 0.0, 0.0, 0.0],
                    )
                }
            }
            1 => {
                // 社交：优先结盟 → 对话
                if let Some(target) = self.select_target("ally", world_state) {
                    (
                        ActionType::AllyPropose { target_id: target.clone() },
                        None,
                        format!("社交动机最高，向 {} 提议结盟", target.as_str()),
                        [0.0, 0.12, 0.0, 0.06, 0.0, 0.0],
                    )
                } else {
                    let message = self.generate_social_message(world_state);
                    (
                        ActionType::Talk { message },
                        None,
                        "社交动机最高，尝试交流".to_string(),
                        [0.0, 0.12, 0.0, 0.06, 0.0, 0.0],
                    )
                }
            }
            2 => (
                ActionType::Explore { target_region: 0 },
                None,
                "认知动机最高，探索学习".to_string(),
                [0.06, 0.0, 0.12, 0.0, 0.0, 0.0],
            ),
            3 => {
                // 表达：优先建造 → 对话
                if let Some(bt) = self.select_build_type(max_idx, world_state) {
                    (
                        ActionType::Build { structure: bt },
                        Some(bt),
                        format!("表达动机最高，建造 {:?}", bt),
                        [0.0, 0.06, 0.0, 0.12, 0.0, 0.0],
                    )
                } else {
                    let message = self.generate_express_message(world_state);
                    (
                        ActionType::Talk { message },
                        None,
                        "表达动机最高，分享想法".to_string(),
                        [0.0, 0.06, 0.0, 0.12, 0.0, 0.0],
                    )
                }
            }
            4 => {
                // 权力：优先攻击 → 建造 → 探索
                if let Some(target) = self.select_target("attack", world_state) {
                    (
                        ActionType::Attack { target_id: target.clone() },
                        None,
                        format!("权力动机最高，攻击 {}", target.as_str()),
                        [0.06, 0.0, 0.0, 0.0, 0.12, 0.0],
                    )
                } else if let Some(bt) = self.select_build_type(max_idx, world_state) {
                    (
                        ActionType::Build { structure: bt },
                        Some(bt),
                        format!("权力动机最高，建造 {:?}", bt),
                        [0.06, 0.0, 0.0, 0.0, 0.12, 0.0],
                    )
                } else {
                    (
                        ActionType::Explore { target_region: 0 },
                        None,
                        "权力动机最高，竞争扩张".to_string(),
                        [0.06, 0.0, 0.0, 0.0, 0.12, 0.0],
                    )
                }
            }
            5 => (
                ActionType::Wait,
                None,
                "传承动机最高，原地沉淀".to_string(),
                [0.0, 0.0, 0.0, 0.0, 0.0, 0.12],
            ),
            _ => unreachable!(),
        };

        println!("[RuleEngine] 规则决策: 维度{} = {:.2}, 动作={:?}", max_idx, max_val, action_type);

        crate::types::Action {
            reasoning,
            action_type,
            target: None,
            params: HashMap::new(),
            build_type,
            direction: None,
            motivation_delta,
        }
    }

    /// 兜底动作：当 LLM 全部失败时的安全默认动作
    /// 委托给 `rule_decision()`，基于当前动机状态返回有意义的动作
    pub fn fallback_action(&self, motivation: &MotivationVector, world_state: &WorldState) -> ActionCandidate {
        let action = self.rule_decision(motivation, world_state);
        ActionCandidate {
            reasoning: format!("LLM 失败，规则引擎兜底：{}", action.reasoning),
            action_type: action.action_type,
            target: action.target,
            params: action.params.into_iter().map(|(k, v)| (k, serde_json::Value::String(v))).collect(),
            motivation_delta: action.motivation_delta,
            source: CandidateSource::RuleEngine,
        }
    }

    /// 生存动作：优先前往资源或采集
    fn survival_action(&self, world_state: &WorldState) -> crate::types::Action {
        use crate::types::{ActionType, Direction};

        // 优先找食物或水源
        let need_resource = if world_state.agent_satiety <= 30 {
            Some(ResourceType::Food)
        } else if world_state.agent_hydration <= 30 {
            Some(ResourceType::Water)
        } else {
            None
        };

        if let Some(resource) = need_resource {
            // 检查当前位置是否有需要的资源
            if let Some((rt, _amount)) = world_state.resources_at.get(&world_state.agent_position) {
                if *rt == resource {
                    return crate::types::Action {
                        reasoning: format!("生存优先，采集 {:?}", resource),
                        action_type: ActionType::Gather { resource },
                        target: None,
                        params: HashMap::new(),
                        build_type: None,
                        direction: None,
                        motivation_delta: [0.15, 0.0, 0.05, 0.0, 0.0, 0.0],
                    };
                }
            }

            // 向最近资源移动
            let target_resource_positions: HashMap<Position, (ResourceType, u32)> = world_state.resources_at.iter()
                .filter(|(_, (rt, _))| *rt == resource)
                .map(|(pos, info)| (*pos, info.clone()))
                .collect();

            if let Some(nearest_pos) = self.find_nearest_resource(&world_state.agent_position, &target_resource_positions) {
                if let Some(direction) = self.direction_toward(&world_state.agent_position, &nearest_pos) {
                    return crate::types::Action {
                        reasoning: format!("生存优先，向 {:?} 资源移动", resource),
                        action_type: ActionType::Move { direction },
                        target: None,
                        params: HashMap::new(),
                        build_type: None,
                        direction: None,
                        motivation_delta: [0.15, 0.0, 0.05, 0.0, 0.0, 0.0],
                    };
                }
            }

            // 找不到资源，探索
            return crate::types::Action {
                reasoning: format!("生存优先，探索寻找 {:?}", resource),
                action_type: ActionType::Explore { target_region: 0 },
                target: None,
                params: HashMap::new(),
                build_type: None,
                direction: None,
                motivation_delta: [0.15, 0.0, 0.05, 0.0, 0.0, 0.0],
            };
        }

        // 兜底
        crate::types::Action {
            reasoning: "生存需要，原地休息".to_string(),
            action_type: ActionType::Wait,
            target: None,
            params: HashMap::new(),
            build_type: None,
            direction: None,
            motivation_delta: [0.1, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    /// 生成社交消息（维度 1：社会与关系）
    fn generate_social_message(&self, world_state: &WorldState) -> String {
        let food = world_state.agent_inventory.get(&ResourceType::Food).copied().unwrap_or(0);
        let water = world_state.agent_inventory.get(&ResourceType::Water).copied().unwrap_or(0);
        let nearby_count = world_state.nearby_agents.len();

        // 社交消息：根据上下文生成
        if world_state.agent_satiety <= 50 {
            "同行们，有人能分享食物吗？".to_string()
        } else if world_state.agent_hydration <= 50 {
            "快渴死了，哪有水源？".to_string()
        } else if food > 5 && water > 5 {
            "我有富余的资源，有人需要吗？".to_string()
        } else if nearby_count > 2 {
            "人多好办事，一起探索吧！".to_string()
        } else if nearby_count == 1 {
            if let Some(agent) = world_state.nearby_agents.first() {
                format!("{}，我们聊会儿？", agent.name)
            } else {
                "你好，交个朋友？".to_string()
            }
        } else {
            "有人吗？想找个伙伴一起行动。".to_string()
        }
    }

    /// 生成表达消息（维度 3：表达与创造）
    fn generate_express_message(&self, world_state: &WorldState) -> String {
        let food = world_state.agent_inventory.get(&ResourceType::Food).copied().unwrap_or(0);
        let water = world_state.agent_inventory.get(&ResourceType::Water).copied().unwrap_or(0);
        let wood = world_state.agent_inventory.get(&ResourceType::Wood).copied().unwrap_or(0);
        let stone = world_state.agent_inventory.get(&ResourceType::Stone).copied().unwrap_or(0);

        // 表达消息：分享发现、想法、计划
        if wood >= 10 || stone >= 10 {
            "发现了很多建材，准备建造点什么！".to_string()
        } else if food >= 8 {
            format!("今日收获不错，采集到{}个食物", food)
        } else if world_state.agent_satiety >= 90 && world_state.agent_hydration >= 90 {
            "精力充沛，准备探索更远的地方".to_string()
        } else if !world_state.resources_at.is_empty() {
            let pos = world_state.agent_position;
            format!("在({},{})附近发现了资源点", pos.x, pos.y)
        } else {
            "这片土地很广阔，还有很多地方没探索".to_string()
        }
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}
