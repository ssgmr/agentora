//! 策略与动机向量联动

use crate::motivation::MotivationVector;
use crate::strategy::Strategy;

/// 策略成功强化动机
pub fn on_strategy_success(motivation: &mut MotivationVector, strategy: &Strategy) {
    if let Some(delta) = strategy.motivation_delta {
        // 按 success_rate 加权
        let weighted_delta: [f32; 6] = delta.iter()
            .map(|d| d * strategy.success_rate)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        motivation.apply_delta(weighted_delta, 1.0);
    }
}

/// 策略失败弱化动机（反向调整，系数 0.5）
pub fn on_strategy_failure(motivation: &mut MotivationVector, strategy: &Strategy) {
    if let Some(delta) = strategy.motivation_delta {
        // 反向调整，系数 0.5
        let negated: [f32; 6] = delta.iter()
            .map(|d| -d * 0.5)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        motivation.apply_delta(negated, 1.0);
    }
}

/// 计算成功率权重
pub fn calculate_success_rate_weight(success_count: u32, total_count: u32) -> f32 {
    if total_count == 0 {
        return 1.0;
    }
    success_count as f32 / total_count as f32
}
