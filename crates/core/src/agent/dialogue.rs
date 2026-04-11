//! 对话系统

impl crate::agent::Agent {
    /// 发起对话
    pub fn talk(&self, message: &str) -> DialogueMessage {
        DialogueMessage {
            speaker_id: self.id.clone(),
            content: message.to_string(),
            tick: 0, // TODO: 使用世界tick
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