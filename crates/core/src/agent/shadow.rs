//! 影子 Agent（P2P 模式）
//!
//! 用于表示远程 peer 管理的 Agent，只保留渲染和基础交互所需的最少字段。

use serde::{Deserialize, Serialize};
use crate::simulation::AgentDelta;

/// 影子 Agent：精简字段，仅用于 P2P 远程 Agent 同步
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowAgent {
    pub id: String,
    pub name: String,
    pub position: (u32, u32),
    pub health: u32,
    pub max_health: u32,
    pub is_alive: bool,
    pub age: u32,
    /// 最后一次收到更新的 tick
    pub last_seen_tick: u64,
    /// 来源 peer ID
    pub source_peer_id: String,
}

impl ShadowAgent {
    /// 应用远程 Delta 更新影子状态
    pub fn apply_delta(&mut self, delta: &AgentDelta) {
        match delta {
            AgentDelta::AgentMoved { id, position, health, max_health, is_alive, age, .. } => {
                if id == &self.id {
                    self.position = *position;
                    self.health = *health;
                    self.max_health = *max_health;
                    self.is_alive = *is_alive;
                    self.age = *age;
                    self.last_seen_tick = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                }
            }
            AgentDelta::AgentDied { id, .. } => {
                if id == &self.id {
                    self.is_alive = false;
                }
            }
            _ => {}
        }
    }

    /// 检查是否过期（超过指定 tick 数未收到更新）
    pub fn is_expired(&self, _current_tick: u64, timeout_ticks: u64) -> bool {
        // 这里用 wall-clock 近似，实际应由 simulation 传入 last_seen_tick 对比
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now.saturating_sub(self.last_seen_tick) > timeout_ticks * 5 // 每 tick ~5s
    }

    /// 从 AgentMoved 创建新的 ShadowAgent
    pub fn from_moved(delta: &AgentDelta, source_peer_id: &str, current_tick: u64) -> Option<Self> {
        match delta {
            AgentDelta::AgentMoved { id, name, position, health, max_health, is_alive, age } => {
                Some(ShadowAgent {
                    id: id.clone(),
                    name: name.clone(),
                    position: *position,
                    health: *health,
                    max_health: *max_health,
                    is_alive: *is_alive,
                    age: *age,
                    last_seen_tick: current_tick,
                    source_peer_id: source_peer_id.to_string(),
                })
            }
            _ => None,
        }
    }
}
