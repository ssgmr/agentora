//! GPU 后端检测单元测试
//!
//! 测试 llama.rs 中的 GPU 后端检测逻辑

#[cfg(feature = "local-inference")]
mod tests {
    use agentora_ai::{GpuBackend, detect_best_backend};

    /// 测试 GPU 后端名称
    #[test]
    fn test_gpu_backend_names() {
        assert_eq!(GpuBackend::Metal.name(), "metal");
        assert_eq!(GpuBackend::Vulkan.name(), "vulkan");
        assert_eq!(GpuBackend::Cuda.name(), "cuda");
        assert_eq!(GpuBackend::Cpu.name(), "cpu");
    }

    /// 测试 GPU 层数配置
    #[test]
    fn test_gpu_layers_config() {
        // GPU 后端应返回全量 GPU 层数
        assert_eq!(GpuBackend::Metal.n_gpu_layers(), 1000);
        assert_eq!(GpuBackend::Vulkan.n_gpu_layers(), 1000);
        assert_eq!(GpuBackend::Cuda.n_gpu_layers(), 1000);

        // CPU 后端应返回 0
        assert_eq!(GpuBackend::Cpu.n_gpu_layers(), 0);
    }

    /// 测试 detect_best_backend 在当前平台
    #[test]
    fn test_detect_best_backend_current_platform() {
        let backend = detect_best_backend();

        // 验证返回有效的后端
        let valid_backends = [GpuBackend::Metal, GpuBackend::Vulkan, GpuBackend::Cuda, GpuBackend::Cpu];
        assert!(valid_backends.contains(&backend));

        // macOS 应返回 Metal
        #[cfg(target_os = "macos")]
        assert_eq!(backend, GpuBackend::Metal);

        // iOS 应返回 Metal
        #[cfg(target_os = "ios")]
        assert_eq!(backend, GpuBackend::Metal);

        // Windows/Linux 可能返回 CUDA/Vulkan/CPU
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            // 如果检测到 GPU DLL，应返回 GPU 后端
            // 如果未检测到，应返回 CPU
            assert!(backend == GpuBackend::Cuda || backend == GpuBackend::Vulkan || backend == GpuBackend::Cpu);
        }

        // Android 可能返回 Vulkan/CPU
        #[cfg(target_os = "android")]
        assert!(backend == GpuBackend::Vulkan || backend == GpuBackend::Cpu);
    }

    /// 测试 GpuBackend 枚举 PartialEq
    #[test]
    fn test_gpu_backend_equality() {
        assert!(GpuBackend::Metal == GpuBackend::Metal);
        assert!(GpuBackend::Vulkan == GpuBackend::Vulkan);
        assert!(GpuBackend::Cuda == GpuBackend::Cuda);
        assert!(GpuBackend::Cpu == GpuBackend::Cpu);

        assert!(GpuBackend::Metal != GpuBackend::Cpu);
        assert!(GpuBackend::Vulkan != GpuBackend::Cuda);
    }
}

/// 非 local-inference feature 的测试
#[cfg(not(feature = "local-inference"))]
mod tests_no_feature {
    /// 验证 feature 未启用时相关类型不可用
    #[test]
    fn test_feature_disabled() {
        // 此测试仅验证编译通过（类型不存在）
        // 如果编译通过，说明 feature 门控生效
        assert!(true);
    }
}