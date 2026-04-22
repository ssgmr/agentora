//! 移动系统：Agent 位置变更

use crate::types::Position;

impl crate::agent::Agent {
    /// 移动到目标位置
    /// 返回：(是否成功, 旧位置, 新位置)
    pub fn move_to(&mut self, target: Position) -> (bool, Position, Position) {
        let old_pos = self.position;
        self.last_position = Some(old_pos);
        self.position = target;
        (true, old_pos, target)
    }
}