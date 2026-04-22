//! P2P 消息处理器
//!
//! 处理远程 GossipSub 消息，过滤本地回环，更新影子状态。

use std::sync::mpsc::Sender;
use crate::simulation::{AgentDelta, DeltaEnvelope};
use crate::agent::ShadowAgent;
use crate::types::AgentId;
use std::collections::HashMap;

/// P2P 消息处理器
///
/// 负责处理远程 Delta 消息，过滤本地回环，更新影子 Agent 状态
pub struct P2PMessageHandler {
    /// 本地 peer ID（用于过滤回环）
    local_peer_id: String,
    /// 远程 Agent 影子状态存储
    shadow_agents: HashMap<AgentId, ShadowAgent>,
    /// 本地 mpsc 通道（用于通知渲染）
    local_tx: Sender<AgentDelta>,
    /// 超时 tick 数
    shadow_timeout_ticks: u64,
}

impl P2PMessageHandler {
    /// 创建新的 P2P 消息处理器
    pub fn new(local_peer_id: String, local_tx: Sender<AgentDelta>, shadow_timeout_ticks: u64) -> Self {
        Self {
            local_peer_id,
            shadow_agents: HashMap::new(),
            local_tx,
            shadow_timeout_ticks,
        }
    }

    /// 处理远程 Delta 消息
    ///
    /// 1. 过滤本地回环（source_peer_id != local_peer_id）
    /// 2. 更新或创建影子 Agent
    /// 3. 发送本地 mpsc 通知渲染
    pub fn handle(&mut self, envelope: &DeltaEnvelope, current_tick: u64) {
        // 过滤本地回环
        if envelope.is_from_peer(&self.local_peer_id) {
            tracing::trace!("[P2PHandler] 过滤本地回环 delta");
            return;
        }

        let source_peer_id = envelope.source_peer_id.clone().unwrap_or_default();

        // 尝试从 AgentMoved 创建新影子
        if let Some(new_shadow) = ShadowAgent::from_moved(&envelope.delta, &source_peer_id, current_tick) {
            let agent_id = AgentId::new(new_shadow.id.clone());
            let shadow_id = new_shadow.id.clone();
            self.shadow_agents.insert(agent_id, new_shadow);
            tracing::info!("[P2PHandler] 创建新影子 Agent: {}", shadow_id);
        } else {
            // 更新现有影子
            let agent_id_str = envelope.delta.event_agent_id();
            if !agent_id_str.is_empty() {
                let agent_id = AgentId::new(agent_id_str);
                if let Some(shadow) = self.shadow_agents.get_mut(&agent_id) {
                    shadow.apply_delta(&envelope.delta);
                    shadow.last_seen_tick = current_tick;
                    tracing::trace!("[P2PHandler] 更新影子 Agent: {}", agent_id.as_str());
                }
            }
        }

        // 发送本地 mpsc 通知渲染
        if let Err(e) = self.local_tx.send(envelope.delta.clone()) {
            tracing::error!("[P2PHandler] 本地 delta 发送失败: {:?}", e);
        }
    }

    /// 清理过期影子 Agent
    pub fn cleanup_expired(&mut self, current_tick: u64) {
        let expired: Vec<AgentId> = self.shadow_agents.iter()
            .filter(|(_, shadow)| shadow.is_expired(current_tick, self.shadow_timeout_ticks))
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired {
            self.shadow_agents.remove(id);
            tracing::info!("[P2PHandler] 清理过期影子 Agent: {}", id.as_str());
        }
    }

    /// 获取所有影子 Agent（用于渲染）
    pub fn get_shadow_agents(&self) -> &HashMap<AgentId, ShadowAgent> {
        &self.shadow_agents
    }

    /// 获取本地 peer ID
    pub fn local_peer_id(&self) -> &str {
        &self.local_peer_id
    }
}

/// AgentDelta event_agent_id 辅助方法
impl AgentDelta {
    /// 获取事件涉及的 Agent ID（用于影子状态更新）
    pub fn event_agent_id(&self) -> String {
        match self {
            AgentDelta::AgentMoved { id, .. } => id.clone(),
            AgentDelta::AgentDied { id, .. } => id.clone(),
            AgentDelta::AgentSpawned { id, .. } => id.clone(),
            AgentDelta::HealedByCamp { agent_id, .. } => agent_id.clone(),
            AgentDelta::SurvivalWarning { agent_id, .. } => agent_id.clone(),
            // 其他事件可能不涉及特定 Agent ID
            _ => String::new(),
        }
    }
}