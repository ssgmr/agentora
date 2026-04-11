//! LLM 配置加载器
//!
//! 从 config/llm.toml 加载 Provider 配置

use serde::Deserialize;
use std::path::Path;

/// OpenAI Provider 配置
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiConfig {
    /// API 基础 URL
    pub api_base: String,
    /// API Key
    pub api_key: String,
    /// 模型名称
    pub model: String,
    /// 超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Anthropic Provider 配置
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicConfig {
    /// API Key
    pub api_key: String,
    /// 模型名称
    pub model: String,
    /// 超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// 本地 GGUF Provider 配置
#[derive(Debug, Clone, Deserialize)]
pub struct LocalConfig {
    /// 模型文件路径
    pub model_path: String,
    /// 后端类型：cpu/metal/cuda
    #[serde(default = "default_backend")]
    pub backend: String,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// LLM 配置结构体
#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    /// OpenAI Provider 配置（主）
    #[serde(default)]
    pub primary: OpenAiConfig,
    /// Anthropic Provider 配置（备用）
    #[serde(default, alias = "anthropic_compat")]
    pub anthropic: AnthropicConfig,
    /// 本地 GGUF Provider 配置
    #[serde(default)]
    pub local: LocalConfig,
    /// 决策配置
    #[serde(default)]
    pub decision: DecisionConfig,
    /// 记忆系统配置
    #[serde(default, alias = "memory")]
    pub memory: MemoryConfig,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            api_base: "http://localhost:1234".to_string(),
            api_key: String::new(),
            model: "qwen3.5-2b".to_string(),
            timeout_seconds: default_timeout(),
            enabled: true,
        }
    }
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "claude-sonnet-4-6-20250929".to_string(),
            timeout_seconds: default_timeout(),
            enabled: false,
        }
    }
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            model_path: String::new(),
            backend: default_backend(),
            enabled: false,
        }
    }
}

/// 决策配置
#[derive(Debug, Clone, Deserialize)]
pub struct DecisionConfig {
    /// 最大生成 tokens
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// 温度
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Prompt 最大 tokens
    #[serde(default = "default_prompt_max_tokens_u32")]
    pub prompt_max_tokens: u32,
}

/// 记忆系统配置
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryConfig {
    // 预算层
    /// 记忆总预算（字符数）
    #[serde(default = "default_total_budget")]
    pub total_budget: usize,
    /// 编年史快照预算（字符数）
    #[serde(default = "default_chronicle_budget")]
    pub chronicle_budget: usize,
    /// 数据库检索预算（字符数）
    #[serde(default = "default_db_budget")]
    pub db_budget: usize,
    /// 策略参考预算（字符数）
    #[serde(default = "default_strategy_budget")]
    pub strategy_budget: usize,
    // 存储层
    /// 单编年史文件内容上限（字符数）
    #[serde(default = "default_chronicle_limit")]
    pub chronicle_limit: usize,
    /// 世界认知文件上限（字符数）
    #[serde(default = "default_world_seed_limit")]
    pub world_seed_limit: usize,
    // 检索层
    /// 重要性过滤阈值
    #[serde(default = "default_importance_threshold")]
    pub importance_threshold: f32,
    /// 检索返回最大条数
    #[serde(default = "default_search_limit")]
    pub search_limit: usize,
    /// 单个记忆片段最大字符数
    #[serde(default = "default_snippet_max_chars")]
    pub snippet_max_chars: usize,
    // 容量层
    /// 短期记忆最多条数
    #[serde(default = "default_short_term_capacity")]
    pub short_term_capacity: usize,
    // Prompt约束
    /// 决策Prompt总上限（字符数）
    #[serde(default = "default_prompt_max_tokens_usize")]
    pub prompt_max_tokens: usize,
}

impl MemoryConfig {
    /// 校验配置合法性
    pub fn validate(&self) -> Result<(), String> {
        // 预算值必须 > 0
        if self.total_budget == 0 || self.chronicle_budget == 0
            || self.db_budget == 0 || self.strategy_budget == 0
        {
            return Err("预算值必须大于 0".to_string());
        }

        // 重要性阈值必须在 (0.0, 1.0]
        if self.importance_threshold <= 0.0 || self.importance_threshold > 1.0 {
            return Err("重要性阈值必须在 (0.0, 1.0] 范围内".to_string());
        }

        // 容量值必须 > 0
        if self.search_limit == 0 || self.short_term_capacity == 0 {
            return Err("容量值必须大于 0".to_string());
        }

        // 子预算之和 <= 总预算
        let sub_sum = self.chronicle_budget + self.db_budget + self.strategy_budget;
        if sub_sum > self.total_budget {
            return Err(format!("子预算之和({sub_sum})超过总预算({})", self.total_budget));
        }

        // 总预算 <= Prompt 上限
        if self.total_budget > self.prompt_max_tokens {
            return Err(format!("记忆总预算({})超过Prompt上限({})", self.total_budget, self.prompt_max_tokens));
        }

        Ok(())
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            total_budget: default_total_budget(),
            chronicle_budget: default_chronicle_budget(),
            db_budget: default_db_budget(),
            strategy_budget: default_strategy_budget(),
            chronicle_limit: default_chronicle_limit(),
            world_seed_limit: default_world_seed_limit(),
            importance_threshold: default_importance_threshold(),
            search_limit: default_search_limit(),
            snippet_max_chars: default_snippet_max_chars(),
            short_term_capacity: default_short_term_capacity(),
            prompt_max_tokens: default_prompt_max_tokens_usize(),
        }
    }
}

// --- 默认值函数（等于当前硬编码常量） ---
fn default_total_budget() -> usize { 1800 }
fn default_chronicle_budget() -> usize { 800 }
fn default_db_budget() -> usize { 600 }
fn default_strategy_budget() -> usize { 400 }
fn default_chronicle_limit() -> usize { 1800 }
fn default_world_seed_limit() -> usize { 500 }
fn default_importance_threshold() -> f32 { 0.5 }
fn default_search_limit() -> usize { 5 }
fn default_snippet_max_chars() -> usize { 200 }
fn default_short_term_capacity() -> usize { 5 }

fn default_timeout() -> u32 { 300 }
fn default_true() -> bool { true }
fn default_backend() -> String { "cpu".to_string() }
fn default_max_tokens() -> u32 { 500 }
fn default_temperature() -> f32 { 0.7 }
fn default_prompt_max_tokens_u32() -> u32 { 2500 }
fn default_prompt_max_tokens_usize() -> usize { 2500 }

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            prompt_max_tokens: default_prompt_max_tokens_u32(),
        }
    }
}

/// 加载 LLM 配置
///
/// 从 config/llm.toml 加载配置，文件不存在时返回默认配置
pub fn load_llm_config<P: AsRef<Path>>(config_path: P) -> Result<LlmConfig, Box<dyn std::error::Error>> {
    let path = config_path.as_ref();

    if !path.exists() {
        // 返回默认配置
        return Ok(LlmConfig::default());
    }

    // 读取文件内容
    let content = std::fs::read_to_string(path)?;
    let mut config: LlmConfig = toml::from_str(&content)?;

    // 环境变量覆盖
    if let Ok(api_key) = std::env::var("LLM_API_KEY") {
        config.primary.api_key = api_key;
    }
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        config.anthropic.api_key = api_key;
    }

    Ok(config)
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            primary: OpenAiConfig::default(),
            anthropic: AnthropicConfig::default(),
            local: LocalConfig::default(),
            decision: DecisionConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();
        assert_eq!(config.total_budget, 1800);
        assert_eq!(config.chronicle_budget, 800);
        assert_eq!(config.db_budget, 600);
        assert_eq!(config.strategy_budget, 400);
        assert_eq!(config.chronicle_limit, 1800);
        assert_eq!(config.world_seed_limit, 500);
        assert_eq!(config.importance_threshold, 0.5);
        assert_eq!(config.search_limit, 5);
        assert_eq!(config.snippet_max_chars, 200);
        assert_eq!(config.short_term_capacity, 5);
        assert_eq!(config.prompt_max_tokens, 2500);
    }

    #[test]
    fn test_default_config() {
        let config = load_llm_config("nonexistent.toml").unwrap();
        assert!(config.primary.enabled);
        assert!(!config.anthropic.enabled);
        assert!(!config.local.enabled);
        assert_eq!(config.decision.max_tokens, 500);
        assert_eq!(config.decision.temperature, 0.7);
        assert_eq!(config.memory.total_budget, 1800);
        assert_eq!(config.memory.chronicle_budget, 800);
        assert_eq!(config.memory.db_budget, 600);
        assert_eq!(config.memory.strategy_budget, 400);
        assert_eq!(config.memory.prompt_max_tokens, 2500);
    }

    #[test]
    fn test_memory_config_validation() {
        let mut config = MemoryConfig::default();
        assert!(config.validate().is_ok());

        // 子预算超限
        config.chronicle_budget = 1000;
        config.db_budget = 800;
        config.strategy_budget = 500;
        assert!(config.validate().is_err());

        // 总预算超 Prompt 上限
        let mut config2 = MemoryConfig::default();
        config2.total_budget = 3000;
        config2.prompt_max_tokens = 2500;
        assert!(config2.validate().is_err());

        // 阈值越界
        let mut config3 = MemoryConfig::default();
        config3.importance_threshold = 0.0;
        assert!(config3.validate().is_err());

        let mut config4 = MemoryConfig::default();
        config4.importance_threshold = 1.5;
        assert!(config4.validate().is_err());

        // 零值
        let mut config5 = MemoryConfig::default();
        config5.total_budget = 0;
        assert!(config5.validate().is_err());
    }
}
