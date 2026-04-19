//! 对话系统

use crate::agent::RelationType;

impl crate::agent::Agent {
    /// 发起对话
    pub fn talk(&self, message: &str, world_tick: u32) -> DialogueMessage {
        DialogueMessage {
            speaker_id: self.id.clone(),
            content: message.to_string(),
            tick: world_tick,
        }
    }
}

/// 对话消息
#[derive(Debug, Clone)]
pub struct DialogueMessage {
    pub speaker_id: crate::types::AgentId,
    pub content: String,
    pub tick: u32,
}
