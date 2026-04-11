//! LLM Provider trait定义

use async_trait::async_trait;
use crate::types::{LlmRequest, LlmResponse, LlmError};

/// LLM Provider抽象接口
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 生成响应
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Provider名称
    fn name(&self) -> &str;

    /// 检查是否可用
    fn is_available(&self) -> bool;
}