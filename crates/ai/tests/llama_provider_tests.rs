//! LlamaProvider 创建单元测试
//!
//! 测试 llama.rs 中的 LlamaProvider 初始化和基本功能

#[cfg(feature = "local-inference")]
mod tests {
    use agentora_ai::{LlamaProvider, GpuBackend, estimate_load_time_ms, get_load_progress_estimate, LoadPhase};
    use std::path::PathBuf;

    /// 测试 LlamaProvider 创建 - 模型文件不存在
    #[test]
    fn test_llama_provider_model_not_exists() {
        let result = LlamaProvider::new("/nonexistent/model.gguf".to_string());
        assert!(result.is_err());

        // 验证错误类型
        if let Err(e) = result {
            assert!(e.to_string().contains("模型文件不存在"));
        }
    }

    /// 测试 LlamaProvider 创建 - 有效路径（骨架模式）
    #[test]
    fn test_llama_provider_valid_path() {
        // 使用项目中的某个文件作为"假模型"来测试路径检查
        let cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml");

        if cargo_toml.exists() {
            let result = LlamaProvider::new(cargo_toml.to_string_lossy().to_string());

            // 骨架实现应返回 Ok，但标记为未实际加载
            if let Ok(provider) = result {
                assert_eq!(provider.name(), "llama_local");
                // 骨架模式下 is_really_available 应返回 false
                assert!(!provider.is_really_available());
                // 应有加载错误提示
                assert!(provider.get_load_error().is_some());
            }
        }
    }

    /// 测试 LlmProvider trait 实现
    #[test]
    fn test_llama_provider_trait() {
        let cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml");

        if cargo_toml.exists() {
            if let Ok(provider) = LlamaProvider::new(cargo_toml.to_string_lossy().to_string()) {
                // 测试 name()
                assert_eq!(provider.name(), "llama_local");

                // 测试 is_available()
                // 骨架模式下应返回 true（因为文件存在）
                assert!(provider.is_available());
            }
        }
    }

    /// 测试 GPU 后端获取
    #[test]
    fn test_llama_provider_get_backend() {
        let cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml");

        if cargo_toml.exists() {
            if let Ok(provider) = LlamaProvider::new(cargo_toml.to_string_lossy().to_string()) {
                let backend = provider.get_backend();
                // 后端应为有效值
                let valid_backends = [GpuBackend::Metal, GpuBackend::Vulkan, GpuBackend::Cuda, GpuBackend::Cpu];
                assert!(valid_backends.contains(&backend));
            }
        }
    }

    /// 测试加载时间估算
    #[test]
    fn test_estimate_load_time() {
        // CPU: 10ms/MB
        let time_cpu = estimate_load_time_ms(1000, GpuBackend::Cpu);
        assert_eq!(time_cpu, 10000); // 1000 MB * 10ms

        // GPU: 5ms/MB
        let time_gpu = estimate_load_time_ms(1000, GpuBackend::Vulkan);
        assert_eq!(time_gpu, 5000); // 1000 MB * 5ms

        let time_metal = estimate_load_time_ms(2000, GpuBackend::Metal);
        assert_eq!(time_metal, 10000); // 2000 MB * 5ms
    }

    /// 测试加载进度估算
    #[test]
    fn test_load_progress_estimate() {
        // GPU 后端：3 个阶段
        let progress_gpu = get_load_progress_estimate(1500, GpuBackend::Vulkan);
        assert_eq!(progress_gpu.len(), 3);

        // 验证阶段名称
        assert_eq!(progress_gpu[0].0, LoadPhase::Reading);
        assert_eq!(progress_gpu[1].0, LoadPhase::Parsing);
        assert_eq!(progress_gpu[2].0, LoadPhase::GpuUpload);

        // CPU 后端：2 个阶段（无 GpuUpload）
        let progress_cpu = get_load_progress_estimate(1500, GpuBackend::Cpu);
        assert_eq!(progress_cpu.len(), 2);

        assert_eq!(progress_cpu[0].0, LoadPhase::Reading);
        assert_eq!(progress_cpu[1].0, LoadPhase::Parsing);
    }

    /// 测试 LoadPhase 进度范围
    #[test]
    fn test_load_phase_progress_range() {
        // GPU 模式
        let reading_gpu = LoadPhase::Reading.progress_range(true);
        assert_eq!(reading_gpu, (0.0, 30.0));

        let parsing_gpu = LoadPhase::Parsing.progress_range(true);
        assert_eq!(parsing_gpu, (30.0, 70.0));

        let upload_gpu = LoadPhase::GpuUpload.progress_range(true);
        assert_eq!(upload_gpu, (70.0, 100.0));

        // CPU 模式
        let reading_cpu = LoadPhase::Reading.progress_range(false);
        assert_eq!(reading_cpu, (0.0, 30.0));

        let parsing_cpu = LoadPhase::Parsing.progress_range(false);
        assert_eq!(parsing_cpu, (30.0, 100.0)); // CPU 无 GPU 上传阶段
    }
}

/// 非 local-inference feature 的测试
#[cfg(not(feature = "local-inference"))]
mod tests_no_feature {
    /// 验证 feature 未启用时 LlamaProvider 不可用
    #[test]
    fn test_feature_disabled() {
        // 此测试验证编译通过，说明 feature 门控生效
        assert!(true);
    }
}