//! 策略检索与应用

use crate::strategy::{StrategyHub, StrategyFile};
use crate::decision::SparkType;

/// 检索匹配 Spark 类型的策略
pub fn retrieve_strategy(hub: &StrategyHub, spark_type: SparkType) -> Option<StrategyFile> {
    let type_name = spark_type_name(spark_type);
    hub.load_strategy(&type_name)
}

/// 获取策略摘要（用于 Prompt 注入）
pub fn get_strategy_summary(strategy: &StrategyFile) -> String {
    format!(
        "策略：{} (成功率 {:.0}%, 使用{}次)\n条件：{}\n推荐：{}",
        strategy.frontmatter.spark_type,
        strategy.frontmatter.success_rate * 100.0,
        strategy.frontmatter.use_count,
        "待补充", // TODO: 从 content 提取条件
        strategy.content.lines().next().unwrap_or("")
    )
}

/// 包裹策略内容（围栏保护）
pub fn wrap_strategy_for_prompt(summary: &str) -> String {
    format!(
        "<strategy-context>\n[系统注：以下是历史成功策略参考]\n\n{}\n\n</strategy-context>",
        summary
    )
}

/// 计算候选动作与策略的对齐度
pub fn calculate_alignment(candidate_reasoning: &str, strategy_content: &str) -> f32 {
    // 简单的关键词匹配
    let strategy_keywords = extract_keywords(strategy_content);
    let candidate_keywords = extract_keywords(candidate_reasoning);

    let common = strategy_keywords.iter()
        .filter(|k| candidate_keywords.contains(k))
        .count();

    common as f32 / strategy_keywords.len().max(1) as f32
}

/// 对齐度 boost（+0.1 额外权重）
pub const ALIGNMENT_BOOST: f32 = 0.1;

/// Progressive disclosure Tier
pub enum Tier {
    Tier1 = 50,   // 仅 metadata
    Tier2 = 200,  // 摘要 + 简短说明
    Tier3 = 500,  // 完整内容
}

/// 根据 Tier 获取策略内容（progressive disclosure）
pub fn get_strategy_by_tier(strategy: &StrategyFile, tier: Tier) -> String {
    match tier {
        Tier::Tier1 => {
            // 仅 metadata
            format!(
                "[{}] 成功率 {:.0}%, 使用{}次",
                strategy.frontmatter.spark_type,
                strategy.frontmatter.success_rate * 100.0,
                strategy.frontmatter.use_count
            )
        }
        Tier::Tier2 => {
            // 摘要 + 简短说明
            get_strategy_summary(strategy)
        }
        Tier::Tier3 => {
            // 完整内容
            strategy.content.clone()
        }
    }
}

fn spark_type_name(spark_type: SparkType) -> String {
    match spark_type {
        SparkType::ResourcePressure => "resource_pressure",
        SparkType::SocialPressure => "social_pressure",
        SparkType::CognitivePressure => "cognitive_pressure",
        SparkType::ExpressivePressure => "expressive_pressure",
        SparkType::PowerPressure => "power_pressure",
        SparkType::LegacyPressure => "legacy_pressure",
        SparkType::Explore => "explore",
    }.to_string()
}

fn extract_keywords(text: &str) -> Vec<String> {
    // 简单提取：分割空格，过滤短词
    text.split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|w| w.to_lowercase())
        .collect()
}
