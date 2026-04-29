//! WorldSeed配置解析

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::types::{PersonalityTemplate, PersonalitySeed};

/// 玩家 Agent 配置（引导页面注入）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerAgentConfig {
    /// Agent 名字
    pub name: String,

    /// 自定义系统提示词
    #[serde(default)]
    pub custom_prompt: String,

    /// 预设图标 ID
    #[serde(default)]
    pub icon_id: String,

    /// 自定义图标文件路径
    #[serde(default)]
    pub custom_icon_path: String,
}

impl PlayerAgentConfig {
    /// 转换为 PersonalitySeed 扩展字段
    pub fn to_personality_extensions(&self) -> (Option<String>, Option<String>, Option<String>) {
        let custom_prompt = if self.custom_prompt.is_empty() {
            None
        } else {
            Some(self.custom_prompt.clone())
        };

        let icon_id = if self.icon_id.is_empty() || self.icon_id == "default" {
            None
        } else {
            Some(self.icon_id.clone())
        };

        let custom_icon_path = if self.custom_icon_path.is_empty() {
            None
        } else {
            Some(self.custom_icon_path.clone())
        };

        (custom_prompt, icon_id, custom_icon_path)
    }
}

/// WorldSeed.toml配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSeed {
    /// 地图大小 [width, height]
    pub map_size: [u32; 2],

    /// 地形分布比例
    pub terrain_ratio: BTreeMap<String, f32>,

    /// 资源密度 (0.0-1.0)
    pub resource_density: f32,

    /// 区域大小 (每个区域的格子数)
    pub region_size: u32,

    /// 初始Agent数量
    pub initial_agents: u32,

    /// 生成位置策略: random/clustered/scattered
    pub spawn_strategy: String,

    /// P2P种子节点地址
    pub seed_peers: Vec<String>,

    /// 随机种子（用于确定性世界生成）
    /// 如果设置为固定值，所有客户端生成相同世界
    /// 设置为 0 或不设置则使用时间戳（每次生成不同世界）
    #[serde(default)]
    pub random_seed: u64,

    /// 压力池配置
    pub pressure_config: PressureConfig,

    /// Agent性格配置（任务 2.3）
    #[serde(default)]
    pub agent_personalities: AgentPersonalities,

    /// Agent名字前缀（用于P2P模式下区分不同节点的Agent）
    /// 例如："N4001_" 会让 Agent 名字变为 "N4001_Agent"
    #[serde(default)]
    pub agent_name_prefix: String,

    /// P2P 模式下跳过初始 Agent 生成（Agent 将在 Simulation.start() 中动态创建）
    #[serde(default)]
    pub skip_initial_agents: bool,

    /// 玩家 Agent 配置（引导页面注入）
    #[serde(default)]
    pub player_agent_config: Option<PlayerAgentConfig>,
}

impl Default for WorldSeed {
    fn default() -> Self {
        Self {
            map_size: [256, 256],
            terrain_ratio: BTreeMap::from([
                ("plains".to_string(), 0.5),
                ("forest".to_string(), 0.25),
                ("mountain".to_string(), 0.1),
                ("water".to_string(), 0.1),
                ("desert".to_string(), 0.05),
            ]),
            resource_density: 0.02,
            region_size: 16,
            initial_agents: 5,
            spawn_strategy: "scattered".to_string(),
            seed_peers: vec![],
            random_seed: 42, // 默认使用固定种子，确保 P2P 模式下世界一致
            pressure_config: PressureConfig::default(),
            agent_personalities: AgentPersonalities::default(),
            agent_name_prefix: String::new(), // 默认无前缀
            skip_initial_agents: false, // 默认生成初始 Agent
            player_agent_config: None, // 默认无玩家配置
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

    /// 合并用户配置（引导页面注入）
    ///
    /// 将 UserConfig 的 agent 和 p2p 配置合并到 WorldSeed
    pub fn merge_user_config(
        &mut self,
        agent_name: String,
        custom_prompt: String,
        icon_id: String,
        custom_icon_path: String,
        p2p_mode: String,
        seed_address: String,
    ) {
        // 合并 Agent 配置
        self.player_agent_config = Some(PlayerAgentConfig {
            name: agent_name,
            custom_prompt,
            icon_id,
            custom_icon_path,
        });

        // 合并 P2P 配置
        if p2p_mode == "join" && !seed_address.is_empty() {
            // 加入模式：添加种子节点地址
            if !self.seed_peers.contains(&seed_address) {
                self.seed_peers.push(seed_address);
            }
        }
    }

    /// 应用玩家配置到 PersonalitySeed
    pub fn apply_player_config(&self, personality: &mut PersonalitySeed, agent_name: &mut String) {
        if let Some(config) = &self.player_agent_config {
            // 应用自定义提示词
            if !config.custom_prompt.is_empty() {
                personality.custom_prompt = Some(config.custom_prompt.clone());
            }

            // 应用图标
            if !config.icon_id.is_empty() && config.icon_id != "default" {
                personality.icon_id = Some(config.icon_id.clone());
            }

            if !config.custom_icon_path.is_empty() {
                personality.custom_icon_path = Some(config.custom_icon_path.clone());
            }

            // 应用名字
            if !config.name.is_empty() {
                *agent_name = config.name.clone();
            }
        }
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

/// Agent性格配置（任务 2.3）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonalities {
    /// 性格模板字典
    #[serde(default)]
    pub templates: BTreeMap<String, PersonalityTemplate>,

    /// 分配方式：random/default/指定模板名
    #[serde(default = "default_assignment")]
    pub assignment: String,

    /// 默认性格（未配置或assignment=default时使用）
    #[serde(default)]
    pub default: PersonalityTemplate,
}

fn default_assignment() -> String {
    "random".to_string()
}

impl Default for AgentPersonalities {
    fn default() -> Self {
        let mut templates = BTreeMap::new();

        // 四种预设性格模板
        templates.insert("explorer".to_string(), PersonalityTemplate {
            openness: 0.8,
            agreeableness: 0.3,
            neuroticism: 0.4,
            description: "一个好奇的探索者，喜欢发现新事物，倾向于独自行动".to_string(),
        });

        templates.insert("socializer".to_string(), PersonalityTemplate {
            openness: 0.6,
            agreeableness: 0.8,
            neuroticism: 0.3,
            description: "一个友善的交际者，喜欢与其他Agent交流，乐于合作".to_string(),
        });

        templates.insert("survivor".to_string(), PersonalityTemplate {
            openness: 0.3,
            agreeableness: 0.4,
            neuroticism: 0.7,
            description: "一个谨慎的生存者，注重自身安全，会优先储备资源".to_string(),
        });

        templates.insert("builder".to_string(), PersonalityTemplate {
            openness: 0.5,
            agreeableness: 0.6,
            neuroticism: 0.3,
            description: "一个创造者，喜欢建造建筑和留下遗产".to_string(),
        });

        Self {
            templates,
            assignment: "random".to_string(),
            default: PersonalityTemplate::default(),
        }
    }
}

impl AgentPersonalities {
    /// 根据assignment方式选择性格模板
    pub fn select_template(&self) -> &PersonalityTemplate {
        use rand::Rng;

        match self.assignment.as_str() {
            "default" => &self.default,
            // 如果assignment是特定模板名，尝试找到它
            name if self.templates.contains_key(name) => {
                self.templates.get(name).unwrap()
            },
            // random 或未知值：随机选择
            _ => {
                if self.templates.is_empty() {
                    return &self.default;
                }
                let mut rng = rand::thread_rng();
                let keys: Vec<&String> = self.templates.keys().collect();
                let idx = rng.gen_range(0..keys.len());
                self.templates.get(keys[idx]).unwrap()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default_seed() {
        let seed = WorldSeed::load("../../worldseeds/default.toml").expect("加载世界种子失败");
        assert_eq!(seed.initial_agents, 5);
        assert_eq!(seed.spawn_strategy, "scattered");
    }
}