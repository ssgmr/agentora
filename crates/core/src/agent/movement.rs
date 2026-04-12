//! 移动与感知系统

use crate::types::{Position, Direction, TerrainType, AgentId, ResourceType};

/// 感知到的 Agent
#[derive(Debug, Clone)]
pub struct PerceivedAgent {
    pub id: AgentId,
    pub position: Position,
    pub motivation_summary: [f32; 6],
}

/// 感知到的资源
#[derive(Debug, Clone)]
pub struct PerceivedResource {
    pub position: Position,
    pub resource_type: ResourceType,
    pub amount: u32,
}

/// 感知结果
#[derive(Debug, Clone)]
pub struct PerceptionResult {
    pub nearby_agents: Vec<PerceivedAgent>,
    pub nearby_resources: Vec<PerceivedResource>,
    pub position: Position,
}

impl crate::agent::Agent {
    /// 向指定方向移动
    pub fn move_direction(&mut self, direction: Direction, terrain_at: impl Fn(Position) -> TerrainType) -> bool {
        let delta = direction.delta();
        let new_x = self.position.x as i32 + delta.0;
        let new_y = self.position.y as i32 + delta.1;

        if new_x < 0 || new_y < 0 {
            return false;
        }

        let new_pos = Position::new(new_x as u32, new_y as u32);

        if terrain_at(new_pos).is_passable() {
            self.position = new_pos;
            return true;
        }

        false
    }

    /// 感知视野内的环境
    /// 视野半径为5格
    pub fn perceive_nearby<F1, F2, F3>(
        &self,
        get_agent_data: F1,
        mut get_resource_data: F2,
        map_size: u32,
    ) -> PerceptionResult
    where
        F1: Fn(&AgentId) -> Option<(Position, [f32; 6])>,
        F2: FnMut(&Position) -> Option<(ResourceType, u32)>,
    {
        let radius = 5u32;
        let mut nearby_agents = Vec::new();
        let mut nearby_resources = Vec::new();

        // 扫描视野范围内的所有位置
        let min_x = self.position.x.saturating_sub(radius);
        let max_x = (self.position.x + radius).min(map_size.saturating_sub(1));
        let min_y = self.position.y.saturating_sub(radius);
        let max_y = (self.position.y + radius).min(map_size.saturating_sub(1));

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let pos = Position::new(x, y);
                // 跳过自己所在位置
                if pos == self.position {
                    continue;
                }

                // 检查资源
                if let Some((resource_type, amount)) = get_resource_data(&pos) {
                    nearby_resources.push(PerceivedResource {
                        position: pos,
                        resource_type,
                        amount,
                    });
                }
            }
        }

        // 感知 Agent（通过传入的列表查询）
        // 这里需要一个外部传入的 Agent 列表来遍历
        // 实际使用时由 World::apply_action 中调用并传入数据

        PerceptionResult {
            nearby_agents,
            nearby_resources,
            position: self.position,
        }
    }
}