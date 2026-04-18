//! Agent核心实体与交互

pub mod movement;
pub mod inventory;
pub mod trade;
pub mod dialogue;
pub mod combat;
pub mod alliance;

use crate::motivation::MotivationVector;
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
    pub motivation: MotivationVector,
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
}

/// 临时偏好
#[derive(Debug, Clone)]
pub struct TempPreference {
    pub dimension: usize,
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
            motivation: MotivationVector::new(),
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
        }
    }

    /// 注入临时偏好
    pub fn inject_preference(&mut self, dimension: usize, boost: f32, duration_ticks: u32) {
        // 检查是否已有同维度偏好，有则叠加
        if let Some(pref) = self.temp_preferences.iter_mut().find(|p| p.dimension == dimension) {
            pref.boost += boost;
            pref.remaining_ticks = pref.remaining_ticks.max(duration_ticks);
        } else {
            self.temp_preferences.push(TempPreference {
                dimension,
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

    /// 计算有效动机（基础 + 临时偏好加成 + 生存压力）
    pub fn effective_motivation(&self) -> [f32; 6] {
        let mut base = self.motivation.to_array();
        for pref in &self.temp_preferences {
            if pref.dimension < 6 {
                base[pref.dimension] = (base[pref.dimension] + pref.boost).clamp(0.0, 1.0);
            }
        }

        // 生存压力驱动：satiety/hydration 低时增加生存维度
        // 阈值从 30 提高到 50，让 Agent 更早开始储备资源
        let survival_boost = if self.satiety == 0 || self.hydration == 0 {
            0.5 // 极端饥渴：生存+0.5
        } else if self.satiety <= 50 || self.hydration <= 50 {
            0.3 // 饥饿/口渴：生存+0.3
        } else {
            0.0
        };
        base[0] = (base[0] + survival_boost).clamp(0.0, 1.0);

        base
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