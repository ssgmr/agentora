//! 核心共享类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent唯一标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// 2D坐标位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

impl Position {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    /// 计算与另一位置的曼哈顿距离
    pub fn manhattan_distance(&self, other: &Position) -> u32 {
        (self.x as i32 - other.x as i32).abs() as u32 + (self.y as i32 - other.y as i32).abs() as u32
    }
}

/// 方向枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    /// 获取方向对应的位移
    pub fn delta(&self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
        }
    }
}

/// 资源类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Iron,      // 铁矿
    Food,      // 食物
    Wood,      // 木材
    Water,     // 水源
    Stone,     // 石材
}

impl ResourceType {
    pub fn as_str(&self) -> &str {
        match self {
            ResourceType::Iron => "iron",
            ResourceType::Food => "food",
            ResourceType::Wood => "wood",
            ResourceType::Water => "water",
            ResourceType::Stone => "stone",
        }
    }
}

/// 地形类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainType {
    Plains,    // 平原
    Forest,    // 森林
    Mountain,  // 山地
    Water,     // 水域
    Desert,    // 沙漠
}

impl TerrainType {
    /// 判断地形是否可通行
    pub fn is_passable(&self) -> bool {
        match self {
            TerrainType::Plains | TerrainType::Forest | TerrainType::Desert => true,
            TerrainType::Mountain | TerrainType::Water => false,
        }
    }
}

/// 结构类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureType {
    Camp,      // 营地
    Fence,     // 围栏
    Warehouse, // 仓库
}

impl StructureType {
    /// 建造所需的资源消耗
    pub fn resource_cost(&self) -> HashMap<ResourceType, u32> {
        match self {
            StructureType::Camp => {
                [(ResourceType::Wood, 5), (ResourceType::Stone, 2)].into_iter().collect()
            }
            StructureType::Fence => {
                [(ResourceType::Wood, 2)].into_iter().collect()
            }
            StructureType::Warehouse => {
                [(ResourceType::Wood, 10), (ResourceType::Stone, 5)].into_iter().collect()
            }
        }
    }
}

/// 动作类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Move { direction: Direction },
    MoveToward { target: Position },  // 导航到目标位置，每次移动一格
    Gather { resource: ResourceType },
    TradeOffer { offer: HashMap<ResourceType, u32>, want: HashMap<ResourceType, u32>, target_id: AgentId },
    TradeAccept { trade_id: String },
    TradeReject { trade_id: String },
    Talk { message: String },
    Attack { target_id: AgentId },
    Build { structure: StructureType },
    AllyPropose { target_id: AgentId },
    AllyAccept { ally_id: AgentId },
    AllyReject { ally_id: AgentId },
    Explore { target_region: u32 },
    Wait,
    InteractLegacy { legacy_id: String, interaction: LegacyInteraction },
}

/// 遗产交互类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyInteraction {
    Worship,   // 祭拜
    Explore,   // 探索遗迹
    Pickup,    // 拾取物品
}

/// 结构化动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub reasoning: String,
    pub action_type: ActionType,
    pub target: Option<String>,
    pub params: HashMap<String, String>,
    pub build_type: Option<StructureType>,  // Build 动作专用参数
    pub direction: Option<Direction>,       // Move 动作专用参数
    pub motivation_delta: [f32; 6],
}

/// 人格种子（大五人格三维）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PersonalitySeed {
    pub openness: f32,       // 开放性 [0.0, 1.0]
    pub agreeableness: f32,  // 宜人性 [0.0, 1.0]
    pub neuroticism: f32,    // 神经质 [0.0, 1.0]
}

impl Default for PersonalitySeed {
    fn default() -> Self {
        Self {
            openness: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
        }
    }
}

/// Peer节点标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}