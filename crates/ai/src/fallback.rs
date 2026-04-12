//! Provider 降级链
//!
//! 支持多个 Provider 依次尝试，最后使用规则引擎兜底

use async_trait::async_trait;
use crate::provider::LlmProvider;
use crate::types::{LlmRequest, LlmResponse, LlmError};

/// 降级链：多个 Provider 依次尝试
pub struct FallbackChain {
    providers: Vec<Box<dyn LlmProvider>>,
}

impl FallbackChain {
    pub fn new(providers: Vec<Box<dyn LlmProvider>>) -> Self {
        Self {
            providers,
        }
    }

    /// 添加 Provider
    pub fn add(&mut self, provider: Box<dyn LlmProvider>) {
        self.providers.push(provider);
    }

    /// 依次尝试所有 Provider
    pub async fn generate_with_fallback(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        for provider in &self.providers {
            if !provider.is_available() {
                tracing::debug!("Provider {} 不可用，跳过", provider.name());
                continue;
            }

            match provider.generate(request.clone()).await {
                Ok(response) => {
                    tracing::info!("Provider {} 成功", provider.name());
                    return Ok(response);
                }
                Err(e) => {
                    tracing::warn!("Provider {} 失败：{}", provider.name(), e);
                    continue;
                }
            }
        }

        // 所有 Provider 都失败
        Err(LlmError::ProviderUnavailable("所有 Provider 都失败".to_string()))
    }
}

#[async_trait]
impl LlmProvider for FallbackChain {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.generate_with_fallback(request).await
    }

    fn name(&self) -> &str {
        "fallback_chain"
    }

    fn is_available(&self) -> bool {
        self.providers.iter().any(|p| p.is_available())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenUsage;

    /// 模拟 Provider 用于测试
    struct MockProvider {
        name: String,
        should_fail: bool,
    }

    impl MockProvider {
        fn new(name: &str, should_fail: bool) -> Self {
            Self {
                name: name.to_string(),
                should_fail,
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            if self.should_fail {
                Err(LlmError::ProviderUnavailable(format!("{} 失败", self.name)))
            } else {
                Ok(LlmResponse {
                    raw_text: format!("{} 成功", self.name),
                    parsed_action: None,
                    usage: TokenUsage::default(),
                    provider_name: self.name.clone(),
                })
            }
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn is_available(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_fallback_chain_success_on_first() {
        let chain = FallbackChain::new(
            vec![
                Box::new(MockProvider::new("first", false)),
                Box::new(MockProvider::new("second", true)),
            ],
        );

        let request = LlmRequest::default();
        let result = chain.generate(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().raw_text, "first 成功");
    }

    #[tokio::test]
    async fn test_fallback_chain_success_on_second() {
        let chain = FallbackChain::new(
            vec![
                Box::new(MockProvider::new("first", true)),
                Box::new(MockProvider::new("second", false)),
            ],
        );

        let request = LlmRequest::default();
        let result = chain.generate(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().raw_text, "second 成功");
    }

    #[tokio::test]
    async fn test_fallback_chain_all_fail() {
        let chain = FallbackChain::new(
            vec![
                Box::new(MockProvider::new("first", true)),
                Box::new(MockProvider::new("second", true)),
            ],
        );

        let request = LlmRequest::default();
        let result = chain.generate(request).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("所有 Provider 都失败")
        );
    }
}
