//! 策略衰减机制

use crate::strategy::{Strategy, StrategyHub, StrategyFrontmatter};

const DECAY_FACTOR: f32 = 0.95;
const DECAY_INTERVAL: u32 = 50;
const DEPRECATION_THRESHOLD: f32 = 0.3;
const AUTO_DELETE_THRESHOLD: u32 = 100;

/// 对所有策略执行衰减（每 50 tick 调用）
pub fn decay_all_strategies(hub: &StrategyHub, current_tick: u32) -> std::io::Result<()> {
    let strategies = hub.list_strategies()?;

    for spark_type in strategies {
        if let Some(mut strategy_file) = hub.load_strategy(&spark_type) {
            // 仅在未使用时衰减
            if current_tick - strategy_file.frontmatter.last_used_tick >= DECAY_INTERVAL {
                strategy_file.frontmatter.success_rate *= DECAY_FACTOR;

                // 保存更新后的策略
                let strategy = Strategy {
                    spark_type: strategy_file.frontmatter.spark_type.clone(),
                    success_rate: strategy_file.frontmatter.success_rate,
                    use_count: strategy_file.frontmatter.use_count,
                    last_used_tick: strategy_file.frontmatter.last_used_tick,
                    created_tick: strategy_file.frontmatter.created_tick,
                    deprecated: strategy_file.frontmatter.deprecated,
                    motivation_delta: strategy_file.frontmatter.motivation_delta,
                    content: strategy_file.content.clone(),
                };
                hub.save_strategy(&strategy)?;
            }
        }
    }

    Ok(())
}

/// 检查废弃标记
pub fn check_deprecation(hub: &StrategyHub) -> std::io::Result<Vec<String>> {
    let mut deprecated = Vec::new();
    let strategies = hub.list_strategies()?;

    for spark_type in strategies {
        if let Some(mut strategy_file) = hub.load_strategy(&spark_type) {
            if strategy_file.frontmatter.success_rate < DEPRECATION_THRESHOLD {
                strategy_file.frontmatter.deprecated = true;

                let strategy = Strategy {
                    spark_type: strategy_file.frontmatter.spark_type.clone(),
                    success_rate: strategy_file.frontmatter.success_rate,
                    use_count: strategy_file.frontmatter.use_count,
                    last_used_tick: strategy_file.frontmatter.last_used_tick,
                    created_tick: strategy_file.frontmatter.created_tick,
                    deprecated: true,
                    motivation_delta: strategy_file.frontmatter.motivation_delta,
                    content: strategy_file.content.clone(),
                };
                hub.save_strategy(&strategy)?;
                deprecated.push(spark_type);
            }
        }
    }

    Ok(deprecated)
}

/// 自动删除废弃策略（deprecated 且 100 tick 未使用）
pub fn auto_delete_deprecated(hub: &StrategyHub, current_tick: u32) -> std::io::Result<Vec<String>> {
    let mut deleted = Vec::new();
    let strategies = hub.list_strategies()?;

    for spark_type in strategies {
        if let Some(strategy_file) = hub.load_strategy(&spark_type) {
            if strategy_file.frontmatter.deprecated
                && current_tick - strategy_file.frontmatter.last_used_tick >= AUTO_DELETE_THRESHOLD
            {
                hub.delete_strategy(&spark_type)?;
                deleted.push(spark_type);
            }
        }
    }

    Ok(deleted)
}

/// 检查是否应该自动删除
pub fn should_auto_delete(strategy: &StrategyFrontmatter, current_tick: u32) -> bool {
    strategy.deprecated && current_tick - strategy.last_used_tick >= AUTO_DELETE_THRESHOLD
}
