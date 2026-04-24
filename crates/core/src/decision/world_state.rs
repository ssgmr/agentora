//! 世界状态快照
//!
//! WorldState 是决策管道和规则校验的输入，由 WorldStateBuilder 从 World 构建。

use crate::types::{AgentId, Position, TerrainType, ResourceType, PersonalitySeed};
use crate::world::vision::{NearbyAgentInfo, NearbyStructureInfo, NearbyLegacyInfo};
use std::collections::{HashMap, HashSet};

/// 待处理交易信息（用于 AI 了解有待处理的交易提议）
#[derive(Debug, Clone)]
pub struct PendingTradeInfo {
    pub trade_id: String,
    pub proposer_name: String,
    pub proposer_id: AgentId,
    pub offer: HashMap<ResourceType, u32>,
    pub want: HashMap<ResourceType, u32>,
}

/// 待处理结盟请求信息（用于 AI 了解有待处理的结盟请求）
#[derive(Debug, Clone)]
pub struct PendingAllyRequestInfo {
    pub ally_id: AgentId,
    pub proposer_name: String,
}

/// 世界状态快照（用于决策和规则校验）
#[derive(Debug, Clone)]
pub struct WorldState {
    pub map_size: u32,
    pub agent_position: Position,
    pub agent_inventory: HashMap<ResourceType, u32>,
    pub agent_satiety: u32,
    pub agent_hydration: u32,
    pub terrain_at: HashMap<Position, TerrainType>,
    pub self_id: AgentId,
    /// Agent名称（用于判断建筑归属）
    pub agent_name: String,
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
    /// Agent性格描述（用于Prompt注入）
    pub agent_personality: Option<PersonalitySeed>,
    /// 待处理的交易提议（AI 收到的交易请求）
    pub pending_trades: Vec<PendingTradeInfo>,
    /// 待处理的结盟请求（AI 收到的结盟请求）
    pub pending_ally_requests: Vec<PendingAllyRequestInfo>,
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
            agent_name: String::new(),
            existing_agents: HashSet::new(),
            resources_at: HashMap::new(),
            nearby_agents: Vec::new(),
            nearby_structures: Vec::new(),
            nearby_legacies: Vec::new(),
            active_pressures: Vec::new(),
            last_move_direction: None,
            temp_preferences: Vec::new(),
            agent_personality: None,
            pending_trades: Vec::new(),
            pending_ally_requests: Vec::new(),
        }
    }
}