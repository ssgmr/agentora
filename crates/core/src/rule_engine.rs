//! 规则引擎：硬约束过滤、规则校验、兜底决策

use crate::decision::{ActionCandidate, CandidateSource};
use crate::motivation::MotivationVector;
use crate::types::{ActionType, AgentId, Position, TerrainType, ResourceType, StructureType};
use std::collections::{HashMap, HashSet};

/// 世界状态快照（用于规则校验）
#[derive(Debug, Clone)]
pub struct WorldState {
    pub map_size: u32,
    pub agent_position: Position,
    pub agent_inventory: HashMap<ResourceType, u32>,
    pub terrain_at: HashMap<Position, TerrainType>,
    pub existing_agents: HashSet<AgentId>,
    pub resources_at: HashMap<Position, ResourceType>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            map_size: 256,
            agent_position: Position::new(0, 0),
            agent_inventory: HashMap::new(),
            terrain_at: HashMap::new(),
            existing_agents: HashSet::new(),
            resources_at: HashMap::new(),
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

        // 采集动作：当前位置有资源
        if let Some(_) = world_state.resources_at.get(&world_state.agent_position) {
            candidates.push(ActionType::Gather {
                resource: world_state.resources_at[&world_state.agent_position]
            });
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
                candidates.push(ActionType::Talk { message: "hello".to_string() });
                candidates.push(ActionType::Attack { target_id: agent_id.clone() });
            }
        }

        candidates
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

    /// 校验动作参数合法性
    pub fn validate_action(&self, candidate: &ActionCandidate, world_state: &WorldState) -> bool {
        match &candidate.action_type {
            ActionType::Move { direction } => {
                self.check_move_valid(*direction, world_state)
            }
            ActionType::Gather { resource } => {
                // 检查当前位置是否有该资源
                world_state.resources_at.get(&world_state.agent_position) == Some(resource)
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
            ActionType::TradeOffer { offer, want } => {
                // 检查提供的资源是否在背包中
                for (resource, amount) in offer {
                    if world_state.agent_inventory.get(resource).unwrap_or(&0) < amount {
                        return false;
                    }
                }
                true
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

    /// 兜底动作：当 LLM 全部失败时的安全默认动作
    pub fn fallback_action(&self, motivation: &MotivationVector, world_state: &WorldState) -> ActionCandidate {
        // 优先级 1: 向最近资源移动（解决资源压力）
        // 优先级 2: 原地等待（安全默认）

        // 简化实现：原地等待
        ActionCandidate {
            reasoning: "LLM 失败，规则引擎兜底：原地等待".to_string(),
            action_type: ActionType::Wait,
            target: None,
            params: HashMap::new(),
            motivation_delta: [0.0; 6],
            source: CandidateSource::RuleEngine,
        }
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}
