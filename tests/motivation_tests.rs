//! 单元测试 - 动机引擎
//!
//! 测试惯性衰减、缺口计算、事件微调

use agentora_core::motivation::{MotivationVector, DECAY_ALPHA, NEUTRAL_VALUE};

#[test]
fn test_motivation_vector_new() {
    let vec = MotivationVector::new();
    for i in 0..6 {
        assert_eq!(vec[i], NEUTRAL_VALUE);
    }
}

#[test]
fn test_motivation_vector_from_array() {
    let values = [0.8, 0.6, 0.4, 0.3, 0.7, 0.5];
    let vec = MotivationVector::from_array(values);

    assert_eq!(vec[0], 0.8);
    assert_eq!(vec[1], 0.6);
}

#[test]
fn test_motivation_vector_clamp() {
    let values = [1.5, -0.3, 0.5, 0.5, 0.5, 0.5];
    let vec = MotivationVector::from_array(values);

    // 越界值应被截断
    assert_eq!(vec[0], 1.0);
    assert_eq!(vec[1], 0.0);
}

#[test]
fn test_decay_convergence() {
    let mut vec = MotivationVector::from_array([1.0, 0.0, 0.8, 0.2, 0.9, 0.1]);

    // 连续衰减10次
    for _ in 0..10 {
        vec.decay();
    }

    // 应收敛到接近0.5
    for i in 0..6 {
        let diff = (vec[i] - NEUTRAL_VALUE).abs();
        assert!(diff < 0.1, "维度{}未收敛: {} vs {}", i, vec[i], NEUTRAL_VALUE);
    }
}

#[test]
fn test_decay_formula() {
    let mut vec = MotivationVector::from_array([1.0, 0.5, 0.5, 0.5, 0.5, 0.5]);
    vec.decay();

    // 公式: new = old * α + 0.5 * (1 - α)
    // 对于初始值1.0: 1.0 * 0.85 + 0.5 * 0.15 = 0.85 + 0.075 = 0.925
    let expected = 1.0 * DECAY_ALPHA + NEUTRAL_VALUE * (1.0 - DECAY_ALPHA);
    assert!((vec[0] - expected).abs() < 0.001);
}

#[test]
fn test_apply_delta() {
    let mut vec = MotivationVector::new();
    let delta = [0.1, -0.1, 0.05, 0.0, 0.0, 0.0];

    vec.apply_delta(delta, 1.0);

    assert_eq!(vec[0], 0.6);  // 0.5 + 0.1
    assert_eq!(vec[1], 0.4);  // 0.5 - 0.1
}

#[test]
fn test_compute_gap() {
    let vec = MotivationVector::from_array([0.8, 0.5, 0.3, 0.5, 0.5, 0.5]);
    let satisfaction = [0.4, 0.5, 0.5, 0.5, 0.5, 0.5];

    let gap = vec.compute_gap(&satisfaction);

    // gap = max(0, dimension - satisfaction)
    assert_eq!(gap[0], 0.4);  // 0.8 - 0.4
    assert_eq!(gap[1], 0.0);  // 0.5 - 0.5
    assert_eq!(gap[2], 0.0);  // max(0, 0.3 - 0.5) = 0
}

#[test]
fn test_max_gap_dimension() {
    let vec = MotivationVector::from_array([0.9, 0.5, 0.5, 0.5, 0.5, 0.5]);
    let satisfaction = [0.3, 0.5, 0.5, 0.5, 0.5, 0.5];

    let max_dim = vec.max_gap_dimension(&satisfaction);

    // 维度0缺口最大: 0.9 - 0.3 = 0.6
    assert_eq!(max_dim, 0);
}

#[test]
fn test_personality_modifier() {
    let mut vec = MotivationVector::new();
    let delta = [0.1, 0.0, 0.0, 0.0, 0.0, 0.0];

    // 高开放性人格应该放大认知维度响应
    vec.apply_delta(delta, 1.3);  // openness > 0.7

    assert_eq!(vec[0], 0.5 + 0.1 * 1.3);
}