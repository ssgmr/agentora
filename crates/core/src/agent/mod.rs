//! Agent核心实体与交互

pub mod movement;
pub mod inventory;
pub mod trade;
pub mod dialogue;
pub mod combat;
pub mod alliance;

use crate::types::{AgentId, Position, PersonalitySeed, Action, ActionType};
use crate::memory::MemorySystem;
use crate::strategy::StrategyHub;
use std::collections::HashMap;

/// Agent核心实体
#[derive(Debug, Clone)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub position: Position,
    pub health: u32,
    pub max_health: u32,
    pub satiety: u32,       // 饱食度 0-100，初始100
    pub hydration: u32,     // 水分度 0-100，初始100
    pub inventory: HashMap<String, u32>,
    pub memory: MemorySystem,
    pub relations: HashMap<AgentId, Relation>,
    pub strategies: StrategyHub,
    pub personality: PersonalitySeed,
    pub age: u32,
    pub max_age: u32,
    pub is_alive: bool,
    /// 临时偏好（由外部注入，随 tick 衰减）
    pub temp_preferences: Vec<TempPreference>,
    /// 经验值与等级
    pub experience: u32,
    pub level: u32,
    /// 上一次执行的动作类型（用于避免重复选择相同动作）
    pub last_action_type: Option<String>,
    /// 上一次动作执行结果反馈（成功/失败原因，传递给 LLM 感知）
    pub last_action_result: Option<String>,
    /// 上一次移动前的位置（用于检测来回振荡）
    pub last_position: Option<Position>,
}

/// 临时偏好
#[derive(Debug, Clone)]
pub struct TempPreference {
    pub key: String,
    pub boost: f32,
    pub remaining_ticks: u32,
}

/// 社会关系
#[derive(Debug, Clone)]
pub struct Relation {
    pub trust: f32,
    pub relation_type: RelationType,
    pub last_interaction_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    Neutral,
    Ally,
    Enemy,
}

impl Agent {
    pub fn new(id: AgentId, name: String, position: Position) -> Self {
        let id_str = id.as_str().to_string();
        let mut inventory = HashMap::new();
        // 初始配备：3 个食物 + 2 个水，帮助 Agent 度过早期探索阶段
        inventory.insert("food".to_string(), 3);
        inventory.insert("water".to_string(), 2);

        Self {
            id,
            name,
            position,
            health: 100,
            max_health: 100,
            satiety: 100,
            hydration: 100,
            inventory,
            memory: MemorySystem::new(&id_str),
            relations: HashMap::new(),
            strategies: StrategyHub::new(&id_str),
            personality: PersonalitySeed::default(),
            age: 0,
            max_age: 200,
            is_alive: true,
            temp_preferences: Vec::new(),
            experience: 0,
            level: 1,
            last_action_type: None,
            last_action_result: None,
            last_position: None,
        }
    }

    /// 注入临时偏好
    pub fn inject_preference(&mut self, key: &str, boost: f32, duration_ticks: u32) {
        // 检查是否已有同 key 偏好，有则叠加
        if let Some(pref) = self.temp_preferences.iter_mut().find(|p| p.key == key) {
            pref.boost += boost;
            pref.remaining_ticks = pref.remaining_ticks.max(duration_ticks);
        } else {
            self.temp_preferences.push(TempPreference {
                key: key.to_string(),
                boost,
                remaining_ticks: duration_ticks,
            });
        }
    }

    /// 所有临时偏好 tick 衰减
    pub fn tick_preferences(&mut self) {
        for pref in &mut self.temp_preferences {
            pref.remaining_ticks = pref.remaining_ticks.saturating_sub(1);
        }
        // 移除过期的偏好
        self.temp_preferences.retain(|p| p.remaining_ticks > 0);
    }

    /// 增加经验值，返回是否升级
    pub fn add_experience(&mut self, amount: u32) -> bool {
        self.experience += amount;
        // 升级公式：level * 100 XP（LV1→2需要100，LV2→3需要200，...）
        let xp_for_next = self.level * 100;
        if self.experience >= xp_for_next {
            self.experience -= xp_for_next;
            self.level += 1;
            // 升级奖励：HP上限+10，恢复少量HP
            self.max_health += 10;
            self.health = (self.health + 20).min(self.max_health);
            return true;
        }
        false
    }

    /// 增加信任值
    pub fn increase_trust(&mut self, target_id: &AgentId, delta: f32) {
        let trust = self.relations.entry(target_id.clone()).or_insert(Relation {
            trust: 0.0,
            relation_type: RelationType::Neutral,
            last_interaction_tick: 0,
        });
        trust.trust = (trust.trust + delta).clamp(-100.0, 100.0);
        // 根据信任值更新关系类型
        trust.relation_type = if trust.trust > 30.0 {
            RelationType::Ally
        } else if trust.trust < -20.0 {
            RelationType::Enemy
        } else {
            RelationType::Neutral
        };
    }
}
