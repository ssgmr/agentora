//! Bridge Provider 创建逻辑单元测试
//!
//! 测试 UserConfig 不同模式下的 Provider 创建逻辑

use agentora_bridge::user_config::UserConfig;

/// 测试 UserConfig 验证 - local 模式需要模型路径
#[test]
fn test_user_config_local_mode_requires_path() {
    let mut config = UserConfig::default();
    config.llm.mode = "local".to_string();
    config.llm.local_model_path = String::new();

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("本地模式需要指定模型路径"));
}

/// 测试 UserConfig 验证 - remote 模式需要 endpoint
#[test]
fn test_user_config_remote_mode_requires_endpoint() {
    let mut config = UserConfig::default();
    config.llm.mode = "remote".to_string();
    config.llm.api_endpoint = String::new();

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("远程模式需要配置 API Endpoint"));
}

/// 测试 UserConfig 验证 - remote 模式有效配置
#[test]
fn test_user_config_remote_mode_valid() {
    let mut config = UserConfig::default();
    config.llm.mode = "remote".to_string();
    config.llm.api_endpoint = "http://localhost:1234/v1".to_string();
    config.agent.name = "智行者".to_string();

    let result = config.validate();
    assert!(result.is_ok());
}

/// 测试 UserConfig 验证 - rule_only 模式无需额外配置
#[test]
fn test_user_config_rule_only_mode() {
    let mut config = UserConfig::default();
    config.llm.mode = "rule_only".to_string();
    config.agent.name = "智行者".to_string();

    let result = config.validate();
    assert!(result.is_ok());
}

/// 测试 UserConfig 验证 - 无效 LLM 模式
#[test]
fn test_user_config_invalid_llm_mode() {
    let mut config = UserConfig::default();
    config.llm.mode = "invalid_mode".to_string();
    config.agent.name = "智行者".to_string();

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("无效的 LLM 模式"));
}

/// 测试 UserConfig 验证 - Agent 名字为空
#[test]
fn test_user_config_empty_agent_name() {
    let mut config = UserConfig::default();
    config.agent.name = String::new();

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Agent 名字不能为空"));
}

/// 测试 UserConfig 默认值
#[test]
fn test_user_config_default() {
    let config = UserConfig::default();

    assert_eq!(config.llm.mode, "rule_only");
    assert_eq!(config.agent.name, "智行者");
    assert_eq!(config.p2p.mode, "single");
}

/// 测试 UserConfig 序列化/反序列化
#[test]
fn test_user_config_serialize_deserialize() {
    let mut config = UserConfig::default();
    config.llm.mode = "remote".to_string();
    config.llm.api_endpoint = "http://localhost:1234/v1".to_string();
    config.agent.name = "测试Agent".to_string();

    // 序列化
    let toml_str = toml::to_string_pretty(&config).unwrap();

    // 反序列化
    let parsed: UserConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(config.llm.mode, parsed.llm.mode);
    assert_eq!(config.agent.name, parsed.agent.name);
}

/// 测试 UserConfig 加载和保存
#[test]
fn test_user_config_save_and_load() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let path = dir.path().join("user_config.toml");

    let mut config = UserConfig::default();
    config.llm.mode = "local".to_string();
    config.llm.local_model_path = "/path/to/model.gguf".to_string();
    config.agent.name = "测试Agent".to_string();

    // 保存
    config.save(&path).unwrap();

    // 加载
    let loaded = UserConfig::load(&path).unwrap();

    assert_eq!(config.llm.mode, loaded.llm.mode);
    assert_eq!(config.llm.local_model_path, loaded.llm.local_model_path);
    assert_eq!(config.agent.name, loaded.agent.name);
}

/// 测试 UserConfig P2P join 模式需要种子地址
#[test]
fn test_user_config_join_mode_requires_seed() {
    let mut config = UserConfig::default();
    config.p2p.mode = "join".to_string();
    config.p2p.seed_address = String::new();
    config.agent.name = "智行者".to_string();

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("加入模式需要输入种子节点地址"));
}

/// 测试 UserConfig P2P join 模式有效配置
#[test]
fn test_user_config_join_mode_valid() {
    let mut config = UserConfig::default();
    config.p2p.mode = "join".to_string();
    config.p2p.seed_address = "/ip4/192.168.1.100/tcp/7000/p2p/12D3Koo...".to_string();
    config.agent.name = "智行者".to_string();

    let result = config.validate();
    assert!(result.is_ok());
}