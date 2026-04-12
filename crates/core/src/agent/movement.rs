//! 移动系统

use crate::types::{Position, Direction, TerrainType};

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
}
