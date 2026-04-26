//! 影子 Agent（P2P 模式）
//!
//! 用于表示远程 peer 管理的 Agent，只保留渲染和基础交互所需的最少字段。
//! 使用统一的 AgentState 结构。

use serde::{Deserialize, Serialize};
use crate::simulation::{Delta, ChangeHint};
use crate::snapshot::AgentState;

/// 影子 Agent：精简字段，仅用于 P2P 远程 Agent 同步
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowAgent {
    /// 统一的 Agent 状态
    pub state: AgentState,
    /// 最后一次收到更新的 tick
    pub last_seen_tick: u64,
    /// 来源 peer ID
    pub source_peer_id: String,
}

impl ShadowAgent {
    /// 应用远程 Delta 更新影子状态
    pub fn apply_delta(&mut self, delta: &Delta) {
        match delta {
            Delta::AgentStateChanged { agent_id, state, change_hint, .. } => {
                if agent_id == &self.state.id {
                    self.state = state.clone();
                    self.last_seen_tick = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    // 处理死亡标记
                    if *change_hint == ChangeHint::Died {
                        self.state.is_alive = false;
                    }
                }
            }
            _ => {}
        }
    }

    /// 检查是否过期（超过指定 tick 数未收到更新）
    ///
    /// last_seen_tick 存储的是 wall-clock 秒数，timeout_ticks 是仿真 tick 数。
    /// 假设每 tick 约 5 秒，将 timeout_ticks 转换为秒后比较。
    pub fn is_expired(&self, _current_tick: u64, timeout_ticks: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let timeout_secs = timeout_ticks.saturating_mul(5);
        now.saturating_sub(self.last_seen_tick) > timeout_secs
    }

    /// 从 AgentStateChanged 创建新的 ShadowAgent
    pub fn from_state(state: &AgentState, source_peer_id: &str, _current_tick: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        ShadowAgent {
            state: state.clone(),
            last_seen_tick: now,
            source_peer_id: source_peer_id.to_string(),
        }
    }

    /// 获取 Agent ID
    pub fn id(&self) -> &str {
        &self.state.id
    }

    /// 获取位置
    pub fn position(&self) -> (u32, u32) {
        self.state.position
    }

    /// 是否存活
    pub fn is_alive(&self) -> bool {
        self.state.is_alive
    }
}