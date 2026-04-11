//! 网格地图与地形

use crate::types::{Position, TerrainType};

/// 256×256单元格网格
pub struct CellGrid {
    width: u32,
    height: u32,
    cells: Vec<TerrainType>,
}

impl CellGrid {
    pub fn new(width: u32, height: u32) -> Self {
        // 默认全部为平原
        let cells = vec![TerrainType::Plains; (width * height) as usize];
        Self { width, height, cells }
    }

    /// 获取位置索引
    fn index(&self, pos: Position) -> usize {
        (pos.y * self.width + pos.x) as usize
    }

    /// 检查位置是否有效
    pub fn is_valid(&self, pos: Position) -> bool {
        pos.x < self.width && pos.y < self.height
    }

    /// 获取地形类型
    pub fn get_terrain(&self, pos: Position) -> TerrainType {
        if self.is_valid(pos) {
            self.cells[self.index(pos)]
        } else {
            TerrainType::Mountain // 越界视为不可通行
        }
    }

    /// 设置地形类型
    pub fn set_terrain(&mut self, pos: Position, terrain: TerrainType) {
        if self.is_valid(pos) {
            let idx = (pos.y * self.width + pos.x) as usize;
            self.cells[idx] = terrain;
        }
    }

    /// 获取地图尺寸
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}