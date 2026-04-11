//! WorldSeed配置解析

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// WorldSeed.toml配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSeed {
    /// 地图大小 [width, height]
    pub map_size: [u32; 2],

    /// 地形分布比例
    pub terrain_ratio: HashMap<String, f32>,

    /// 资源密度 (0.0-1.0)
    pub resource_density: f32,

    /// 区域大小 (每个区域的格子数)
    pub region_size: u32,

    /// 初始Agent数量
    pub initial_agents: u32,

    /// 动机模板
    pub motivation_templates: HashMap<String, [f32; 6]>,

    /// 生成位置策略: random/clustered/scattered
    pub spawn_strategy: String,

    /// P2P种子节点地址
    pub seed_peers: Vec<String>,

    /// 压力池配置
    pub pressure_config: PressureConfig,
}

impl Default for WorldSeed {
    fn default() -> Self {
        Self {
            map_size: [256, 256],
            terrain_ratio: HashMap::from([
                ("plains".to_string(), 0.5),
                ("forest".to_string(), 0.25),
                ("mountain".to_string(), 0.1),
                ("water".to_string(), 0.1),
                ("desert".to_string(), 0.05),
            ]),
            resource_density: 0.15,
            region_size: 16,
            initial_agents: 5,
            motivation_templates: HashMap::from([
                ("gatherer".to_string(), [0.8, 0.4, 0.3, 0.2, 0.3, 0.2]),
                ("trader".to_string(), [0.5, 0.8, 0.4, 0.3, 0.7, 0.3]),
                ("explorer".to_string(), [0.4, 0.3, 0.9, 0.6, 0.3, 0.4]),
                ("builder".to_string(), [0.6, 0.5, 0.4, 0.8, 0.4, 0.3]),
            ]),
            spawn_strategy: "scattered".to_string(),
            seed_peers: vec![],
            pressure_config: PressureConfig::default(),
        }
    }
}

impl WorldSeed {
    /// 从文件加载配置
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let seed: WorldSeed = toml::from_str(&content)?;
        Ok(seed)
    }

    /// 保存配置到文件
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// 压力池配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureConfig {
    /// 压力事件触发间隔 (tick)
    pub trigger_interval_range: [u32; 2],

    /// 资源波动幅度 (0.0-1.0)
    pub resource_fluctuation: f32,

    /// 气候事件概率
    pub climate_event_probability: f32,

    /// 区域封锁持续时间 (tick)
    pub blockade_duration_range: [u32; 2],
}

impl Default for PressureConfig {
    fn default() -> Self {
        Self {
            trigger_interval_range: [20, 50],
            resource_fluctuation: 0.3,
            climate_event_probability: 0.1,
            blockade_duration_range: [10, 30],
        }
    }
}