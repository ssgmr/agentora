# 功能规格说明 - gpu-backend-detection

## ADDED Requirements

### Requirement: 平台检测

系统 SHALL 根据运行平台自动选择最优 GPU 后端。

#### Scenario: macOS/iOS Metal 后端

- **WHEN** 在 macOS 或 iOS 平台运行
- **THEN** 系统 SHALL 直接使用 Metal 后端
- **AND** n_gpu_layers SHALL 设置为 1000（全量 GPU）
- **AND** 无需检测硬件

#### Scenario: Windows/Linux CUDA 后端

- **WHEN** 在 Windows 或 Linux 平台运行
- **AND** 检测到 NVIDIA GPU
- **AND** CUDA DLL 存在
- **THEN** 系统 SHALL 使用 CUDA 后端
- **AND** n_gpu_layers SHALL 设置为 1000

#### Scenario: Windows/Linux Vulkan 后端

- **WHEN** 在 Windows 或 Linux 平台运行
- **AND** CUDA 不可用
- **AND** Vulkan DLL 存在
- **THEN** 系统 SHALL 使用 Vulkan 后端
- **AND** n_gpu_layers SHALL 设置为 1000

#### Scenario: Android Vulkan 后端

- **WHEN** 在 Android 平台运行
- **AND** 设备支持 Vulkan
- **THEN** 系统 SHALL 使用 Vulkan 后端
- **AND** n_gpu_layers SHALL 设置为 1000

#### Scenario: CPU 兜底

- **WHEN** 所有 GPU 后端不可用
- **THEN** 系统 SHALL 使用 CPU 后端
- **AND** n_gpu_layers SHALL 设置为 0
- **AND** 记录日志"GPU 后端不可用，使用 CPU 推理"

### Requirement: DLL 存在检测

系统 SHALL 检测必要的 DLL 文件是否存在。

#### Scenario: Windows CUDA DLL 检测

- **WHEN** 在 Windows 平台
- **THEN** 系统 SHALL 检测以下 DLL 是否存在：
  - ggml-cuda.dll
  - cudart64_*.dll（CUDA Runtime）
- **AND** 如果所有 DLL 存在，返回 CUDA 可用

#### Scenario: Windows/Linux Vulkan DLL 检测

- **WHEN** 在 Windows 平台
- **THEN** 系统 SHALL 检测 ggml-vulkan.dll 是否存在
- **AND** Vulkan-1.dll 由系统提供，无需检测

- **WHEN** 在 Linux 平台
- **THEN** 系统 SHALL 检测 libggml-vulkan.so 是否存在

#### Scenario: Android Vulkan 检测

- **WHEN** 在 Android 平台
- **THEN** 系统 SHALL 检测 libggml-vulkan.so 是否存在
- **AND** 可通过 Vulkan API 查询设备支持情况

### Requirement: 后端日志输出

系统 SHALL 记录后端检测结果到日志。

#### Scenario: 检测成功日志

- **WHEN** 后端检测完成
- **THEN** 系统 SHALL 记录日志"检测到 GPU 后端: {backend}"
- **AND** 记录"n_gpu_layers: {layers}"

#### Scenario: GPU 不可用警告

- **WHEN** 所有 GPU 后端不可用
- **THEN** 系统 SHALL 记录警告日志"GPU 后端不可用，将使用 CPU 推理"
- **AND** 记录原因（DLL 缺失或硬件不支持）