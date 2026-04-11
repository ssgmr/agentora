//! 环境压力系统

use serde::{Deserialize, Serialize};

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