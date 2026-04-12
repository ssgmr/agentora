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
    pub terrain_at: HashMap<Position, TerrainType>,
    pub existing_agents: HashSet<AgentId>,
    pub resources_at: HashMap<Position, (ResourceType, u32)>,
    pub nearby_agents: Vec<NearbyAgentInfo>,
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
            nearby_agents: Vec::new(),
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
        if let Some((resource_type, _amount)) = world_state.resources_at.get(&world_state.agent_position) {
            candidates.push(ActionType::Gather {
                resource: resource_type.clone()
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

    /// 规则决策：基于 6 维动机缺口选择对应动作
    ///
    /// 动机维度索引：0=生存, 1=社交, 2=认知, 3=表达, 4=权力, 5=传承
    /// 平局打破：使用 Agent 位置坐标哈希 `(x + y + i) % 2`
    pub fn rule_decision(&self, motivation: &MotivationVector, world_state: &WorldState) -> crate::types::Action {
        use crate::types::ActionType;

        let mot = motivation.to_array();

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

        // 2. 动机-动作映射表
        //    0=生存→Explore, 1=社交→Talk, 2=认知→Explore,
        //    3=表达→Talk, 4=权力→Explore, 5=传承→Wait
        let (action_type, reasoning, motivation_delta) = match max_idx {
            0 => (
                ActionType::Explore { target_region: 0 },
                "生存动机最高，寻找资源".to_string(),
                [0.12, 0.0, 0.06, 0.0, 0.0, 0.0], // 生存+认知
            ),
            1 => (
                ActionType::Talk { message: "问候".to_string() },
                "社交动机最高，尝试交流".to_string(),
                [0.0, 0.12, 0.0, 0.06, 0.0, 0.0], // 社交+表达
            ),
            2 => (
                ActionType::Explore { target_region: 0 },
                "认知动机最高，探索学习".to_string(),
                [0.06, 0.0, 0.12, 0.0, 0.0, 0.0], // 认知+生存
            ),
            3 => (
                ActionType::Talk { message: "分享".to_string() },
                "表达动机最高，分享想法".to_string(),
                [0.0, 0.06, 0.0, 0.12, 0.0, 0.0], // 表达+社交
            ),
            4 => (
                ActionType::Explore { target_region: 0 },
                "权力动机最高，竞争扩张".to_string(),
                [0.06, 0.0, 0.0, 0.0, 0.12, 0.0], // 权力+生存
            ),
            5 => (
                ActionType::Wait,
                "传承动机最高，原地沉淀".to_string(),
                [0.0, 0.0, 0.0, 0.0, 0.0, 0.12], // 传承
            ),
            _ => unreachable!(),
        };

        println!("[RuleEngine] 规则决策: 维度{} = {:.2}, 动作={:?}", max_idx, max_val, action_type);

        crate::types::Action {
            reasoning,
            action_type,
            target: None,
            params: HashMap::new(),
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
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}
