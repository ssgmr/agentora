//! Action 执行结果 Schema
//!
//! 定义结构化的动作反馈格式，统一 SuccessDetail 和 BlockedReason。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Action 执行结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionResultSchema {
    /// 成功，携带变更详情
    Success {
        action_type: String,
        changes: Vec<FieldChange>,
    },
    /// 失败，携带原因和建议
    Blocked {
        error_code: String,
        reason: String,
        suggestion: Option<ActionSuggestion>,
    },
    /// 已在目标位置（特殊成功情况）
    AlreadyAtPosition {
        detail: String,
    },
    /// Agent 不存在
    InvalidAgent,
    /// Agent 已死亡
    AgentDead,
    /// 超出边界
    OutOfBounds,
    /// 动作未实现
    NotImplemented,
}

/// 字段变更记录
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldChange {
    /// 字段路径（如 "position.x", "inventory.food"）
    pub field: String,
    /// 变更前的值
    pub before: serde_json::Value,
    /// 变更后的值
    pub after: serde_json::Value,
}

/// 动作建议（用于校验失败时的引导）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionSuggestion {
    /// 建议的动作类型
    pub action_type: String,
    /// 建议的参数
    pub params: HashMap<String, serde_json::Value>,
}

impl ActionResultSchema {
    /// 从 ActionResult 转换（向后兼容）
    pub fn from_legacy(result: &crate::world::ActionResult) -> Self {
        match result {
            crate::world::ActionResult::SuccessWithDetail(detail) => {
                ActionResultSchema::Success {
                    action_type: "unknown".to_string(),
                    changes: vec![FieldChange {
                        field: "result".to_string(),
                        before: serde_json::Value::Null,
                        after: serde_json::json!(detail),
                    }],
                }
            }
            crate::world::ActionResult::Blocked(reason) => {
                ActionResultSchema::Blocked {
                    error_code: "blocked".to_string(),
                    reason: reason.clone(),
                    suggestion: None,
                }
            }
            crate::world::ActionResult::AlreadyAtPosition(detail) => {
                ActionResultSchema::AlreadyAtPosition {
                    detail: detail.clone(),
                }
            }
            crate::world::ActionResult::InvalidAgent => ActionResultSchema::InvalidAgent,
            crate::world::ActionResult::AgentDead => ActionResultSchema::AgentDead,
            crate::world::ActionResult::OutOfBounds => ActionResultSchema::OutOfBounds,
            crate::world::ActionResult::NotImplemented => ActionResultSchema::NotImplemented,
        }
    }

    /// 生成反馈文本（用于 LLM）
    pub fn to_feedback_text(&self) -> String {
        match self {
            ActionResultSchema::Success { action_type, changes } => {
                let changes_text = changes.iter()
                    .map(|c| format!("{}: {} → {}", c.field,
                        if c.before.is_null() { "无".to_string() } else { c.before.to_string() },
                        c.after.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("✓ {} 执行成功，变更：{}", action_type, changes_text)
            }
            ActionResultSchema::Blocked { error_code, reason, suggestion } => {
                let mut text = format!("✗ 动作被拒绝（{}）：{}", error_code, reason);
                if let Some(sug) = suggestion {
                    text.push_str(&format!("\n建议：{} {:?}", sug.action_type, sug.params));
                }
                text
            }
            ActionResultSchema::AlreadyAtPosition { detail } => {
                format!("已在目标位置：{}", detail)
            }
            ActionResultSchema::InvalidAgent => "Agent 不存在".to_string(),
            ActionResultSchema::AgentDead => "Agent 已死亡".to_string(),
            ActionResultSchema::OutOfBounds => "超出地图边界".to_string(),
            ActionResultSchema::NotImplemented => "动作未实现".to_string(),
        }
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        matches!(self,
            ActionResultSchema::Success { .. } |
            ActionResultSchema::AlreadyAtPosition { .. })
    }
}

impl Default for ActionResultSchema {
    fn default() -> Self {
        ActionResultSchema::NotImplemented
    }
}