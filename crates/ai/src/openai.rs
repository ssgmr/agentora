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

            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

            let raw_text = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();

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
