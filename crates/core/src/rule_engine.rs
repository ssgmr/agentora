//! 规则引擎：硬约束过滤、动作校验、LLM 不可用时的生存兜底

use crate::decision::ActionCandidate;
use crate::types::{ActionType, AgentId, Position, TerrainType, ResourceType, StructureType};
use crate::vision::{NearbyAgentInfo, NearbyStructureInfo, NearbyLegacyInfo};
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
    pub self_id: AgentId,
    pub existing_agents: HashSet<AgentId>,
    pub resources_at: HashMap<Position, (ResourceType, u32)>,
    pub nearby_agents: Vec<NearbyAgentInfo>,
    pub nearby_structures: Vec<NearbyStructureInfo>,
    pub nearby_legacies: Vec<NearbyLegacyInfo>,
    /// 活跃压力事件描述（用于 Prompt 注入）
    pub active_pressures: Vec<String>,
    /// 上次移动的方向（用于防止来回振荡）
    pub last_move_direction: Option<crate::types::Direction>,
    /// 临时偏好（来自引导面板等）
    pub temp_preferences: Vec<(String, f32, u32)>, // (key, boost, remaining_ticks)
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
            self_id: AgentId::default(),
            existing_agents: HashSet::new(),
            resources_at: HashMap::new(),
            nearby_agents: Vec::new(),
            nearby_structures: Vec::new(),
            nearby_legacies: Vec::new(),
            active_pressures: Vec::new(),
            last_move_direction: None,
            temp_preferences: Vec::new(),
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
        let start_pos = world_state.agent_position;

        // 移动动作：四个方向，统一使用 MoveToward
        for direction in [crate::types::Direction::North, crate::types::Direction::South, crate::types::Direction::East, crate::types::Direction::West] {
            if self.check_move_valid(direction, world_state) {
                let target = match direction {
                    crate::types::Direction::North => Position::new(start_pos.x, start_pos.y.wrapping_sub(1)),
                    crate::types::Direction::South => Position::new(start_pos.x, start_pos.y + 1),
                    crate::types::Direction::East => Position::new(start_pos.x + 1, start_pos.y),
                    crate::types::Direction::West => Position::new(start_pos.x.wrapping_sub(1), start_pos.y),
                };
                candidates.push(ActionType::MoveToward { target });
            } else {
                let delta = direction.delta();
                let nx = start_pos.x as i32 + delta.0;
                let ny = start_pos.y as i32 + delta.1;
                let target = Position::new(nx as u32, ny as u32);
                let reason = if let Some(terrain) = world_state.terrain_at.get(&target) {
                    if !terrain.is_passable() { format!("{:?} 地形阻挡", terrain) }
                    else { "未知原因".to_string() }
                } else {
                    "越界或未知地形".to_string()
                };
                tracing::trace!("[RuleEngine][硬约束] {:?} 方向 → ({},{}) 排除：{}", direction, target.x, target.y, reason);
            }
        }

        // MoveToward 动作：只为相邻格内的资源/可通行位置生成候选
        if !world_state.resources_at.is_empty() {
            for (pos, _) in &world_state.resources_at {
                if pos.manhattan_distance(&world_state.agent_position) == 1
                    && self.is_valid_move_toward_target(pos, world_state)
                {
                    candidates.push(ActionType::MoveToward { target: *pos });
                }
            }
        }

        // 采集动作：当前位置有资源
        if let Some((resource_type, _amount)) = world_state.resources_at.get(&world_state.agent_position) {
            candidates.push(ActionType::Gather {
                resource: resource_type.clone()
            });
        }

        // 向资源移动：当视野内有资源但不在当前位置时，生成朝向最近资源的 MoveToward 候选
        if !world_state.resources_at.is_empty()
            && world_state.resources_at.get(&world_state.agent_position).is_none()
        {
            if let Some(nearest_resource_pos) = self.find_nearest_resource(&world_state.agent_position, &world_state.resources_at) {
                if let Some(direction) = self.direction_toward(&world_state.agent_position, &nearest_resource_pos) {
                    if self.check_move_valid(direction, world_state) {
                        candidates.push(ActionType::MoveToward { target: nearest_resource_pos });
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
            if *agent_id != world_state.self_id {
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
            tracing::trace!("[RuleEngine] MoveToward {:?} 越界 ({},{})", direction, new_x, new_y);
            return false;
        }

        let new_pos = Position::new(new_x as u32, new_y as u32);

        // 地形检查：所有地形均可通行
        if world_state.terrain_at.contains_key(&new_pos) {
            return true;
        }

        // 默认假设未知地形可通行
        true
    }

    /// 检查是否可以建造
    fn can_build(&self, structure: StructureType, world_state: &WorldState) -> bool {
        let required = structure.resource_cost();
        for (resource, amount) in required {
            if world_state.agent_inventory.get(&resource).unwrap_or(&0) < &amount {
                return false;
            }
        }
        true
    }

    /// 验证 MoveToward 目标位置有效性
    pub fn is_valid_move_toward_target(&self, target: &Position, world_state: &WorldState) -> bool {
        // 验证1: 目标在地图有效范围内
        if target.x >= world_state.map_size || target.y >= world_state.map_size {
            tracing::trace!("[RuleEngine] MoveToward 校验失败：目标 ({},{}) 超出地图范围", target.x, target.y);
            return false;
        }

        // 验证2: 目标必须与当前位置相邻（曼哈顿距离 = 1）
        let dist = target.manhattan_distance(&world_state.agent_position);
        if dist != 1 {
            tracing::trace!("[RuleEngine] MoveToward 校验失败：目标 ({},{}) 不相邻（距离={}）", target.x, target.y, dist);
            return false;
        }

        // 验证3: 目标地形可通行
        if let Some(terrain) = world_state.terrain_at.get(target) {
            if !terrain.is_passable() {
                tracing::trace!("[RuleEngine] MoveToward 校验失败：目标 ({},{}) 地形 {:?} 不可通行", target.x, target.y, terrain);
                return false;
            }
        }

        true
    }

    /// 校验动作参数合法性，返回 (是否合法, 失败原因)
    pub fn validate_action(&self, candidate: &ActionCandidate, world_state: &WorldState) -> (bool, Option<String>) {
        match &candidate.action_type {
            ActionType::MoveToward { target } => {
                if target.x >= world_state.map_size || target.y >= world_state.map_size {
                    return (false, Some(format!("移动目标({},{}) 超出地图范围", target.x, target.y)));
                }
                let dist = target.manhattan_distance(&world_state.agent_position);
                if dist != 1 {
                    return (false, Some(format!("移动目标({},{}) 不相邻（距离={}），只能移动到相邻格", target.x, target.y, dist)));
                }
                // 所有地形均可通行，不再校验地形阻挡
                (true, None)
            }
            ActionType::Gather { resource } => {
                match world_state.resources_at.get(&world_state.agent_position) {
                    Some((rt, _)) if rt.as_str() == resource.as_str() => (true, None),
                    Some((rt, _)) => (false, Some(format!("当前位置没有 {:?} 资源（只有 {:?}）", resource, rt))),
                    None => (false, Some("当前位置没有资源，无法采集".to_string())),
                }
            }
            ActionType::Build { structure } => {
                if self.can_build(*structure, world_state) {
                    (true, None)
                } else {
                    let cost = structure.resource_cost();
                    let missing: Vec<_> = cost.iter()
                        .filter(|(r, amount)| world_state.agent_inventory.get(r).unwrap_or(&0) < amount)
                        .map(|(r, amount)| format!("{:?}x{}", r, amount))
                        .collect();
                    (false, Some(format!("缺少建造材料：{}", missing.join(", "))))
                }
            }
            ActionType::Attack { target_id } => {
                if *target_id == world_state.self_id {
                    (false, Some("不能攻击自己".to_string()))
                } else if !world_state.existing_agents.contains(target_id) {
                    (false, Some(format!("攻击目标 {:?} 不存在", target_id)))
                } else {
                    (true, None)
                }
            }
            ActionType::Talk { .. } => {
                (true, None)
            }
            ActionType::TradeOffer { offer, target_id, .. } => {
                for (resource, amount) in offer {
                    if world_state.agent_inventory.get(resource).unwrap_or(&0) < amount {
                        return (false, Some(format!("背包中没有足够的 {:?} 用于交易", resource)));
                    }
                }
                if !world_state.existing_agents.contains(target_id) {
                    (false, Some(format!("交易目标 {:?} 不存在", target_id)))
                } else {
                    (true, None)
                }
            }
            ActionType::TradeAccept { .. } | ActionType::TradeReject { .. } => {
                (true, None)
            }
            ActionType::AllyPropose { target_id } | ActionType::AllyAccept { ally_id: target_id } | ActionType::AllyReject { ally_id: target_id } => {
                if !world_state.existing_agents.contains(target_id) {
                    (false, Some(format!("结盟目标 {:?} 不存在", target_id)))
                } else {
                    (true, None)
                }
            }
            ActionType::Explore { .. } => {
                (true, None)
            }
            ActionType::Wait => {
                (true, None)
            }
            ActionType::Eat => {
                if world_state.agent_inventory.get(&ResourceType::Food).unwrap_or(&0) > &0 {
                    (true, None)
                } else {
                    (false, Some("背包中没有食物，无法进食".to_string()))
                }
            }
            ActionType::Drink => {
                if world_state.agent_inventory.get(&ResourceType::Water).unwrap_or(&0) > &0 {
                    (true, None)
                } else {
                    (false, Some("背包中没有水，无法饮水".to_string()))
                }
            }
            ActionType::InteractLegacy { .. } => {
                (true, None)
            }
        }
    }

    /// 选择动作目标（NPC 专用）
    pub fn select_target(
        &self,
        purpose: &str,
        world_state: &WorldState,
    ) -> Option<AgentId> {
        if world_state.nearby_agents.is_empty() {
            return world_state.existing_agents.iter()
                .filter(|id| **id != world_state.self_id)
                .next()
                .cloned();
        }

        match purpose {
            "attack" => {
                world_state.nearby_agents.iter()
                    .filter(|a| a.id != world_state.self_id)
                    .min_by(|a, b| a.distance.cmp(&b.distance))
                    .map(|info| info.id.clone())
            }
            "ally" | "trade" => {
                world_state.nearby_agents.iter()
                    .max_by(|a, b| a.trust.partial_cmp(&b.trust).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|info| info.id.clone())
            }
            "talk" => {
                world_state.nearby_agents.iter()
                    .min_by(|a, b| a.distance.cmp(&b.distance))
                    .map(|info| info.id.clone())
            }
            _ => {
                world_state.nearby_agents.first().map(|info| info.id.clone())
            }
        }
    }

    /// LLM 不可用时的生存兜底
    ///
    /// 优先级：
    /// 1. satiety/hydration 极低且有食物/水 → Eat/Drink
    /// 2. 脚下有资源 → Gather
    /// 3. 视野有资源 → MoveToward 最近资源
    /// 4. 否则 → Wait
    pub fn survival_fallback(&self, world_state: &WorldState) -> Option<ActionCandidate> {
        // 1. 背包有食物/水且状态低 → 直接进食/饮水
        if world_state.agent_satiety <= 30 {
            if world_state.agent_inventory.get(&ResourceType::Food).copied().unwrap_or(0) > 0 {
                return Some(ActionCandidate {
                    reasoning: "LLM 不可用，背包有食物，直接进食恢复饱食度".to_string(),
                    action_type: ActionType::Eat,
                    target: None,
                    params: HashMap::new(),
                });
            }
        }
        if world_state.agent_hydration <= 30 {
            if world_state.agent_inventory.get(&ResourceType::Water).copied().unwrap_or(0) > 0 {
                return Some(ActionCandidate {
                    reasoning: "LLM 不可用，背包有水源，直接饮水恢复水分度".to_string(),
                    action_type: ActionType::Drink,
                    target: None,
                    params: HashMap::new(),
                });
            }
        }

        // 2. 脚下有资源 → 采集
        if let Some((resource_type, _amount)) = world_state.resources_at.get(&world_state.agent_position) {
            return Some(ActionCandidate {
                reasoning: format!("LLM 不可用，采集脚下的 {:?}", resource_type),
                action_type: ActionType::Gather { resource: *resource_type },
                target: None,
                params: HashMap::new(),
            });
        }

        // 3. 视野有资源 → 向最近资源移动
        if !world_state.resources_at.is_empty() {
            if let Some(nearest_pos) = self.find_nearest_resource(&world_state.agent_position, &world_state.resources_at) {
                // 只考虑可通行的资源（排除 Water 地形）
                if let Some(terrain) = world_state.terrain_at.get(&nearest_pos) {
                    if terrain.is_passable() {
                        return Some(ActionCandidate {
                            reasoning: format!("LLM 不可用，向资源({},{})移动", nearest_pos.x, nearest_pos.y),
                            action_type: ActionType::MoveToward { target: nearest_pos },
                            target: None,
                            params: HashMap::new(),
                        });
                    }
                }
            }
        }

        // 4. 兜底 → Wait
        Some(ActionCandidate {
            reasoning: "LLM 不可用，无明确行动目标，原地等待".to_string(),
            action_type: ActionType::Wait,
            target: None,
            params: HashMap::new(),
        })
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}
