//! 规则引擎兜底决策
//!
//! 当所有LLM Provider失败时，生成安全动作建议

use serde::{Deserialize, Serialize};

/// 简化的位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimplePosition {
    pub x: u32,
    pub y: u32,
}

/// 简化的动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimpleActionType {
    Wait,
    Move { direction: String },
    Explore { target_region: u32 },
}

/// 兜底动作建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackAction {
    pub reasoning: String,
    pub action_type: SimpleActionType,
    pub motivation_delta: [f32; 6],
}

/// 生成兜底安全动作
pub fn fallback_decision(_position: &SimplePosition, motivation: &[f32; 6]) -> FallbackAction {
    // 简单规则：根据最高动机维度选择动作
    let max_dim = find_max_dimension(motivation);

    match max_dim {
        0 => FallbackAction { // 生存：原地等待（安全）
            reasoning: "LLM失败，规则引擎兜底：原地等待".to_string(),
            action_type: SimpleActionType::Wait,
            motivation_delta: [0.0; 6],
        },
        1 => FallbackAction { // 社交：无法执行，等待
            reasoning: "LLM失败，规则引擎兜底：等待社交机会".to_string(),
            action_type: SimpleActionType::Wait,
            motivation_delta: [0.0; 6],
        },
        2 => FallbackAction { // 认知：探索周围
            reasoning: "LLM失败，规则引擎兜底：探索周围".to_string(),
            action_type: SimpleActionType::Explore { target_region: 0 },
            motivation_delta: [0.0; 6],
        },
        _ => FallbackAction { // 其他：等待
            reasoning: "LLM失败，规则引擎兜底：默认等待".to_string(),
            action_type: SimpleActionType::Wait,
            motivation_delta: [0.0; 6],
        },
    }
}

fn find_max_dimension(motivation: &[f32; 6]) -> usize {
    let mut max_idx = 0;
    let mut max_val = motivation[0];
    for (i, val) in motivation.iter().enumerate() {
        if *val > max_val {
            max_val = *val;
            max_idx = i;
        }
    }
    max_idx
}