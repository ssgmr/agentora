//! 世界生成器入口（已废弃，生成逻辑已移至 mod.rs）

use crate::seed::WorldSeed;
use crate::world::World;

/// 世界生成器（入口函数）
///
/// 注意：实际生成逻辑在 World::new() 中实现
/// 此函数仅为便捷入口
pub struct WorldGenerator;

impl WorldGenerator {
    /// 从 WorldSeed 生成世界
    /// 直接调用 World::new()，所有生成逻辑已整合到 World 结构体中
    pub fn generate(seed: &WorldSeed) -> World {
        World::new(seed)
    }
}