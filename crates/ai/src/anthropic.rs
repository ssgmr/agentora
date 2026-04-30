//! Anthropic Provider

use async_trait::async_trait;
use crate::provider::LlmProvider;
use crate::types::{LlmRequest, LlmResponse, LlmError};

/// Anthropic Claude API Provider
pub struct AnthropicProvider {
    api_base: String,
    api_key: String,
    model: String,
    timeout_seconds: u32,
}

impl AnthropicProvider {
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
impl LlmProvider for AnthropicProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_seconds as u64))
            .build()
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        // Anthropic 使用 prefill trick 引导 JSON 输出
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": [
                {"role": "user", "content": request.prompt},
                {"role": "assistant", "content": "{"}  // prefill 引导 JSON
            ],
        });

        // 重试逻辑：最多重试 2 次
        let mut last_error = None;
        for attempt in 0..=2 {
            if attempt > 0 {
                tracing::info!("Anthropic Provider 重试第 {} 次", attempt);
            }

            let response = client
                .post(format!("{}/v1/messages", self.api_base))
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
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

                if attempt < 2 {
                    tracing::warn!("被限流，等待 {} 秒后重试", retry_after);
                    tokio::time::sleep(tokio::time::Duration::from_secs(retry_after as u64)).await;
                    last_error = Some(LlmError::RateLimited { retry_after });
                    continue;
                } else {
                    return Err(LlmError::RateLimited { retry_after });
                }
            }

            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

            // 补全 prefill 的 JSON
            let raw_text = format!("{{{}}}", json["content"][0]["text"]
                .as_str()
                .unwrap_or(""));

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
        "anthropic"
    }

    fn is_available(&self) -> bool {
        !self.api_base.is_empty() && !self.api_key.is_empty()
    }
}
