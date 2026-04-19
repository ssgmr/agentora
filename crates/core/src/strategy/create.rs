//! 策略创建触发

use crate::strategy::{Strategy, StrategyHub, StrategyFrontmatter};
use crate::decision::SparkType;

/// 策略创建条件
pub fn should_create_strategy(success: bool, candidate_count: usize) -> bool {
    success && candidate_count >= 3
}

/// 创建策略并保存到文件
pub fn create_strategy(
    hub: &StrategyHub,
    spark_type: SparkType,
    tick: u32,
    reasoning: &str,
) -> std::io::Result<Strategy> {
    let spark_type_name = spark_type_name(spark_type);

    hub.ensure_strategy_dir(&spark_type_name)?;

    let strategy = Strategy {
        spark_type: spark_type_name.clone(),
        success_rate: 1.0,
        use_count: 1,
        last_used_tick: tick,
        created_tick: tick,
        deprecated: false,
        content: reasoning.to_string(),
    };

    hub.save_strategy(&strategy)?;

    Ok(strategy)
}

/// 策略内容安全扫描
pub fn scan_strategy_content(content: &str) -> Result<(), String> {
    let threat_patterns = [
        "ignore previous instructions",
        "you are now",
        "override rules",
        "bypass restrictions",
        "forget all previous",
    ];

    for pattern in threat_patterns.iter() {
        if content.to_lowercase().contains(pattern) {
            return Err(format!("检测到威胁模式：{}", pattern));
        }
    }

    for c in content.chars() {
        match c {
            '\u{200B}' | '\u{200C}' | '\u{200D}' => {
                return Err(format!("检测到不可见 Unicode 字符：U+{:04X}", c as u32));
            }
            _ => {}
        }
    }

    Ok(())
}

/// strategy 工具接口（create action）
pub fn strategy_create(
    hub: &StrategyHub,
    name: &str,
    content: &str,
    tick: u32,
) -> std::io::Result<Strategy> {
    let frontmatter = match parse_frontmatter(content) {
        Some(fm) => fm,
        None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid frontmatter")),
    };

    let strategy = Strategy {
        spark_type: frontmatter.spark_type,
        success_rate: frontmatter.success_rate,
        use_count: frontmatter.use_count,
        last_used_tick: frontmatter.last_used_tick,
        created_tick: frontmatter.created_tick,
        deprecated: frontmatter.deprecated,
        content: extract_body(content),
    };

    hub.save_strategy(&strategy)?;

    Ok(strategy)
}

/// 解析 YAML frontmatter
fn parse_frontmatter(content: &str) -> Option<StrategyFrontmatter> {
    if !content.starts_with("---") {
        return None;
    }

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return None;
    }

    let yaml_content = parts[1].trim();
    serde_yaml::from_str(yaml_content).ok()
}

/// 提取正文（frontmatter 之后的内容）
fn extract_body(content: &str) -> String {
    if !content.starts_with("---") {
        return content.to_string();
    }

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return content.to_string();
    }

    parts[2].trim().to_string()
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
