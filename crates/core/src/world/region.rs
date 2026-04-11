//! 区域划分系统

use serde::{Deserialize, Serialize};

/// 区域定义（16×16格子）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: u32,
    pub center_x: u32,
    pub center_y: u32,
    pub size: u32,
    pub name: String,
    pub resource_multiplier: f32,
    pub is_blockaded: bool,
    pub blockade_remaining_ticks: u32,
}

impl Region {
    pub fn new(id: u32, center_x: u32, center_y: u32, size: u32) -> Self {
        Self {
            id,
            center_x,
            center_y,
            size,
            name: format!("Region_{}", id),
            resource_multiplier: 1.0,
            is_blockaded: false,
            blockade_remaining_ticks: 0,
        }
    }

    /// 从位置计算所属区域ID
    pub fn position_to_region_id(x: u32, y: u32, region_size: u32) -> u32 {
        (y / region_size) * 1000 + (x / region_size)
    }

    /// 检查位置是否在此区域内
    pub fn contains(&self, x: u32, y: u32) -> bool {
        let min_x = self.center_x.saturating_sub(self.size / 2);
        let max_x = self.center_x + self.size / 2;
        let min_y = self.center_y.saturating_sub(self.size / 2);
        let max_y = self.center_y + self.size / 2;
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }
}