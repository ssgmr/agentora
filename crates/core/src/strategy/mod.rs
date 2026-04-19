//! 策略库系统：StrategyHub
//!
//! Markdown + YAML frontmatter，支持 create/patch/decay 闭环

pub mod create;
pub mod patch;
pub mod decay;
pub mod retrieve;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

/// 策略库
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StrategyHub {
    strategies: Vec<Strategy>,
    agent_id: String,
    base_dir: PathBuf,
}

/// YAML frontmatter 结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyFrontmatter {
    pub spark_type: String,
    pub success_rate: f32,
    pub use_count: u32,
    pub last_used_tick: u32,
    pub created_tick: u32,
    #[serde(default)]
    pub deprecated: bool,
}

/// 策略定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub spark_type: String,
    pub success_rate: f32,
    pub use_count: u32,
    pub last_used_tick: u32,
    pub created_tick: u32,
    pub deprecated: bool,
    pub content: String,
}

/// 策略文件内容（frontmatter + 正文）
#[derive(Debug, Clone)]
pub struct StrategyFile {
    pub frontmatter: StrategyFrontmatter,
    pub content: String,
}

impl StrategyHub {
    pub fn new(agent_id: &str) -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".agentora")
            .join("agents")
            .join(agent_id)
            .join("strategies");

        Self {
            strategies: Vec::new(),
            agent_id: agent_id.to_string(),
            base_dir,
        }
    }

    /// 获取策略目录路径
    fn strategy_dir(&self, spark_type: &str) -> PathBuf {
        self.base_dir.join(spark_type)
    }

    /// 确保策略目录存在
    pub fn ensure_strategy_dir(&self, spark_type: &str) -> std::io::Result<()> {
        let dir = self.strategy_dir(spark_type);
        fs::create_dir_all(&dir)
    }

    /// 检查策略文件是否存在
    pub fn strategy_exists(&self, spark_type: &str) -> bool {
        let path = self.strategy_dir(spark_type).join("STRATEGY.md");
        path.exists()
    }

    /// 添加策略到内存
    pub fn add(&mut self, strategy: Strategy) {
        self.strategies.push(strategy);
    }

    /// 按 Spark 类型检索策略
    pub fn find_by_spark_type(&self, spark_type: &str) -> Option<&Strategy> {
        self.strategies.iter()
            .filter(|s| !s.deprecated && s.spark_type == spark_type)
            .max_by(|a, b| a.success_rate.partial_cmp(&b.success_rate).unwrap())
    }

    /// 获取所有有效策略的摘要（Tier 1）
    pub fn list_metadata(&self) -> String {
        self.strategies.iter()
            .filter(|s| !s.deprecated)
            .map(|s| format!("[{}] success_rate={:.2}, uses={}", s.spark_type, s.success_rate, s.use_count))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 列出所有有效策略文件
    pub fn list_strategies(&self) -> std::io::Result<Vec<String>> {
        let mut strategies = Vec::new();

        if !self.base_dir.exists() {
            return Ok(strategies);
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let spark_type = entry.file_name().to_string_lossy().to_string();
            let strategy_path = entry.path().join("STRATEGY.md");

            if strategy_path.exists() {
                strategies.push(spark_type);
            }
        }

        Ok(strategies)
    }

    /// 加载策略文件
    pub fn load_strategy(&self, spark_type: &str) -> Option<StrategyFile> {
        let path = self.strategy_dir(spark_type).join("STRATEGY.md");

        if !path.exists() {
            return None;
        }

        let content = fs::read_to_string(&path).ok()?;
        Self::parse_strategy_file(&content)
    }

    /// 解析策略文件（YAML frontmatter + 正文）
    pub fn parse_strategy_file(content: &str) -> Option<StrategyFile> {
        if !content.starts_with("---") {
            return None;
        }

        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return None;
        }

        let yaml_content = parts[1].trim();
        let body = parts[2].trim();

        let frontmatter: StrategyFrontmatter = serde_yaml::from_str(yaml_content).ok()?;

        Some(StrategyFile {
            frontmatter,
            content: body.to_string(),
        })
    }

    /// 保存策略文件（原子写入）
    pub fn save_strategy(&self, strategy: &Strategy) -> std::io::Result<()> {
        let dir = self.strategy_dir(&strategy.spark_type);
        fs::create_dir_all(&dir)?;

        let path = dir.join("STRATEGY.md");
        let temp_path = dir.join(format!(".STRATEGY.md.tmp.{}", std::process::id()));

        let content = self.build_strategy_file(strategy);

        fs::write(&temp_path, &content)?;
        fs::rename(&temp_path, &path)?;

        Ok(())
    }

    /// 构建策略文件内容
    fn build_strategy_file(&self, strategy: &Strategy) -> String {
        let frontmatter = StrategyFrontmatter {
            spark_type: strategy.spark_type.clone(),
            success_rate: strategy.success_rate,
            use_count: strategy.use_count,
            last_used_tick: strategy.last_used_tick,
            created_tick: strategy.created_tick,
            deprecated: strategy.deprecated,
        };

        let yaml = serde_yaml::to_string(&frontmatter).unwrap();

        format!("---\n{}---\n\n{}", yaml, strategy.content)
    }

    /// 删除策略文件
    pub fn delete_strategy(&self, spark_type: &str) -> std::io::Result<()> {
        let path = self.strategy_dir(spark_type).join("STRATEGY.md");
        if path.exists() {
            fs::remove_file(path)?;
        }

        let dir = self.strategy_dir(spark_type);
        let _ = fs::remove_dir(dir);

        Ok(())
    }

    /// 从文件加载所有策略
    pub fn load_all_strategies(&mut self) -> std::io::Result<()> {
        self.strategies.clear();

        if !self.base_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let spark_type = entry.file_name().to_string_lossy().to_string();

            if let Some(strategy_file) = self.load_strategy(&spark_type) {
                let strategy = Strategy {
                    spark_type: strategy_file.frontmatter.spark_type,
                    success_rate: strategy_file.frontmatter.success_rate,
                    use_count: strategy_file.frontmatter.use_count,
                    last_used_tick: strategy_file.frontmatter.last_used_tick,
                    created_tick: strategy_file.frontmatter.created_tick,
                    deprecated: strategy_file.frontmatter.deprecated,
                    content: strategy_file.content,
                };
                self.strategies.push(strategy);
            }
        }

        Ok(())
    }
}

impl Default for StrategyHub {
    fn default() -> Self {
        Self::new("default")
    }
}
