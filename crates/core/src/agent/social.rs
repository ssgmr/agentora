//! 社交系统：对话与信任建立

use crate::types::AgentId;
use crate::memory::MemoryEvent;

impl crate::agent::Agent {
    /// 与附近Agent交谈：记录记忆，增加信任
    pub fn talk_with(&mut self, nearby_ids: &[AgentId], message: &str, tick: u32) {
        for target_id in nearby_ids {
            self.increase_trust(target_id, 2.0);
            self.memory.record(&MemoryEvent {
                tick,
                event_type: "social".to_string(),
                content: format!("与 {} 交流：「{}」", target_id.as_str(), message),
                emotion_tags: vec!["positive".to_string()],
                importance: 0.5,
            });
        }
    }

    /// 被交谈：增加信任，记录记忆
    pub fn receive_talk(&mut self, speaker_id: &AgentId, speaker_name: &str, message: &str, tick: u32) {
        self.increase_trust(speaker_id, 1.0);
        self.memory.record(&MemoryEvent {
            tick,
            event_type: "social".to_string(),
            content: format!("{} 与你交流：「{}」", speaker_name, message),
            emotion_tags: vec!["positive".to_string()],
            importance: 0.5,
        });
    }
}