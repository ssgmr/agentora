//! 配置结构单元测试

use agentora_network::{DcutrConfig, AutonatConfig, HybridStrategyConfig};

#[test]
fn test_dcutr_config_default() {
    let config = DcutrConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout_secs, 10);
    assert_eq!(config.concurrent_attempts, 2);
}

#[test]
fn test_autonat_config_default() {
    let config = AutonatConfig::default();
    assert_eq!(config.only_global_ips, false);
    assert_eq!(config.probe_interval_secs, 30);
    assert_eq!(config.probe_timeout_secs, 15);
}

#[test]
fn test_hybrid_strategy_config_default() {
    let config = HybridStrategyConfig::default();
    assert_eq!(config.direct_timeout_secs, 5);
    assert_eq!(config.dcutr_timeout_secs, 15);
    assert_eq!(config.degradation_threshold, 2);
    assert_eq!(config.enable_dcutr, true);
    assert_eq!(config.enable_autonat, true);

    // 验证子配置
    assert_eq!(config.dcutr.max_retries, 3);
    assert_eq!(config.autonat.probe_interval_secs, 30);
}

#[test]
fn test_hybrid_strategy_config_custom() {
    let config = HybridStrategyConfig {
        direct_timeout_secs: 3,
        dcutr_timeout_secs: 20,
        degradation_threshold: 3,
        dcutr: DcutrConfig {
            max_retries: 5,
            timeout_secs: 15,
            concurrent_attempts: 3,
        },
        autonat: AutonatConfig {
            only_global_ips: true,
            probe_interval_secs: 60,
            probe_timeout_secs: 30,
        },
        enable_dcutr: false,
        enable_autonat: false,
    };

    assert_eq!(config.direct_timeout_secs, 3);
    assert_eq!(config.dcutr_timeout_secs, 20);
    assert_eq!(config.degradation_threshold, 3);
    assert_eq!(config.dcutr.max_retries, 5);
    assert_eq!(config.autonat.only_global_ips, true);
    assert_eq!(config.enable_dcutr, false);
    assert_eq!(config.enable_autonat, false);
}

#[test]
fn test_disable_dcutr_only() {
    let mut config = HybridStrategyConfig::default();
    config.enable_dcutr = false;

    assert!(!config.enable_dcutr);
    assert!(config.enable_autonat);
}

#[test]
fn test_disable_autonat_only() {
    let mut config = HybridStrategyConfig::default();
    config.enable_autonat = false;

    assert!(config.enable_dcutr);
    assert!(!config.enable_autonat);
}
