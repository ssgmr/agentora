//! 6维动机向量引擎
//!
//! 动机维度：生存与资源、社会与关系、认知与好奇、表达与创造、权力与影响、意义与传承

use serde::{Deserialize, Serialize};

/// 动机向量索引常量
pub const DIM_SURVIVAL: usize = 0;      // 生存与资源
pub const DIM_SOCIAL: usize = 1;        // 社会与关系
pub const DIM_COGNITIVE: usize = 2;     // 认知与好奇
pub const DIM_EXPRESSIVE: usize = 3;    // 表达与创造
pub const DIM_POWER: usize = 4;         // 权力与影响
pub const DIM_LEGACY: usize = 5;        // 意义与传承

/// 动机维度名称
pub const DIMENSION_NAMES: [&str; 6] = [
    "生存与资源",
    "社会与关系",
    "认知与好奇",
    "表达与创造",
    "权力与影响",
    "意义与传承",
];

/// 惯性衰减系数（值越大衰减越慢，动机变化越持久）
/// 0.99 意味着每 tick 只向中性值回归 1%，6 次 decay 后仍保留约 94% 的变化
pub const DECAY_ALPHA: f32 = 0.99;
pub const NEUTRAL_VALUE: f32 = 0.5;

/// 6维动机向量
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MotivationVector([f32; 6]);

impl MotivationVector {
    /// 创建默认动机向量（全部为0.5中性值）
    pub fn new() -> Self {
        Self([NEUTRAL_VALUE, NEUTRAL_VALUE, NEUTRAL_VALUE, NEUTRAL_VALUE, NEUTRAL_VALUE, NEUTRAL_VALUE])
    }

    /// 从数组创建动机向量
    pub fn from_array(values: [f32; 6]) -> Self {
        let mut vec = Self(values);
        vec.clamp_all();
        vec
    }

    /// 获取指定维度的值
    pub fn get(&self, dim: usize) -> f32 {
        self.0[dim]
    }

    /// 设置指定维度的值（自动截断到合法范围）
    pub fn set(&mut self, dim: usize, value: f32) {
        self.0[dim] = value.clamp(0.0, 1.0);
    }

    /// 获取内部数组引用
    pub fn as_array(&self) -> &[f32; 6] {
        &self.0
    }

    /// 惯性衰减：向中性值0.5收敛
    /// 公式: new = old * α + 0.5 * (1 - α)
    pub fn decay(&mut self) {
        for i in 0..6 {
            self.0[i] = self.0[i] * DECAY_ALPHA + NEUTRAL_VALUE * (1.0 - DECAY_ALPHA);
        }
        self.clamp_all();
    }

    /// 应用动机变化delta
    pub fn apply_delta(&mut self, delta: [f32; 6], personality_modifier: f32) {
        for i in 0..6 {
            self.0[i] += delta[i] * personality_modifier;
        }
        self.clamp_all();
    }

    /// 截断所有维度到[0.0, 1.0]
    fn clamp_all(&mut self) {
        for i in 0..6 {
            self.0[i] = self.0[i].clamp(0.0, 1.0);
        }
    }

    /// 计算动机缺口
    /// gap = max(0, dimension - satisfaction)
    pub fn compute_gap(&self, satisfaction: &[f32; 6]) -> [f32; 6] {
        let mut gap = [0.0; 6];
        for i in 0..6 {
            gap[i] = (self.0[i] - satisfaction[i]).max(0.0);
        }
        gap
    }

    /// 获取缺口最大的维度索引
    pub fn max_gap_dimension(&self, satisfaction: &[f32; 6]) -> usize {
        let gap = self.compute_gap(satisfaction);
        let mut max_idx = 0;
        let mut max_val = gap[0];
        for i in 1..6 {
            if gap[i] > max_val {
                max_val = gap[i];
                max_idx = i;
            }
        }
        max_idx
    }
}

impl Default for MotivationVector {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<usize> for MotivationVector {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for MotivationVector {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl MotivationVector {
    /// 转换为数组
    pub fn to_array(&self) -> [f32; 6] {
        self.0
    }
}