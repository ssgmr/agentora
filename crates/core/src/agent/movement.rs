//! 移动与感知系统

use crate::types::{Position, Direction, TerrainType, AgentId};

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
    pub fn perceive_nearby(&self, world_agents: &[AgentId], world_positions: impl Fn(AgentId) -> Option<Position>) -> Perception {
        let radius = 5u32;
        let mut nearby_agents = Vec::new();
        let mut nearby_resources = Vec::new();

        // TODO: 实现完整的感知逻辑

        Perception {
            nearby_agents,
            nearby_resources,
            position: self.position,
        }
    }
}

/// 感知结果
#[derive(Debug, Clone)]
pub struct Perception {
    pub nearby_agents: Vec<(AgentId, Position)>,
    pub nearby_resources: Vec<(Position, String)>,
    pub position: Position,
}