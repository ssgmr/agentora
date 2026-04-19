//! OpenAI 兼容 Provider

use async_trait::async_trait;
use crate::provider::LlmProvider;
use crate::types::{LlmRequest, LlmResponse, LlmError};

/// OpenAI 兼容 API Provider
pub struct OpenAiProvider {
    api_base: String,
    api_key: String,
    model: String,
    timeout_seconds: u32,
}

impl OpenAiProvider {
    pub fn new(api_base: String, api_key: String, model: String) -> Self {
        Self {
            api_base,
            api_key,
            model,
            timeout_seconds: 10,
        }
    }

    pub fn with_timeout(mut self, timeout: u32) -> Self {
        self.timeout_seconds = timeout;
        self
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_seconds as u64))
            .build()
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "user", "content": request.prompt}
            ],
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        // 记录请求信息
        let prompt_chars = request.prompt.len();
        let prompt_estimated_tokens = estimate_tokens_approx(&request.prompt);
        tracing::info!(
            "[LLM Request] model={}, prompt={} chars (≈{} tokens), max_output_tokens={}, temperature={}",
            self.model, prompt_chars, prompt_estimated_tokens, request.max_tokens, request.temperature
        );

        // 重试逻辑：最多重试 1 次（快速失败，走规则引擎兜底）
        let mut last_error = None;
        for attempt in 0..=1 {
            if attempt > 0 {
                tracing::info!("OpenAI Provider 重试第 {} 次", attempt);
            }

            let response = client
                .post(format!("{}/v1/chat/completions", self.api_base))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;

            if response.status() == 429 {
                let retry_after = response.headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(60);

                if attempt < 1 {
                    tracing::warn!("被限流，等待 {} 秒后重试", retry_after);
                    tokio::time::sleep(tokio::time::Duration::from_secs(retry_after as u64)).await;
                    last_error = Some(LlmError::RateLimited { retry_after });
                    continue;
                } else {
                    return Err(LlmError::RateLimited { retry_after });
                }
            }

            let status = response.status();
            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

            // 检查 LLM 返回的错误信息
            if let Some(error) = json.get("error").and_then(|e| e.as_str()) {
                tracing::error!("LLM 返回错误：{}", error);
                return Err(LlmError::InvalidResponse(error.to_string()));
            }
            if !status.is_success() {
                let error_detail = json.to_string();
                tracing::error!("LLM HTTP {} 失败：{}", status, error_detail);
                return Err(LlmError::InvalidResponse(format!("HTTP {}: {}", status, error_detail)));
            }

            let raw_text = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();

            // 记录响应信息
            let response_chars = raw_text.len();
            let response_estimated_tokens = estimate_tokens_approx(&raw_text);
            let total_estimated_tokens = prompt_estimated_tokens + response_estimated_tokens;

            // 尝试从 usage 字段获取实际 token 数
            let prompt_tokens = json["usage"]["prompt_tokens"].as_u64();
            let completion_tokens = json["usage"]["completion_tokens"].as_u64();
            let total_tokens = json["usage"]["total_tokens"].as_u64();

            if let (Some(p), Some(c), Some(t)) = (prompt_tokens, completion_tokens, total_tokens) {
                tracing::info!(
                    "[LLM Response] {} chars (≈{} tokens), 实际用量: prompt_tokens={}, completion_tokens={}, total_tokens={}",
                    response_chars, response_estimated_tokens, p, c, t
                );
            } else {
                tracing::info!(
                    "[LLM Response] {} chars (≈{} tokens), 总估算用量: {} tokens (input≈{} + output≈{})",
                    response_chars, response_estimated_tokens, total_estimated_tokens,
                    prompt_estimated_tokens, response_estimated_tokens
                );
            }

            // 记录响应内容（截断显示，避免日志过长）
            let preview = if raw_text.len() > 300 {
                let end = raw_text.char_indices().find(|(idx, _)| *idx >= 300)
                    .map(|(idx, c)| idx + c.len_utf8())
                    .unwrap_or(300);
                format!("{}...", &raw_text[..end])
            } else {
                raw_text.clone()
            };
            tracing::debug!("[LLM Response Preview]\n{}", preview);

            return Ok(LlmResponse {
                raw_text,
                parsed_action: None,
                usage: crate::types::TokenUsage::default(),
                provider_name: self.name().to_string(),
            });
        }

        Err(last_error.unwrap_or_else(|| LlmError::NetworkError("重试失败".to_string())))
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn is_available(&self) -> bool {
        // 端点已配置即视为可用（很多本地/私有部署不需要 API key）
        !self.api_base.is_empty()
    }
}

/// 估算文本的 token 数
/// 中文按 1.5 char/token（经验值），英文按 4 char/token
fn estimate_tokens_approx(text: &str) -> usize {
    let mut chinese_chars = 0;
    let mut other_chars = 0;
    for ch in text.chars() {
        if ch.is_ascii() {
            other_chars += 1;
        } else {
            chinese_chars += 1;
        }
    }
    (chinese_chars as f64 / 1.5) as usize + (other_chars as f64 / 4.0) as usize
}
