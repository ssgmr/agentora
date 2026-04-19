//! 策略自我改进（Patch）

use crate::strategy::{Strategy, StrategyHub};
use std::fs;
use std::path::PathBuf;

/// 问题类型
#[derive(Debug, Clone)]
pub enum PatchProblem {
    Outdated,     // 条件已变化
    Incomplete,   // 步骤遗漏
    Wrong,        // 导致负面结果
}

/// 检测策略问题
pub fn detect_problem(echo_result: &str) -> Option<PatchProblem> {
    match echo_result {
        "fail" => Some(PatchProblem::Wrong),
        "regret" => Some(PatchProblem::Outdated),
        "partial_success" => Some(PatchProblem::Incomplete),
        _ => None,
    }
}

/// 策略 Patch 执行（实际修改文件）
pub fn patch_strategy(
    hub: &StrategyHub,
    spark_type: &str,
    find: &str,
    replace: &str,
    tick: u32,
) -> std::io::Result<String> {
    // 加载策略文件
    let mut strategy_file = match hub.load_strategy(spark_type) {
        Some(f) => f,
        None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Strategy not found")),
    };

    // 执行 find/replace
    let mut patch_log = String::new();
    if strategy_file.content.contains(find) {
        strategy_file.content = strategy_file.content.replace(find, replace);
        patch_log = format!("Replaced '{}' with '{}'", find, replace);
    } else {
        patch_log = format!("Pattern '{}' not found in content", find);
    }

    // 更新 frontmatter
    strategy_file.frontmatter.last_used_tick = tick;

    // 保存到文件
    let strategy = Strategy {
        spark_type: strategy_file.frontmatter.spark_type.clone(),
        success_rate: strategy_file.frontmatter.success_rate,
        use_count: strategy_file.frontmatter.use_count,
        last_used_tick: strategy_file.frontmatter.last_used_tick,
        created_tick: strategy_file.frontmatter.created_tick,
        deprecated: strategy_file.frontmatter.deprecated,
        content: strategy_file.content.clone(),
    };

    hub.save_strategy(&strategy)?;

    Ok(patch_log)
}

/// 更新策略 frontmatter
pub fn update_frontmatter(
    hub: &StrategyHub,
    spark_type: &str,
    success: bool,
    tick: u32,
) -> std::io::Result<()> {
    let mut strategy_file = match hub.load_strategy(spark_type) {
        Some(f) => f,
        None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Strategy not found")),
    };

    // 更新 use_count
    strategy_file.frontmatter.use_count += 1;
    strategy_file.frontmatter.last_used_tick = tick;

    // 更新成功率
    if success {
        strategy_file.frontmatter.success_rate = (strategy_file.frontmatter.success_rate * (strategy_file.frontmatter.use_count - 1) as f32 + 1.0)
            / strategy_file.frontmatter.use_count as f32;
    } else {
        strategy_file.frontmatter.success_rate = (strategy_file.frontmatter.success_rate * (strategy_file.frontmatter.use_count - 1) as f32)
            / strategy_file.frontmatter.use_count as f32;
    }

    // 保存
    let strategy = Strategy {
        spark_type: strategy_file.frontmatter.spark_type.clone(),
        success_rate: strategy_file.frontmatter.success_rate,
        use_count: strategy_file.frontmatter.use_count,
        last_used_tick: strategy_file.frontmatter.last_used_tick,
        created_tick: strategy_file.frontmatter.created_tick,
        deprecated: strategy_file.frontmatter.deprecated,
        content: strategy_file.content.clone(),
    };

    hub.save_strategy(&strategy)
}

/// 记录 Patch 日志
pub fn log_patch(logs_dir: &PathBuf, tick: u32, strategy_name: &str, problem: &PatchProblem, patch_content: &str) -> std::io::Result<String> {
    fs::create_dir_all(logs_dir)?;

    let log_path = logs_dir.join(format!("{}_patch.md", tick));
    let log_content = format!(
        "# Patch Log - Tick {}\n\n**Strategy:** {}\n**Problem:** {:?}\n\n**Changes:**\n{}\n",
        tick, strategy_name, problem, patch_content
    );

    fs::write(&log_path, &log_content)?;

    Ok(log_content)
}
