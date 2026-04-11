//! Provider 降级链
//!
//! 支持多个 Provider 依次尝试，最后使用规则引擎兜底

use async_trait::async_trait;
use crate::provider::LlmProvider;
use crate::types::{LlmRequest, LlmResponse, LlmError};

/// 降级链：多个 Provider 依次尝试
pub struct FallbackChain {
    providers: Vec<Box<dyn LlmProvider>>,
    use_rule_engine_fallback: bool,
}

impl FallbackChain {
    pub fn new(providers: Vec<Box<dyn LlmProvider>>, use_rule_engine_fallback: bool) -> Self {
        Self {
            providers,
            use_rule_engine_fallback,
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

        // 所有 Provider 都失败，使用规则引擎兜底
        if self.use_rule_engine_fallback {
            tracing::warn!("所有 LLM Provider 都失败，使用规则引擎兜底");
            return self.generate_rule_engine_fallback(request).await;
        }

        Err(LlmError::ProviderUnavailable("所有 Provider 都失败".to_string()))
    }

    /// 规则引擎兜底生成
    async fn generate_rule_engine_fallback(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // 使用 rule_engine 模块生成兜底动作
        use crate::rule_engine::{fallback_decision, SimplePosition};

        // 默认位置和动机（实际使用时应该从上下文获取）
        let position = SimplePosition { x: 0, y: 0 };
        let motivation = [0.5; 6];

        let action = fallback_decision(&position, &motivation);

        tracing::info!("规则引擎兜底：{}", action.reasoning);

        // 将兜底动作转换为 JSON 响应
        let json_str = match &action.action_type {
            crate::rule_engine::SimpleActionType::Wait => {
                format!(r#"{{"action": "wait", "reasoning": "{}"}}"#, action.reasoning)
            }
            crate::rule_engine::SimpleActionType::Move { direction } => {
                format!(r#"{{"action": "move", "direction": "{}", "reasoning": "{}"}}"#, direction, action.reasoning)
            }
            crate::rule_engine::SimpleActionType::Explore { target_region } => {
                format!(r#"{{"action": "explore", "target_region": {}, "reasoning": "{}"}}"#, target_region, action.reasoning)
            }
        };

        Ok(LlmResponse {
            raw_text: json_str,
            parsed_action: None,
            usage: crate::types::TokenUsage::default(),
            provider_name: "rule_engine_fallback".to_string(),
        })
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
            false,
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
            false,
        );

        let request = LlmRequest::default();
        let result = chain.generate(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().raw_text, "second 成功");
    }

    #[tokio::test]
    async fn test_fallback_chain_all_fail_without_rule_engine() {
        let chain = FallbackChain::new(
            vec![
                Box::new(MockProvider::new("first", true)),
                Box::new(MockProvider::new("second", true)),
            ],
            false, // 不使用规则引擎兜底
        );

        let request = LlmRequest::default();
        let result = chain.generate(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fallback_chain_rule_engine_fallback() {
        let chain = FallbackChain::new(
            vec![
                Box::new(MockProvider::new("first", true)),
                Box::new(MockProvider::new("second", true)),
            ],
            true, // 使用规则引擎兜底
        );

        let request = LlmRequest::default();
        let result = chain.generate(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.provider_name, "rule_engine_fallback");
        assert!(response.raw_text.contains("rule_engine") || response.raw_text.contains("action"));
    }
}
