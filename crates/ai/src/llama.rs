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

use crate::provider::LlmProvider;
use crate::types::{LlmRequest, LlmResponse, LlmError, TokenUsage};
use async_trait::async_trait;
use std::path::Path;

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

/// 检测 CUDA DLL 是否存在
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn cuda_dll_exists() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Windows: 检测 ggml-cuda.dll
        // CUDA Runtime (cudart64_*.dll) 由用户自行安装
        let dll_names = ["ggml-cuda.dll", "llama.dll"];
        for name in dll_names {
            // 尝试在当前目录和 bin 目录查找
            let paths = [
                name,
                format!("bin/{}", name),
                format!("../bin/{}", name),
            ];
            for path in &paths {
                if Path::new(path).exists() {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: 检测 libggml-cuda.so
        let dll_names = ["libggml-cuda.so", "libllama.so"];
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
        // Windows: 检测 ggml-vulkan.dll
        // Vulkan-1.dll 由系统提供
        let dll_names = ["ggml-vulkan.dll", "llama.dll"];
        for name in dll_names {
            let paths = [
                name,
                format!("bin/{}", name),
                format!("../bin/{}", name),
            ];
            for path in &paths {
                if Path::new(path).exists() {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // Linux/Android: 检测 libggml-vulkan.so 或 libllama.so
        let dll_names = ["libggml-vulkan.so", "libllama.so"];
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
pub struct LlamaProvider {
    /// 模型文件路径
    model_path: String,
    /// 使用的 GPU 后端
    backend: GpuBackend,
    /// 是否已加载
    is_loaded: bool,
    /// 加载失败原因
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
            Err(LlmError::ConfigError(
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
            return Err(LlmError::ConfigError(
                format!("模型文件不存在: {}", model_path)
            ));
        }

        // 2. 检测 GPU 后端
        let backend = detect_best_backend();
        tracing::info!("检测到 GPU 后端: {} (n_gpu_layers={})", backend.name(), backend.n_gpu_layers());

        // 3. TODO: 实现 llama-cpp-2 初始化
        // 需要:
        // - 初始化 LlamaBackend
        // - 配置模型参数（GPU 加速）
        // - 加载 GGUF 模型
        //
        // 当前仅返回骨架结构，实际实现需要编译环境和 libclang

        // 检查内存是否足够（简单估算）
        let model_size = std::fs::metadata(&model_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let estimated_memory = model_size as f64 * 1.2; // 估算内存占用约为文件大小 * 1.2

        tracing::info!(
            "模型文件大小: {:.1} MB, 估算内存占用: {:.1} MB",
            model_size as f64 / 1_048_576.0,
            estimated_memory / 1_048_576.0
        );

        // 当前返回骨架结构，实际初始化需要 llama-cpp-2 编译环境
        Ok(Self {
            model_path,
            backend,
            is_loaded: false, // 实际加载需要 llama-cpp-2 编译
            load_error: Some("llama-cpp-2 需要编译环境，当前为骨架实现".to_string()),
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
    pub fn is_really_available(&self) -> bool {
        self.is_loaded && self.load_error.is_none()
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
    fn generate_internal(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // TODO: 实现实际的 llama.cpp 推理
        // 需要:
        // 1. 创建推理上下文 (LlamaContext)
        // 2. Tokenize prompt
        // 3. 创建采样链 (Sampler chain)
        // 4. 推理生成 tokens
        // 5. Detokenize 输出

        tracing::warn!(
            "LlamaProvider.generate 骨架调用，模型: {}, 后端: {}",
            self.model_path,
            self.backend.name()
        );

        // 当前返回占位响应
        Ok(LlmResponse {
            raw_text: format!(
                "[本地推理骨架 - 模型: {}, 后端: {} - 请安装 libclang 并重新编译以启用实际推理]",
                self.model_path,
                self.backend.name()
            ),
            parsed_action: None,
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
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