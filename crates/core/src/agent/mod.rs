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
    pub inventory: HashMap<String, u32>,
    pub memory: MemorySystem,
    pub relations: HashMap<AgentId, Relation>,
    pub strategies: StrategyHub,
    pub personality: PersonalitySeed,
    pub age: u32,
    pub max_age: u32,
    pub is_alive: bool,
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
        Self {
            id,
            name,
            position,
            motivation: MotivationVector::new(),
            health: 100,
            max_health: 100,
            inventory: HashMap::new(),
            memory: MemorySystem::new(&id_str),
            relations: HashMap::new(),
            strategies: StrategyHub::new(&id_str),
            personality: PersonalitySeed::default(),
            age: 0,
            max_age: 200,
            is_alive: true,
        }
    }
}