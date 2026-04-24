//! 环境压力系统

use serde::{Deserialize, Serialize};
use crate::world::World;
use crate::types::{AgentId, ResourceType};
use crate::snapshot::{NarrativeEvent, NarrativeChannel, AgentSource};

/// 压力事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureType {
    ResourceFluctuation,  // 资源产出波动
    ClimateEvent,         // 气候事件
    RegionBlockade,       // 区域封锁
}

/// 压力事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureEvent {
    pub id: String,
    pub pressure_type: PressureType,
    pub affected_region: Option<u32>,
    pub affected_resource: Option<String>,
    pub intensity: f32,
    pub duration_ticks: u32,
    pub remaining_ticks: u32,
    pub description: String,
    pub created_tick: u64,
}

impl PressureEvent {
    /// 生成压力事件
    pub fn generate(pressure_type: PressureType, tick: u64) -> Self {
        let (description, intensity, duration) = match pressure_type {
            PressureType::ResourceFluctuation => (
                "资源产出波动".to_string(),
                0.3,
                30,
            ),
            PressureType::ClimateEvent => (
                "气候异常".to_string(),
                0.5,
                20,
            ),
            PressureType::RegionBlockade => (
                "区域封锁".to_string(),
                1.0,
                15,
            ),
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pressure_type,
            affected_region: None,
            affected_resource: None,
            intensity,
            duration_ticks: duration,
            remaining_ticks: duration,
            description,
            created_tick: tick,
        }
    }

    /// 推进压力事件
    pub fn advance(&mut self) {
        self.remaining_ticks = self.remaining_ticks.saturating_sub(1);
    }

    /// 检查是否结束
    pub fn is_finished(&self) -> bool {
        self.remaining_ticks == 0
    }
}

impl World {
    /// 环境压力 tick
    pub fn pressure_tick(&mut self) {
        // 生成新压力事件
        if self.tick >= self.next_pressure_tick && self.pressure_pool.len() < 3 {
            use rand::Rng;
            let mut rng = rand::thread_rng();

            // 从三种压力事件中随机选择
            let event_variants = ["drought", "abundance", "plague"];
            let event_type = event_variants[rng.gen_range(0..event_variants.len())];

            let (description, duration) = match event_type {
                "drought" => ("干旱来袭，水源产出减半".to_string(), 30),
                "abundance" => ("丰饶时节，食物产出翻倍".to_string(), 20),
                "plague" => ("瘟疫蔓延，生命受到威胁".to_string(), 1),
                _ => unreachable!(),
            };

            let event = PressureEvent {
                id: uuid::Uuid::new_v4().to_string(),
                pressure_type: match event_type {
                    "drought" => PressureType::ResourceFluctuation,
                    "abundance" => PressureType::ResourceFluctuation,
                    "plague" => PressureType::ClimateEvent,
                    _ => unreachable!(),
                },
                affected_resource: Some(match event_type {
                    "drought" => "Water".to_string(),
                    "abundance" => "Food".to_string(),
                    _ => String::new(),
                }),
                description: description.clone(),
                duration_ticks: duration,
                remaining_ticks: duration,
                intensity: match event_type {
                    "drought" => 0.5,
                    "abundance" => 2.0,
                    "plague" => 1.0,
                    _ => 1.0,
                },
                affected_region: None,
                created_tick: self.tick,
            };

            // 应用立即效果
            match event_type {
                "drought" => {
                    self.pressure_multiplier.insert("water".to_string(), 0.5);
                }
                "abundance" => {
                    // 食物节点数量翻倍
                    for node in self.resources.values_mut() {
                        if node.resource_type == ResourceType::Food {
                            node.current_amount = (node.current_amount * 2).min(node.max_amount);
                        }
                    }
                }
                "plague" => {
                    // 随机 1-3 个 Agent HP -20
                    let mut alive_agents: Vec<AgentId> = self.agents.iter()
                        .filter(|(_, a)| a.is_alive)
                        .map(|(id, _)| id.clone())
                        .collect();
                    let plague_count = rng.gen_range(1..=3).min(alive_agents.len());
                    // 简单随机选择
                    for _ in 0..plague_count {
                        if alive_agents.is_empty() { break; }
                        let idx = rng.gen_range(0..alive_agents.len());
                        let target_id = alive_agents.remove(idx);
                        if let Some(agent) = self.agents.get_mut(&target_id) {
                            agent.health = agent.health.saturating_sub(20);
                        }
                    }
                }
                _ => {}
            }

            tracing::info!("压力事件生成: {} (持续{}tick)", description, duration);
            // 添加叙事事件
            self.tick_events.push(NarrativeEvent {
                tick: self.tick,
                agent_id: "system".to_string(),
                agent_name: "世界".to_string(),
                event_type: "pressure_start".to_string(),
                description: format!("⚠️ {}", description),
                color_code: "#FF9800".to_string(),
                channel: NarrativeChannel::World, // 压力事件是世界频道
                agent_source: AgentSource::Local,
            });
            self.pressure_pool.push(event);
            self.next_pressure_tick = self.tick + rng.gen_range(40..80);
        } else if self.tick >= self.next_pressure_tick {
            // 已达上限，推迟
            self.next_pressure_tick = self.tick + 20;
        }

        // 推进现有事件
        for pressure in &mut self.pressure_pool.iter_mut() {
            pressure.advance();
        }

        // 移除过期事件并恢复效果
        let expired: Vec<PressureEvent> = self.pressure_pool.drain(..)
            .filter(|p| p.is_finished())
            .collect();
        for event in &expired {
            // 恢复持续效果
            if let Some(ref resource) = event.affected_resource {
                match resource.as_str() {
                    "Water" | "water" => {
                        self.pressure_multiplier.remove("water");
                    }
                    _ => {}
                }
            }
            tracing::info!("压力事件结束: {}", event.description);
            // 添加叙事事件
            self.tick_events.push(NarrativeEvent {
                tick: self.tick,
                agent_id: "system".to_string(),
                agent_name: "世界".to_string(),
                event_type: "pressure_end".to_string(),
                description: format!("✓ {} 已结束", event.description),
                color_code: "#8BC34A".to_string(),
                channel: NarrativeChannel::World, // 压力事件是世界频道
                agent_source: AgentSource::Local,
            });
        }
    }
}