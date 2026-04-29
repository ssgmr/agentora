//! Agentora AI接入层
//!
//! 统一LlmProvider trait，支持OpenAI兼容API、Anthropic API、本地GGUF推理。

pub mod provider;
pub mod types;
pub mod openai;
pub mod anthropic;
pub mod parser;
pub mod fallback;
pub mod local;
pub mod config;
pub mod retry;
pub mod model_downloader;

#[cfg(feature = "local-inference")]
pub mod llama;

pub use provider::LlmProvider;
pub use types::{LlmRequest, LlmResponse, LlmError, ResponseFormat};
pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;
pub use fallback::FallbackChain;
pub use parser::{parse_action_json, ParseError};
pub use config::{LlmConfig, load_llm_config};
pub use retry::{RetryProvider, is_rate_limit_status, parse_retry_after};
pub use model_downloader::{ModelDownloader, DownloadProgress, DownloadError, ModelEntry, get_available_models};

#[cfg(feature = "local-inference")]
pub use llama::{LlamaProvider, GpuBackend, LoadPhase, detect_best_backend, get_load_progress_estimate, estimate_load_time_ms};