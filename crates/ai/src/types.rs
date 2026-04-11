//! LLM请求/响应类型定义

use serde::{Deserialize, Serialize};

/// 响应格式类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseFormat {
    Text,
    Json { schema: Option<String> },
}

/// LLM请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub response_format: ResponseFormat,
    pub stop_sequences: Vec<String>,
}

impl Default for LlmRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            max_tokens: 500,
            temperature: 0.7,
            response_format: ResponseFormat::Json { schema: None },
            stop_sequences: vec!["\n\n".to_string()],
        }
    }
}

/// LLM响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub raw_text: String,
    pub parsed_action: Option<serde_json::Value>,
    pub usage: TokenUsage,
    pub provider_name: String,
}

/// Token使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl Default for TokenUsage {
    fn default() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        }
    }
}

/// LLM错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmError {
    NetworkError(String),
    Timeout,
    RateLimited { retry_after: u32 },
    InvalidResponse(String),
    JsonParseError(String),
    ProviderUnavailable(String),
    ApiError { code: u32, message: String },
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::NetworkError(msg) => write!(f, "网络错误: {}", msg),
            LlmError::Timeout => write!(f, "请求超时"),
            LlmError::RateLimited { retry_after } => write!(f, "被限流，{}秒后重试", retry_after),
            LlmError::InvalidResponse(msg) => write!(f, "无效响应: {}", msg),
            LlmError::JsonParseError(msg) => write!(f, "JSON解析失败: {}", msg),
            LlmError::ProviderUnavailable(name) => write!(f, "Provider不可用: {}", name),
            LlmError::ApiError { code, message } => write!(f, "API错误({}): {}", code, message),
        }
    }
}

impl std::error::Error for LlmError {}