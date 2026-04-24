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

    /// 获取方向的中文名称
    pub fn as_chinese(&self) -> &'static str {
        match self {
            Direction::North => "北",
            Direction::South => "南",
            Direction::East => "东",
            Direction::West => "西",
        }
    }

    /// 根据位移判断方向（假设每次只移动一格）
    pub fn from_delta(dx: i32, dy: i32) -> Option<Self> {
        if dx == 0 && dy == -1 { Some(Direction::North) }
        else if dx == 0 && dy == 1 { Some(Direction::South) }
        else if dx == 1 && dy == 0 { Some(Direction::East) }
        else if dx == -1 && dy == 0 { Some(Direction::West) }
        else { None }
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

impl std::str::FromStr for ResourceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "iron" | "铁矿" => Ok(ResourceType::Iron),
            "food" | "食物" => Ok(ResourceType::Food),
            "wood" | "木材" => Ok(ResourceType::Wood),
            "water" | "水源" => Ok(ResourceType::Water),
            "stone" | "石材" => Ok(ResourceType::Stone),
            _ => Err(format!("Unknown resource type: {}", s)),
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
    /// 判断地形是否可通行（所有地形均可移动）
    pub fn is_passable(&self) -> bool {
        true
    }

    /// 地形转数字索引（用于地形网格压缩传输）
    pub fn to_index(&self) -> u8 {
        match self {
            TerrainType::Plains => 0,
            TerrainType::Forest => 1,
            TerrainType::Mountain => 2,
            TerrainType::Water => 3,
            TerrainType::Desert => 4,
        }
    }

    /// 数字索引转地形字符串（用于Godot渲染）
    pub fn from_index(index: u8) -> &'static str {
        match index {
            0 => "plains",
            1 => "forest",
            2 => "mountain",
            3 => "water",
            4 => "desert",
            _ => "plains",
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

    /// 获取结构类型名称字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            StructureType::Camp => "Camp",
            StructureType::Fence => "Fence",
            StructureType::Warehouse => "Warehouse",
        }
    }
}

/// 动作类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    MoveToward { target: Position },  // 导航到目标位置（每次移动一格，支持坐标或方向）
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
    Wait,
    Eat,
    Drink,
    InteractLegacy { legacy_id: String, interaction: LegacyInteraction },
}

/// 遗产交互类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyInteraction {
    Worship,   // 祭拜
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
}

/// 人格种子（大五人格三维）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalitySeed {
    pub openness: f32,       // 开放性 [0.0, 1.0]
    pub agreeableness: f32,  // 宜人性 [0.0, 1.0]
    pub neuroticism: f32,    // 神经质 [0.0, 1.0]
    /// 性格描述文本，注入Prompt影响决策倾向（任务 2.1）
    pub description: String,
}

impl Default for PersonalitySeed {
    fn default() -> Self {
        Self {
            openness: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
            description: String::new(),
        }
    }
}

impl PersonalitySeed {
    /// 从性格模板创建（任务 2.1）
    pub fn from_template(template: &PersonalityTemplate) -> Self {
        Self {
            openness: template.openness,
            agreeableness: template.agreeableness,
            neuroticism: template.neuroticism,
            description: template.description.clone(),
        }
    }
}

/// 性格模板配置（任务 2.2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTemplate {
    pub openness: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
    pub description: String,
}

impl Default for PersonalityTemplate {
    fn default() -> Self {
        Self {
            openness: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
            description: "一个普通的世界居民".to_string(),
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