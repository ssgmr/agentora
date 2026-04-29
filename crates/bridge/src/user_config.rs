//! 用户配置模块
//!
//! 用于引导页面配置管理，支持 TOML 序列化。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use thiserror::Error;

/// 用户配置错误
#[derive(Debug, Error)]
pub enum UserConfigError {
    #[error("配置文件读取失败: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("配置文件解析失败: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("配置文件写入失败: {0}")]
    WriteError(#[from] toml::ser::Error),

    #[error("配置路径无效: {0}")]
    InvalidPath(String),
}

/// 用户配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub llm: LlmUserConfig,
    pub agent: AgentUserConfig,
    pub p2p: P2PUserConfig,
}

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUserConfig {
    /// 模式：local / remote / rule_only
    pub mode: String,

    /// 远程 API endpoint（remote 模式）
    #[serde(default)]
    pub api_endpoint: String,

    /// 远程 API token（remote 模式）
    #[serde(default)]
    pub api_token: String,

    /// 远程 API model name（remote 模式）
    #[serde(default)]
    pub model_name: String,

    /// 本地模型路径（local 模式）
    #[serde(default)]
    pub local_model_path: String,
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUserConfig {
    /// Agent 名字
    pub name: String,

    /// 自定义系统提示词
    #[serde(default)]
    pub custom_prompt: String,

    /// 预设图标 ID（fox, wizard, dragon, lion, robot, default）
    #[serde(default = "default_icon_id")]
    pub icon_id: String,

    /// 自定义图标文件路径（用户上传）
    #[serde(default)]
    pub custom_icon_path: String,
}

fn default_icon_id() -> String {
    "default".to_string()
}

/// P2P 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PUserConfig {
    /// 模式：single / create / join
    #[serde(default = "default_p2p_mode")]
    pub mode: String,

    /// 种子节点地址（join 模式）
    #[serde(default)]
    pub seed_address: String,
}

fn default_p2p_mode() -> String {
    "single".to_string()
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            llm: LlmUserConfig {
                mode: "rule_only".to_string(),
                api_endpoint: String::new(),
                api_token: String::new(),
                model_name: String::new(),
                local_model_path: String::new(),
            },
            agent: AgentUserConfig {
                name: "智行者".to_string(),
                custom_prompt: String::new(),
                icon_id: "default".to_string(),
                custom_icon_path: String::new(),
            },
            p2p: P2PUserConfig {
                mode: "single".to_string(),
                seed_address: String::new(),
            },
        }
    }
}

impl UserConfig {
    /// 配置文件默认路径
    pub const CONFIG_FILENAME: &'static str = "user_config.toml";

    /// 从 TOML 文件加载配置
    pub fn load(path: &Path) -> Result<Self, UserConfigError> {
        if !path.exists() {
            return Err(UserConfigError::InvalidPath(
                format!("配置文件不存在: {}", path.display())
            ));
        }

        let content = fs::read_to_string(path)?;
        let config: UserConfig = toml::from_str(&content)?;

        Ok(config)
    }

    /// 保存配置到 TOML 文件
    pub fn save(&self, path: &Path) -> Result<(), UserConfigError> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| UserConfigError::WriteError(e))?;

        fs::write(path, content)?;

        Ok(())
    }

    /// 获取配置文件路径
    ///
    /// 在 Godot 环境中，配置文件位于:
    /// - Desktop: `config/user_config.toml`
    /// - Mobile: `user://user_config.toml`（Godot user 目录）
    pub fn get_config_path(base_dir: &Path) -> PathBuf {
        base_dir.join(Self::CONFIG_FILENAME)
    }

    /// 检查配置文件是否存在
    pub fn exists(base_dir: &Path) -> bool {
        Self::get_config_path(base_dir).exists()
    }

    /// 验证配置是否有效
    pub fn validate(&self) -> Result<(), String> {
        // Agent 名字不能为空
        if self.agent.name.trim().is_empty() {
            return Err("Agent 名字不能为空".to_string());
        }

        // LLM 模式验证
        let valid_modes = ["local", "remote", "rule_only"];
        if !valid_modes.contains(&self.llm.mode.as_str()) {
            return Err(format!("无效的 LLM 模式: {}", self.llm.mode));
        }

        // P2P 模式验证
        let valid_p2p_modes = ["single", "create", "join"];
        if !valid_p2p_modes.contains(&self.p2p.mode.as_str()) {
            return Err(format!("无效的 P2P 模式: {}", self.p2p.mode));
        }

        // remote 模式需要 endpoint
        if self.llm.mode == "remote" && self.llm.api_endpoint.trim().is_empty() {
            return Err("远程模式需要配置 API Endpoint".to_string());
        }

        // local 模式需要模型路径
        if self.llm.mode == "local" && self.llm.local_model_path.trim().is_empty() {
            return Err("本地模式需要指定模型路径".to_string());
        }

        // join 模式需要种子地址
        if self.p2p.mode == "join" && self.p2p.seed_address.trim().is_empty() {
            return Err("加入模式需要输入种子节点地址".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = UserConfig::default();
        assert_eq!(config.llm.mode, "rule_only");
        assert_eq!(config.agent.name, "智行者");
        assert_eq!(config.p2p.mode, "single");
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = UserConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: UserConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.agent.name, parsed.agent.name);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("user_config.toml");

        let config = UserConfig::default();
        config.save(&path).unwrap();

        let loaded = UserConfig::load(&path).unwrap();
        assert_eq!(config.agent.name, loaded.agent.name);
    }

    #[test]
    fn test_validate_empty_name() {
        let mut config = UserConfig::default();
        config.agent.name = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_llm_mode() {
        let mut config = UserConfig::default();
        config.llm.mode = "invalid".to_string();
        assert!(config.validate().is_err());
    }
}