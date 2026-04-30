//! 本地 GGUF 推理 Provider
//!
//! 使用 llama-cpp-2 进行本地 GGUF 模型推理。
//! 此模块需要启用 `local-inference` feature。
//!
//! ## GPU 后端支持
//!
//! - Metal: macOS/iOS (系统自带)
//! - Vulkan: Windows/Linux/Android (跨平台)
//! - CUDA: Windows/Linux (NVIDIA GPU)
//! - CPU: 所有平台 (兜底)
//!
//! ## 并发安全
//!
//! llama-cpp-2 不支持同一个模型的并发推理，因此使用 Mutex 保护。

use crate::provider::LlmProvider;
use crate::types::{LlmRequest, LlmResponse, LlmError, TokenUsage};
use async_trait::async_trait;
use std::path::Path;
use std::num::NonZeroU32;
use std::sync::Mutex;

#[cfg(feature = "local-inference")]
use llama_cpp_2::llama_backend::LlamaBackend;
#[cfg(feature = "local-inference")]
use llama_cpp_2::model::{LlamaModel, AddBos};
#[cfg(feature = "local-inference")]
use llama_cpp_2::model::params::LlamaModelParams;
#[cfg(feature = "local-inference")]
use llama_cpp_2::context::params::LlamaContextParams;
#[cfg(feature = "local-inference")]
use llama_cpp_2::llama_batch::LlamaBatch;
#[cfg(feature = "local-inference")]
use llama_cpp_2::sampling::LlamaSampler;

/// GPU 后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackend {
    /// macOS/iOS Metal GPU
    Metal,
    /// Windows/Linux/Android Vulkan GPU
    Vulkan,
    /// Windows/Linux NVIDIA CUDA GPU
    Cuda,
    /// CPU 兜底
    Cpu,
}

impl GpuBackend {
    /// 获取后端名称
    pub fn name(&self) -> &'static str {
        match self {
            GpuBackend::Metal => "metal",
            GpuBackend::Vulkan => "vulkan",
            GpuBackend::Cuda => "cuda",
            GpuBackend::Cpu => "cpu",
        }
    }

    /// 获取 GPU 层数配置
    pub fn n_gpu_layers(&self) -> i32 {
        match self {
            GpuBackend::Metal | GpuBackend::Vulkan | GpuBackend::Cuda => 1000, // 全量 GPU
            GpuBackend::Cpu => 0,
        }
    }
}

/// 模型加载阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadPhase {
    /// 文件读取阶段 0-30%
    Reading,
    /// 权重解析阶段 30-70%
    Parsing,
    /// GPU 上传阶段 70-100%
    GpuUpload,
}

impl LoadPhase {
    /// 获取阶段名称
    pub fn name(&self) -> &'static str {
        match self {
            LoadPhase::Reading => "reading",
            LoadPhase::Parsing => "parsing",
            LoadPhase::GpuUpload => "gpu_upload",
        }
    }

    /// 获取进度范围
    pub fn progress_range(&self, use_gpu: bool) -> (f32, f32) {
        match self {
            LoadPhase::Reading => (0.0, 30.0),
            LoadPhase::Parsing => (30.0, if use_gpu { 70.0 } else { 100.0 }),
            LoadPhase::GpuUpload => (70.0, 100.0),
        }
    }
}

// ===== GPU 后端检测 =====

/// 检测最优 GPU 后端
///
/// 检测顺序：
/// - macOS/iOS: 直接使用 Metal
/// - Windows/Linux: CUDA → Vulkan → CPU
/// - Android: Vulkan → CPU
pub fn detect_best_backend() -> GpuBackend {
    // macOS: Metal (系统自带，无需检测)
    #[cfg(target_os = "macos")]
    {
        tracing::info!("检测到 macOS 平台，使用 Metal GPU 后端");
        return GpuBackend::Metal;
    }

    // iOS: Metal (系统自带)
    #[cfg(target_os = "ios")]
    {
        tracing::info!("检测到 iOS 平台，使用 Metal GPU 后端");
        return GpuBackend::Metal;
    }

    // Windows/Linux: CUDA → Vulkan → CPU
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        // 尝试 CUDA
        if cuda_dll_exists() {
            tracing::info!("检测到 CUDA DLL，使用 CUDA GPU 后端");
            return GpuBackend::Cuda;
        }

        // 尝试 Vulkan
        if vulkan_dll_exists() {
            tracing::info!("检测到 Vulkan DLL，使用 Vulkan GPU 后端");
            return GpuBackend::Vulkan;
        }

        tracing::warn!("未检测到 GPU DLL，使用 CPU 后端");
        return GpuBackend::Cpu;
    }

    // Android: Vulkan → CPU
    #[cfg(target_os = "android")]
    {
        if vulkan_dll_exists() {
            tracing::info!("检测到 Android Vulkan 支持，使用 Vulkan GPU 后端");
            return GpuBackend::Vulkan;
        }

        tracing::warn!("Android Vulkan 不可用，使用 CPU 后端");
        return GpuBackend::Cpu;
    }

    // 其他平台: CPU
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        tracing::info!("未知平台，使用 CPU 后端");
        return GpuBackend::Cpu;
    }
}

/// 检测 DLL 是否存在于指定路径列表
#[cfg(target_os = "windows")]
fn find_dll(dll_names: &[&str], search_paths: &[&str]) -> bool {
    for name in dll_names {
        for base in search_paths {
            let path = if base.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", base, name)
            };
            if Path::new(&path).exists() {
                return true;
            }
        }
    }
    false
}

/// 检测 CUDA DLL 是否存在
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn cuda_dll_exists() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Windows: 只检测 ggml-cuda.dll（CUDA 后端专属）
        // 不检测 llama.dll，因为它是核心库，不代表任何 GPU 后端
        let dll_names = ["ggml-cuda.dll"];
        let search_paths = [
            "",            // 当前目录
            "bin",         // bin/
            "../bin",      // 上级 bin/
            "../../client/bin", // 项目根 client/bin/（cargo test 场景）
            "client/bin",  // 从项目根运行
        ];
        return find_dll(&dll_names, &search_paths);
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: 检测 libggml-cuda.so
        let dll_names = ["libggml-cuda.so"];
        for name in dll_names {
            if Path::new(name).exists() {
                return true;
            }
        }
        false
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn cuda_dll_exists() -> bool {
    false
}

/// 检测 Vulkan DLL 是否存在
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
fn vulkan_dll_exists() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Windows: 只检测 ggml-vulkan.dll（Vulkan 后端专属）
        // 不检测 llama.dll，因为它是核心库，不代表任何 GPU 后端
        let dll_names = ["ggml-vulkan.dll"];
        let search_paths = [
            "",            // 当前目录
            "bin",         // bin/
            "../bin",      // 上级 bin/
            "../../client/bin", // 项目根 client/bin/（cargo test 场景）
            "client/bin",  // 从项目根运行
        ];
        return find_dll(&dll_names, &search_paths);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // Linux/Android: 只检测 libggml-vulkan.so
        let dll_names = ["libggml-vulkan.so"];
        for name in dll_names {
            if Path::new(name).exists() {
                return true;
            }
        }
        false
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "android")))]
fn vulkan_dll_exists() -> bool {
    false
}

// ===== LlamaProvider =====

/// 本地 Llama Provider
///
/// 通过 llama-cpp-2 加载 GGUF 模型进行本地推理。
/// 使用 Mutex 保护并发推理（llama-cpp-2 不支持同一个模型的并发调用）。
#[cfg(feature = "local-inference")]
pub struct LlamaProvider {
    /// 模型文件路径
    model_path: String,
    /// 使用的 GPU 后端
    backend: GpuBackend,
    /// llama.cpp 后端（Mutex 保护）
    llama_backend: Mutex<Option<LlamaBackend>>,
    /// 加载的模型（Mutex 保护）
    model: Mutex<Option<LlamaModel>>,
    /// 加载失败原因
    load_error: Option<String>,
}

/// 骨架 Llama Provider（feature 未启用时）
#[cfg(not(feature = "local-inference"))]
pub struct LlamaProvider {
    model_path: String,
    backend: GpuBackend,
    load_error: Option<String>,
}

impl LlamaProvider {
    /// 创建新的 Llama Provider
    ///
    /// # Arguments
    ///
    /// * `model_path` - GGUF 模型文件路径
    ///
    /// # Returns
    ///
    /// 成功返回 Provider，失败返回错误
    pub fn new(model_path: String) -> Result<Self, LlmError> {
        // 当 feature 未启用时，返回错误提示
        #[cfg(not(feature = "local-inference"))]
        {
            Err(LlmError::ProviderUnavailable(
                "local-inference feature 未启用。请使用 cargo build --features local-inference 编译。".to_string()
            ))
        }

        // 当 feature 启用时，实际初始化
        #[cfg(feature = "local-inference")]
        {
            Self::init_llama(model_path)
        }
    }

    /// 实际初始化 llama 模型（feature 启用时）
    #[cfg(feature = "local-inference")]
    fn init_llama(model_path: String) -> Result<Self, LlmError> {
        tracing::info!("LlamaProvider 初始化，模型路径: {}", model_path);

        // 1. 检查模型文件是否存在
        if !Path::new(&model_path).exists() {
            return Err(LlmError::ProviderUnavailable(
                format!("模型文件不存在: {}", model_path)
            ));
        }

        // 2. 检测最优 GPU 后端
        // 注意：需要在编译时启用 cuda/vulkan feature 才能真正使用 GPU
        // 当前仅用 local-inference feature 时，即使检测到 CUDA DLL，
        // llama.cpp 库本身不支持 GPU offload，会回退到 CPU
        let backend = if cfg!(feature = "cuda") || cfg!(feature = "vulkan") {
            detect_best_backend()
        } else {
            GpuBackend::Cpu
        };
        let n_gpu_layers = backend.n_gpu_layers();
        tracing::info!("检测到 GPU 后端: {} (n_gpu_layers={})", backend.name(), n_gpu_layers);

        // 3. 初始化 llama.cpp 后端
        let llama_backend = LlamaBackend::init().map_err(|e| {
            LlmError::ProviderUnavailable(format!("llama.cpp 后端初始化失败: {}", e))
        })?;

        // 4. 配置模型参数（GPU 加速）
        let model_params = LlamaModelParams::default()
            .with_n_gpu_layers(n_gpu_layers as u32);

        // 5. 加载 GGUF 模型
        let model = LlamaModel::load_from_file(
            &llama_backend,
            Path::new(&model_path),
            &model_params
        ).map_err(|e| {
            LlmError::ProviderUnavailable(format!("模型加载失败: {}", e))
        })?;

        let model_size = std::fs::metadata(&model_path)
            .map(|m| m.len())
            .unwrap_or(0);

        tracing::info!(
            "模型加载成功: {} 参数, {} 层, 词表大小 {}, 文件大小: {:.1} MB",
            model.n_params(),
            model.n_layer(),
            model.n_vocab(),
            model_size as f64 / 1_048_576.0
        );

        Ok(Self {
            model_path,
            backend,
            llama_backend: Mutex::new(Some(llama_backend)),
            model: Mutex::new(Some(model)),
            load_error: None,
        })
    }

    /// 获取模型路径
    pub fn get_model_path(&self) -> &str {
        &self.model_path
    }

    /// 获取 GPU 后端
    pub fn get_backend(&self) -> GpuBackend {
        self.backend
    }

    /// 检查模型文件是否存在
    pub fn model_exists(&self) -> bool {
        Path::new(&self.model_path).exists()
    }

    /// 检查是否实际可用（已加载且无错误）
    #[cfg(feature = "local-inference")]
    pub fn is_really_available(&self) -> bool {
        self.model.lock().map(|g| g.is_some()).unwrap_or(false) && self.load_error.is_none()
    }

    /// 获取加载错误（如果有）
    pub fn get_load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }
}

#[async_trait]
impl LlmProvider for LlamaProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        #[cfg(not(feature = "local-inference"))]
        {
            Err(LlmError::ConfigError(
                "local-inference feature 未启用".to_string()
            ))
        }

        #[cfg(feature = "local-inference")]
        {
            self.generate_internal(request)
        }
    }

    fn name(&self) -> &str {
        "llama_local"
    }

    fn is_available(&self) -> bool {
        // feature 启用时检查模型文件存在
        #[cfg(feature = "local-inference")]
        {
            self.model_exists()
        }

        #[cfg(not(feature = "local-inference"))]
        {
            false
        }
    }
}

#[cfg(feature = "local-inference")]
impl LlamaProvider {
    /// 内部推理实现（feature 启用时）
    /// 使用 Mutex 保护并发推理
    fn generate_internal(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // 获取 Mutex 锁，防止并发推理
        let mut model_guard = self.model.lock()
            .map_err(|_| LlmError::ProviderUnavailable("无法获取模型锁".to_string()))?;

        let model = model_guard.as_ref()
            .ok_or_else(|| LlmError::ProviderUnavailable("模型未加载".to_string()))?;

        tracing::info!(
            "LlamaProvider.generate 开始推理, 模型: {}, 后端: {}, prompt长度: {} chars",
            self.model_path,
            self.backend.name(),
            request.prompt.len()
        );

        // 获取 backend 锁
        let backend_guard = self.llama_backend.lock()
            .map_err(|_| LlmError::ProviderUnavailable("无法获取后端锁".to_string()))?;
        let llama_backend = backend_guard.as_ref()
            .ok_or_else(|| LlmError::ProviderUnavailable("后端未初始化".to_string()))?;

        // 1. 创建推理上下文
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(2048));
        let mut ctx = model.new_context(
            llama_backend,
            ctx_params
        ).map_err(|e| LlmError::ProviderUnavailable(format!("创建推理上下文失败: {}", e)))?;

        // 2. Tokenize prompt
        let tokens = model.str_to_token(&request.prompt, AddBos::Always)
            .map_err(|e| LlmError::ProviderUnavailable(format!("Tokenize失败: {}", e)))?;

        let prompt_token_count = tokens.len();
        tracing::debug!("Prompt tokenize: {} tokens", prompt_token_count);

        // 3. 创建 batch
        let max_tokens = request.max_tokens.max(1) as i32;
        let batch_capacity = (prompt_token_count as i32 + max_tokens) as usize;
        let mut batch = LlamaBatch::new(batch_capacity, 1);

        // 4. 添加 prompt tokens 到 batch
        let last_prompt_idx = (prompt_token_count - 1) as i32;
        for (i, token) in tokens.iter().enumerate() {
            let is_last = i as i32 == last_prompt_idx;
            batch.add(*token, i as i32, &[0], is_last)
                .map_err(|e| LlmError::ProviderUnavailable(format!("添加token到batch失败: {}", e)))?;
        }

        // 5. 解码 prompt
        tracing::debug!("开始解码 prompt ({} tokens)...", prompt_token_count);
        ctx.decode(&mut batch)
            .map_err(|e| LlmError::ProviderUnavailable(format!("解码prompt失败: {}", e)))?;
        tracing::debug!("Prompt 解码完成");

        // 6. 创建采样链
        let temperature = request.temperature;
        let mut sampler = if temperature > 0.0 {
            LlamaSampler::chain_simple([
                LlamaSampler::temp(temperature),
                LlamaSampler::top_k(40),
                LlamaSampler::top_p(0.95, 1),
                LlamaSampler::dist(42),
            ])
        } else {
            LlamaSampler::greedy()
        };

        // 7. 自回归生成
        let mut output_tokens = Vec::new();
        let mut n_cur = batch.n_tokens();
        let max_gen_tokens = request.max_tokens.max(1) as usize;
        tracing::debug!("开始自回归生成 (max_tokens={})...", max_gen_tokens);

        for gen_idx in 0..max_gen_tokens {
            // 采样下一个 token
            tracing::trace!("采样 token #{}...", gen_idx);
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);

            // 检查是否为结束 token
            if model.is_eog_token(token) {
                break;
            }

            sampler.accept(token);
            output_tokens.push(token);

            // 将新 token 添加到 batch
            batch.clear();
            batch.add(token, n_cur, &[0], true)
                .map_err(|e| LlmError::ProviderUnavailable(format!("添加生成token到batch失败: {}", e)))?;
            n_cur += 1;

            // 解码
            ctx.decode(&mut batch)
                .map_err(|e| LlmError::ProviderUnavailable(format!("解码失败: {}", e)))?;
        }

        // 8. Detokenize 输出
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut raw_text = String::new();
        for token in &output_tokens {
            let piece = model.token_to_piece(*token, &mut decoder, true, None)
                .unwrap_or_else(|_| "[?]".to_string());
            raw_text.push_str(&piece);
        }

        let completion_token_count = output_tokens.len();
        tracing::info!(
            "推理完成: prompt_tokens={}, completion_tokens={}, total_tokens={}",
            prompt_token_count,
            completion_token_count,
            prompt_token_count + completion_token_count
        );

        Ok(LlmResponse {
            raw_text,
            parsed_action: None,
            usage: TokenUsage {
                prompt_tokens: prompt_token_count as u32,
                completion_tokens: completion_token_count as u32,
                total_tokens: (prompt_token_count + completion_token_count) as u32,
            },
            provider_name: "llama_local".to_string(),
        })
    }
}

// ===== 辅助函数 =====

/// 获取加载进度估算
///
/// 根据模型大小估算各阶段进度（llama-cpp-2 同步加载无法获取真实进度）
pub fn get_load_progress_estimate(model_size_mb: u32, backend: GpuBackend) -> Vec<(LoadPhase, f32)> {
    let use_gpu = backend != GpuBackend::Cpu;

    let phases: Vec<LoadPhase> = if use_gpu {
        vec![LoadPhase::Reading, LoadPhase::Parsing, LoadPhase::GpuUpload]
    } else {
        vec![LoadPhase::Reading, LoadPhase::Parsing]
    };

    phases
        .into_iter()
        .map(|phase| (phase, phase.progress_range(use_gpu).0))
        .collect()
}

/// 估算加载时间（毫秒）
///
/// 基于模型大小估算总加载时间
pub fn estimate_load_time_ms(model_size_mb: u32, backend: GpuBackend) -> u32 {
    // 基准：10ms/MB (CPU), 5ms/MB (GPU)
    let rate = if backend == GpuBackend::Cpu { 10.0 } else { 5.0 };
    (model_size_mb as f32 * rate) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_backend_name() {
        assert_eq!(GpuBackend::Metal.name(), "metal");
        assert_eq!(GpuBackend::Vulkan.name(), "vulkan");
        assert_eq!(GpuBackend::Cuda.name(), "cuda");
        assert_eq!(GpuBackend::Cpu.name(), "cpu");
    }

    #[test]
    fn test_gpu_layers_config() {
        assert_eq!(GpuBackend::Metal.n_gpu_layers(), 1000);
        assert_eq!(GpuBackend::Vulkan.n_gpu_layers(), 1000);
        assert_eq!(GpuBackend::Cuda.n_gpu_layers(), 1000);
        assert_eq!(GpuBackend::Cpu.n_gpu_layers(), 0);
    }

    #[test]
    fn test_load_phase_range() {
        let reading_cpu = LoadPhase::Reading.progress_range(false);
        assert_eq!(reading_cpu, (0.0, 30.0));

        let parsing_gpu = LoadPhase::Parsing.progress_range(true);
        assert_eq!(parsing_gpu, (30.0, 70.0));

        let parsing_cpu = LoadPhase::Parsing.progress_range(false);
        assert_eq!(parsing_cpu, (30.0, 100.0));

        let upload = LoadPhase::GpuUpload.progress_range(true);
        assert_eq!(upload, (70.0, 100.0));
    }

    #[test]
    fn test_llama_provider_creation() {
        // feature 未启用时应返回错误
        #[cfg(not(feature = "local-inference"))]
        {
            let result = LlamaProvider::new("test.gguf".to_string());
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_provider_name() {
        #[cfg(feature = "local-inference")]
        {
            // 仅在 feature 启用时测试
            if let Ok(provider) = LlamaProvider::new("test.gguf".to_string()) {
                assert_eq!(provider.name(), "llama_local");
            }
        }
    }

    #[test]
    fn test_estimate_load_time() {
        let time_cpu = estimate_load_time_ms(1500, GpuBackend::Cpu);
        assert_eq!(time_cpu, 15000); // 1500 MB * 10ms

        let time_gpu = estimate_load_time_ms(1500, GpuBackend::Vulkan);
        assert_eq!(time_gpu, 7500); // 1500 MB * 5ms
    }

    #[test]
    fn test_load_progress_estimate() {
        let progress_gpu = get_load_progress_estimate(1500, GpuBackend::Vulkan);
        assert_eq!(progress_gpu.len(), 3);

        let progress_cpu = get_load_progress_estimate(1500, GpuBackend::Cpu);
        assert_eq!(progress_cpu.len(), 2);
    }
}