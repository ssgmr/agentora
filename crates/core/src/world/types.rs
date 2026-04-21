//! 世界辅助类型定义
//!
//! 包含里程碑、交易、对话等辅助数据结构。

use crate::types::AgentId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 里程碑类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MilestoneType {
    FirstCamp,           // 第一座营地
    FirstTrade,          // 贸易萌芽
    FirstFence,          // 领地意识
    FirstAttack,         // 冲突爆发
    FirstLegacyInteract, // 首次传承
    CityState,           // 城邦雏形
    GoldenAge,           // 文明黄金期
}

/// 文明里程碑
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub name: String,
    pub display_name: String,
    pub achieved_tick: u64,
}

/// 交易状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeStatus {
    Pending,
    Accepted,
    Rejected,
}

/// 待处理交易
#[derive(Debug, Clone)]
pub struct PendingTrade {
    pub proposer_id: AgentId,
    pub acceptor_id: AgentId,
    pub offer_resources: HashMap<String, u32>,
    pub want_resources: HashMap<String, u32>,
    pub status: TradeStatus,
    pub tick_created: u64,
}

/// 对话日志
#[derive(Debug, Clone)]
pub struct DialogueLog {
    pub agent_a: AgentId,
    pub agent_b: AgentId,
    pub messages: Vec<DialogueMessage>,
    pub tick_started: u64,
    pub is_active: bool,
}

/// 对话消息
#[derive(Debug, Clone)]
pub struct DialogueMessage {
    pub speaker_id: AgentId,
    pub content: String,
    pub tick: u64,
}