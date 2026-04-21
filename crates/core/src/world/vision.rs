//! 视觉感知模块：圆形视野扫描，填充地形/资源/Agent/结构/遗产数据

use crate::agent::RelationType;
use crate::types::{AgentId, Direction, Position, TerrainType, ResourceType, StructureType};
use crate::world::World;
use crate::world::legacy::LegacyType;
use std::collections::HashMap;

/// 附近 Agent 信息
#[derive(Debug, Clone)]
pub struct NearbyAgentInfo {
    pub id: AgentId,
    pub name: String,
    pub position: Position,
    pub distance: u32,                   // 曼哈顿距离
    pub relation_type: RelationType,     // 对自己的关系
    pub trust: f32,
}

/// 附近结构信息
#[derive(Debug, Clone)]
pub struct NearbyStructureInfo {
    pub position: Position,
    pub structure_type: StructureType,
    pub owner_name: Option<String>,
    pub durability: u32,
    pub distance: u32,
}

/// 附近遗产信息
#[derive(Debug, Clone)]
pub struct NearbyLegacyInfo {
    pub position: Position,
    pub legacy_type: LegacyType,
    pub original_agent_name: String,
    pub has_items: bool,
    pub distance: u32,
}

/// 视野扫描结果
#[derive(Debug, Clone)]
pub struct VisionScanResult {
    pub self_position: Position,
    pub terrain_at: HashMap<Position, TerrainType>,
    pub resources_at: HashMap<Position, (ResourceType, u32)>,
    pub nearby_agents: Vec<NearbyAgentInfo>,
    pub nearby_structures: Vec<NearbyStructureInfo>,
    pub nearby_legacies: Vec<NearbyLegacyInfo>,
}

/// 扫描指定 Agent 的视野
///
/// 遍历中心 ±radius 的方形区域，曼哈顿距离过滤，O(r²) 常数时间复杂度
pub fn scan_vision(world: &World, agent_id: &AgentId, radius: u32) -> VisionScanResult {
    let agent = match world.agents.get(agent_id) {
        Some(a) => a,
        None => {
            return VisionScanResult {
                self_position: Position::new(0, 0),
                terrain_at: HashMap::new(),
                resources_at: HashMap::new(),
                nearby_agents: Vec::new(),
                nearby_structures: Vec::new(),
                nearby_legacies: Vec::new(),
            };
        }
    };

    let cx = agent.position.x;
    let cy = agent.position.y;
    let (map_width, map_height) = world.map.size();
    let min_x = cx.saturating_sub(radius);
    let max_x = (cx + radius).min(map_width.saturating_sub(1));
    let min_y = cy.saturating_sub(radius);
    let max_y = (cy + radius).min(map_height.saturating_sub(1));

    let mut terrain_at = HashMap::new();
    let mut resources_at = HashMap::new();
    let mut nearby_agents = Vec::new();
    let mut nearby_structures = Vec::new();
    let mut nearby_legacies = Vec::new();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let pos = Position::new(x, y);

            // 曼哈顿距离过滤
            if pos.manhattan_distance(&agent.position) > radius {
                continue;
            }

            // 跳过自身所在格子（地形资源仍需扫描，但跳过 Agent 探测）
            let is_self_position = pos == agent.position;

            // 1. 地形：O(1)
            let terrain = world.map.get_terrain(pos);
            terrain_at.insert(pos, terrain);

            // 2. 源：O(1) HashMap 查询
            if let Some(node) = world.resources.get(&pos) {
                resources_at.insert(pos, (node.resource_type, node.current_amount));
            }

            if is_self_position {
                continue;
            }

            // 3. Agent：通过反向索引 O(1)
            if let Some(ids) = world.agent_positions.get(&pos) {
                for other_id in ids {
                    if *other_id == *agent_id {
                        continue; // 跳过自己
                    }
                    if let Some(other) = world.agents.get(other_id) {
                        // 查当前 Agent 的 relations 获取关系数据
                        let (relation_type, trust) = agent
                            .relations
                            .get(other_id)
                            .map(|r| (r.relation_type, r.trust))
                            .unwrap_or((RelationType::Neutral, 0.0));

                        nearby_agents.push(NearbyAgentInfo {
                            id: other.id.clone(),
                            name: other.name.clone(),
                            position: other.position,
                            distance: pos.manhattan_distance(&agent.position),
                            relation_type,
                            trust,
                        });
                    }
                }
            }
        }
    }

    // 扫描视野内的结构
    for (pos, structure) in &world.structures {
        if pos.manhattan_distance(&agent.position) <= radius {
            let owner_name = structure.owner_id.as_ref().and_then(|id| {
                world.agents.get(id).map(|a| a.name.clone())
            });
            nearby_structures.push(NearbyStructureInfo {
                position: *pos,
                structure_type: structure.structure_type,
                owner_name,
                durability: structure.durability,
                distance: pos.manhattan_distance(&agent.position),
            });
        }
    }

    // 扫描视野内的遗产
    for legacy in &world.legacies {
        if legacy.position.manhattan_distance(&agent.position) <= radius {
            nearby_legacies.push(NearbyLegacyInfo {
                position: legacy.position,
                legacy_type: legacy.legacy_type,
                original_agent_name: legacy.original_agent_name.clone(),
                has_items: !legacy.items.is_empty(),
                distance: legacy.position.manhattan_distance(&agent.position),
            });
        }
    }

    VisionScanResult {
        self_position: agent.position,
        terrain_at,
        resources_at,
        nearby_agents,
        nearby_structures,
        nearby_legacies,
    }
}

/// 计算从源位置到目标位置的主要移动方向
///
/// 返回东/南/西/北四个方向之一，优先选择位移绝对值较大的方向
/// 如果源位置等于目标位置，返回 None
pub fn calculate_direction(from: &Position, to: &Position) -> Option<Direction> {
    let dx = to.x as i32 - from.x as i32;
    let dy = to.y as i32 - from.y as i32;

    if dx == 0 && dy == 0 {
        return None; // 已在目标位置
    }

    // 东西方向优先（取绝对值较大的）
    if dx.abs() >= dy.abs() {
        if dx > 0 {
            Some(Direction::East)
        } else {
            Some(Direction::West)
        }
    } else {
        if dy > 0 {
            Some(Direction::South)
        } else {
            Some(Direction::North)
        }
    }
}

/// 计算方向的中文描述（用于感知摘要）
///
/// 返回格式如 "东北方向，距5格"、"东方向，距3格"
/// 曼哈顿距离作为距离度量
pub fn direction_description(from: &Position, to: &Position) -> String {
    let dx = to.x as i32 - from.x as i32;
    let dy = to.y as i32 - from.y as i32;
    let distance = dx.abs() + dy.abs();

    // 根据 dx/dy 的符号组合判断方向
    let direction = match (dx.cmp(&0), dy.cmp(&0)) {
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

    format!("{}方向，距{}格", direction, distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_direction_cardinal() {
        let center = Position::new(10, 10);

        // 四个基本方向
        assert_eq!(calculate_direction(&center, &Position::new(12, 10)), Some(Direction::East));
        assert_eq!(calculate_direction(&center, &Position::new(8, 10)), Some(Direction::West));
        assert_eq!(calculate_direction(&center, &Position::new(10, 12)), Some(Direction::South));
        assert_eq!(calculate_direction(&center, &Position::new(10, 8)), Some(Direction::North));
    }

    #[test]
    fn test_calculate_direction_diagonal() {
        let center = Position::new(10, 10);

        // 对角线方向：优先选择 |dx| >= |dy| 的方向（东西优先）
        assert_eq!(calculate_direction(&center, &Position::new(12, 8)), Some(Direction::East));  // dx=2, dy=-2
        assert_eq!(calculate_direction(&center, &Position::new(8, 12)), Some(Direction::West));   // dx=-2, dy=2
        assert_eq!(calculate_direction(&center, &Position::new(11, 13)), Some(Direction::South)); // dx=1, dy=3, 南北优先
        assert_eq!(calculate_direction(&center, &Position::new(9, 7)), Some(Direction::North));   // dx=-1, dy=-3, 南北优先
    }

    #[test]
    fn test_calculate_direction_same_position() {
        let pos = Position::new(10, 10);
        assert_eq!(calculate_direction(&pos, &pos), None);
    }

    #[test]
    fn test_direction_description() {
        let center = Position::new(10, 10);

        assert_eq!(direction_description(&center, &Position::new(12, 8)), "东北方向，距4格");
        assert_eq!(direction_description(&center, &Position::new(12, 12)), "东南方向，距4格");
        assert_eq!(direction_description(&center, &Position::new(8, 8)), "西北方向，距4格");
        assert_eq!(direction_description(&center, &Position::new(8, 12)), "西南方向，距4格");
        assert_eq!(direction_description(&center, &Position::new(15, 10)), "东方向，距5格");
        assert_eq!(direction_description(&center, &Position::new(10, 5)), "北方向，距5格");
        assert_eq!(direction_description(&center, &center), "原地方向，距0格");
    }
}