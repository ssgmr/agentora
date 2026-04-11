//! 短期记忆缓存
//!
//! 最近5条事件（默认可配置），不持久化，仅作为Echo到ChronicleDB的中间缓存

use agentora_ai::config::MemoryConfig;

/// 短期记忆
#[derive(Debug, Clone)]
pub struct ShortTermMemory {
    events: Vec<crate::memory::MemoryEvent>,
    capacity: usize,
}

impl ShortTermMemory {
    /// 从配置初始化
    pub fn from_config(config: &MemoryConfig) -> Self {
        Self {
            events: Vec::with_capacity(config.short_term_capacity),
            capacity: config.short_term_capacity,
        }
    }

    /// 使用默认配置初始化（向后兼容）
    pub fn with_defaults() -> Self {
        Self::from_config(&MemoryConfig::default())
    }

    pub fn new() -> Self {
        Self::with_defaults()
    }

    /// 添加事件
    pub fn push(&mut self, event: crate::memory::MemoryEvent) {
        if self.events.len() >= self.capacity {
            // 溢出时移除最旧的
            let oldest = self.events.remove(0);
            // TODO: 若importance > 0.5，迁移到ChronicleDB
            tracing::debug!("Short term memory overflow, oldest event: {:?}", oldest);
        }
        self.events.push(event);
    }

    /// 获取摘要文本
    pub fn summary(&self) -> String {
        self.events.iter()
            .map(|e| format!("[tick {}] {}: {}", e.tick, e.event_type, e.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 获取最近事件
    pub fn latest(&self) -> Option<&crate::memory::MemoryEvent> {
        self.events.last()
    }

    /// 获取最近 N 条事件
    pub fn get_recent(&self, n: usize) -> Vec<&crate::memory::MemoryEvent> {
        self.events.iter().rev().take(n).collect()
    }

    /// 清空
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::with_defaults()
    }
}