//! 本地 GGUF Provider (mistralrs)
//!
//! 使用 mistralrs 进行本地推理

use async_trait::async_trait;
use crate::provider::LlmProvider;
use crate::types::{LlmRequest, LlmResponse, LlmError};

/// 本地 GGUF Provider
pub struct LocalProvider {
    model_path: String,
    backend: String,
    is_loaded: bool,
}

impl LocalProvider {
    pub fn new(model_path: String, backend: String) -> Self {
        Self {
            model_path,
            backend,
            is_loaded: false,
        }
    }

    /// 加载模型
    pub fn load(&mut self) -> Result<(), LlmError> {
        tracing::info!("加载本地模型：{} (backend: {})", self.model_path, self.backend);

        // 检查模型文件是否存在
        if !std::path::Path::new(&self.model_path).exists() {
            return Err(LlmError::ProviderUnavailable(format!(
                "模型文件不存在：{}", self.model_path
            )));
        }

        // 检查内存是否足够
        if !self.check_memory() {
            return Err(LlmError::ProviderUnavailable("内存不足，无法加载模型".to_string()));
        }

        // TODO: 使用 mistralrs 初始化模型
        tracing::info!("mistralrs 初始化成功（占位）");

        self.is_loaded = true;
        Ok(())
    }

    /// 检查内存是否足够
    fn check_memory(&self) -> bool {
        // TODO: 实现内存检查
        // 暂时假设内存充足
        true
    }

    /// 推理生成
    async fn generate_inner(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // TODO: 使用 mistralrs 进行实际推理
        tracing::info!("本地推理：prompt={} tokens", request.max_tokens);

        // 占位实现
        Ok(LlmResponse {
            raw_text: "{\"action\": \"wait\", \"reasoning\": \"本地推理占位\"}".to_string(),
            parsed_action: None,
            usage: crate::types::TokenUsage::default(),
            provider_name: self.name().to_string(),
        })
    }
}

#[async_trait]
impl LlmProvider for LocalProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        if !self.is_loaded {
            return Err(LlmError::ProviderUnavailable("模型未加载".to_string()));
        }

        if !self.check_memory() {
            // OOM 降级到 API
            tracing::warn!("内存不足，建议降级到 API Provider");
            return Err(LlmError::ProviderUnavailable("内存不足 (OOM 降级)".to_string()));
        }

        self.generate_inner(request).await
    }

    fn name(&self) -> &str {
        "local_gguf"
    }

    fn is_available(&self) -> bool {
        self.is_loaded && self.check_memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_provider_creation() {
        let provider = LocalProvider::new(
            "/path/to/model.gguf".to_string(),
            "cpu".to_string(),
        );
        assert!(!provider.is_available()); // 未加载
    }

    #[test]
    fn test_local_provider_load_missing_file() {
        let mut provider = LocalProvider::new(
            "/nonexistent/model.gguf".to_string(),
            "cpu".to_string(),
        );
        let result = provider.load();
        assert!(result.is_err());
    }
}
