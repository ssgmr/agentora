//! 遗产系统：Agent 死亡→遗迹→回响→契约→广播闭环

use crate::types::{AgentId, Position};
use crate::agent::Agent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 遗产类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyType {
    Grave,       // 墓冢
    Ruins,       // 废墟
    Artifact,    // 遗物
}

/// 遗产实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Legacy {
    pub id: String,
    pub position: Position,
    pub legacy_type: LegacyType,
    pub original_agent_id: AgentId,
    pub original_agent_name: String,
    pub items: HashMap<String, u32>,
    pub echo_log: Option<EchoLog>,
    pub created_tick: u64,
    pub decay_tick: u64,  // 物品衰减开始 tick
}

impl Legacy {
    /// 从死亡 Agent 生成遗产
    pub fn from_agent(agent: &Agent, tick: u64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            position: agent.position,
            legacy_type: LegacyType::Grave,
            original_agent_id: agent.id.clone(),
            original_agent_name: agent.name.clone(),
            items: agent.inventory.clone(),
            echo_log: Some(EchoLog::from_agent(agent)),
            created_tick: tick,
            decay_tick: tick + 50,  // 50 tick 后开始衰减
        }
    }

    /// 检查物品是否开始衰减
    pub fn is_decaying(&self, current_tick: u64) -> bool {
        current_tick >= self.decay_tick
    }

    /// 衰减物品（每 tick 衰减 10%）
    pub fn decay_items(&mut self) {
        for (_, amount) in self.items.iter_mut() {
            *amount = (*amount as f32 * 0.9) as u32;
        }
        // 清空零值物品
        self.items.retain(|_, v| *v > 0);
    }
}

/// 回响日志（Agent 死亡时的记忆压缩）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoLog {
    pub summary: String,
    pub emotion_tags: Vec<String>,
    pub final_words: Option<String>,
    pub key_memories: Vec<String>,
}

impl EchoLog {
    /// 从 Agent 生成回响日志
    pub fn from_agent(agent: &Agent) -> Self {
        // 1.1 获取最后 3 条短期记忆
        let last_3_memories: Vec<&crate::memory::MemoryEvent> = agent.memory.get_recent_memories(3);
        let memory_text: String = last_3_memories.iter()
            .map(|e| format!("[tick {}] {}: {}", e.tick, e.event_type, e.content))
            .collect::<Vec<_>>()
            .join("\n");

        // 1.2 构建压缩 Prompt
        let prompt = format!(
            "请压缩以下 Agent 的最后记忆为简短的回响摘要（100 字以内），并提取情感标签和关键记忆：\n\n\
             {}\n\n\
             请按照以下 JSON 格式返回：\n\
             {{\n\
               \"summary\": \"回响摘要\",\n\
               \"emotion_tags\": [\"情感 1\", \"情感 2\"],\n\
               \"key_memories\": [\"关键记忆 1\", \"关键记忆 2\"]\n\
             }}",
            memory_text
        );

        // 1.3 LLM 调用压缩记忆（当前使用兜底逻辑）
        let (summary, emotion_tags, key_memories) = Self::compress_with_llm(&prompt);

        Self {
            summary,
            emotion_tags,
            final_words: None,
            key_memories,
        }
    }

    /// 使用 LLM 压缩记忆
    fn compress_with_llm(prompt: &str) -> (String, Vec<String>, Vec<String>) {
        // 尝试使用 LLM 压缩，失败时使用兜底逻辑
        // 注意：实际使用需要异步运行时，这里使用简化实现

        // 兜底逻辑：简单提取关键信息
        let summary = "记忆已随 Agent 消散".to_string();
        let emotion_tags = vec!["遗憾".to_string()];
        let key_memories = vec![prompt.lines().next().unwrap_or("").to_string()];

        (summary, emotion_tags, key_memories)
    }

    /// 解析 LLM JSON 响应
    #[allow(dead_code)]
    fn parse_llm_response(response: &str) -> (String, Vec<String>, Vec<String>) {
        match serde_json::from_str::<serde_json::Value>(response) {
            Ok(json) => {
                let summary = json["summary"].as_str().unwrap_or("记忆已消散").to_string();
                let emotion_tags = json["emotion_tags"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let key_memories = json["key_memories"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                (summary, emotion_tags, key_memories)
            }
            Err(_) => {
                // 解析失败时使用默认值
                (response.to_string(), vec![], vec![])
            }
        }
    }
}

/// 遗产交互类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyInteractionType {
    Worship,   // 祭拜
    Pickup,    // 拾取物品
}

/// 遗产交互结果
#[derive(Debug, Clone)]
pub enum LegacyInteractionResult {
    Worshipped { legacy_id: String },
    Pickup { legacy_id: String, items_gained: HashMap<String, u32> },
}

/// 遗产事件（用于广播）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyEvent {
    pub legacy_id: String,
    pub original_agent_id: AgentId,
    pub original_agent_name: String,
    pub position: Position,
    pub legacy_type: LegacyType,
    pub created_tick: u64,
}

impl LegacyEvent {
    pub fn from_legacy(legacy: &Legacy) -> Self {
        Self {
            legacy_id: legacy.id.clone(),
            original_agent_id: legacy.original_agent_id.clone(),
            original_agent_name: legacy.original_agent_name.clone(),
            position: legacy.position,
            legacy_type: legacy.legacy_type,
            created_tick: legacy.created_tick,
        }
    }
}