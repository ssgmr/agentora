//! 视觉感知模块：圆形视野扫描，填充地形/资源/Agent/关系数据

use crate::agent::{Agent, RelationType};
use crate::types::{AgentId, Position, TerrainType, ResourceType};
use crate::world::World;
use std::collections::HashMap;

/// 附近 Agent 信息
#[derive(Debug, Clone)]
pub struct NearbyAgentInfo {
    pub id: AgentId,
    pub name: String,
    pub position: Position,
    pub distance: u32,                   // 曼哈顿距离
    pub motivation_summary: [f32; 6],
    pub relation_type: RelationType,     // 对自己的关系
    pub trust: f32,
}

/// 视野扫描结果
#[derive(Debug, Clone)]
pub struct VisionScanResult {
    pub self_position: Position,
    pub terrain_at: HashMap<Position, TerrainType>,
    pub resources_at: HashMap<Position, (ResourceType, u32)>,
    pub nearby_agents: Vec<NearbyAgentInfo>,
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

            // 2. 资源：O(1) HashMap 查询
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
                            motivation_summary: other.motivation.to_array(),
                            relation_type,
                            trust,
                        });
                    }
                }
            }
        }
    }

    VisionScanResult {
        self_position: agent.position,
        terrain_at,
        resources_at,
        nearby_agents,
    }
}
