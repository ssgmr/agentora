//! 重试逻辑
//!
//! 429 限流检测 + Retry-After 解析 + 自动重试（最多 2 次）

use async_trait::async_trait;
use crate::provider::LlmProvider;
use crate::types::{LlmRequest, LlmResponse, LlmError};

/// 重试包装器：在底层 Provider 返回 429 错误时自动重试
pub struct RetryProvider<P: LlmProvider> {
    inner: P,
    max_retries: u32,
}

impl<P: LlmProvider> RetryProvider<P> {
    pub fn new(inner: P, max_retries: u32) -> Self {
        Self { inner, max_retries }
    }

    /// 带重试的生成
    pub async fn generate_with_retry(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                tracing::info!("重试第 {} 次", attempt);
            }

            match self.inner.generate(request.clone()).await {
                Ok(response) => {
                    if attempt > 0 {
                        tracing::info!("重试成功");
                    }
                    return Ok(response);
                }
                Err(LlmError::RateLimited { retry_after }) => {
                    let wait_seconds = if retry_after > 0 {
                        retry_after
                    } else {
                        // 指数退避：2^attempt 秒
                        (1 << attempt).min(60) as u32
                    };

                    if attempt < self.max_retries {
                        tracing::warn!(
                            "被限流，等待 {} 秒后重试 (剩余 {} 次)",
                            wait_seconds,
                            self.max_retries - attempt
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds as u64)).await;
                        last_error = Some(LlmError::RateLimited { retry_after });
                    } else {
                        tracing::error!("重试 {} 次后仍被限流", self.max_retries);
                        last_error = Some(LlmError::RateLimited { retry_after });
                    }
                }
                Err(e) => {
                    // 非 429 错误，直接返回
                    tracing::warn!("非限流错误，不重试：{}", e);
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::ProviderUnavailable("重试失败".to_string())))
    }
}

#[async_trait]
impl<P: LlmProvider> LlmProvider for RetryProvider<P> {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.generate_with_retry(request).await
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn is_available(&self) -> bool {
        self.inner.is_available()
    }
}

/// 检测 HTTP 状态码是否为 429
pub fn is_rate_limit_status(status: u16) -> bool {
    status == 429
}

/// 解析 Retry-After 头
pub fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<u32> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            // 可能是秒数或 HTTP 日期
            v.parse::<u32>().ok().or_else(|| {
                // 解析 HTTP 日期格式：Fri, 02 Dec 2024 07:38:00 GMT
                chrono::DateTime::parse_from_rfc2822(v)
                    .ok()
                    .map(|dt| {
                        let now = chrono::Utc::now();
                        let duration = dt.signed_duration_since(now);
                        duration.num_seconds().max(0) as u32
                    })
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_rate_limit_status() {
        assert!(is_rate_limit_status(429));
        assert!(!is_rate_limit_status(200));
        assert!(!is_rate_limit_status(500));
    }

    #[test]
    fn test_parse_retry_after_seconds() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "60".parse().unwrap());
        assert_eq!(parse_retry_after(&headers), Some(60));
    }

    #[test]
    fn test_parse_retry_after_missing() {
        let headers = reqwest::header::HeaderMap::new();
        assert_eq!(parse_retry_after(&headers), None);
    }
}
