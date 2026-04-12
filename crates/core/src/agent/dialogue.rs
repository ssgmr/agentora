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

    /// 根据动机最高维度生成对话内容（LLM 不可用时的兜底）
    pub fn generate_dialogue_fallback(&self, target_name: &str) -> String {
        // 找到最高动机维度
        let dims = ["生存", "社交", "认知", "表达", "权力", "传承"];
        let motivation = self.motivation.to_array();
        let mut max_idx = 0;
        for i in 1..6 {
            if motivation[i] > motivation[max_idx] {
                max_idx = i;
            }
        }

        // 根据关系类型调整语气
        let trust = self.relations.values().next().map(|r| r.trust).unwrap_or(0.0);
        let tone = if trust > 20.0 { "友好地" } else if trust < -10.0 { "冷淡地" } else { "" };

        match max_idx {
            0 => {
                // 生存动机最高
                let msgs = [
                    format!("{}我需要更多食物和水", tone),
                    format!("{}最近资源越来越少了", tone),
                    format!("{}得赶紧找点吃的", tone),
                ];
                msgs[self.age as usize % msgs.len()].to_string()
            }
            1 => {
                // 社交动机最高
                let msgs = [
                    format!("{}你好，愿意合作吗？", tone),
                    format!("{}我们一起行动吧", tone),
                    format!("{}{}，聊聊你最近的发现？", tone, target_name),
                ];
                msgs[self.age as usize % msgs.len()].to_string()
            }
            2 => {
                // 认知动机最高
                let msgs = [
                    format!("{}我注意到一个有趣的现象", tone),
                    format!("{}这片区域的地形很特别", tone),
                    format!("{}我在想这个世界的运行规律", tone),
                ];
                msgs[self.age as usize % msgs.len()].to_string()
            }
            3 => {
                // 表达动机最高
                let msgs = [
                    format!("{}我想分享一个想法", tone),
                    format!("{}你觉得我们该怎么发展？", tone),
                    format!("{}我有件事想和大家商量", tone),
                ];
                msgs[self.age as usize % msgs.len()].to_string()
            }
            4 => {
                // 权力动机最高
                let msgs = [
                    format!("{}跟着我，我能保护你们", tone),
                    format!("{}这片区域应该由我来管理", tone),
                    format!("{}{}，你需要我的帮助吗？", tone, target_name),
                ];
                msgs[self.age as usize % msgs.len()].to_string()
            }
            _ => {
                // 传承动机最高
                let msgs = [
                    format!("{}我们得为后人留下些什么", tone),
                    format!("{}前人留下的遗产给了我们很多启发", tone),
                    format!("{}文明需要传承下去", tone),
                ];
                msgs[self.age as usize % msgs.len()].to_string()
            }
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
