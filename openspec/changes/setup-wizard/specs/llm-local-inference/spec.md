# 功能规格说明 - 本地 GGUF 推理引擎

## ADDED Requirements

### Requirement: llama-cpp-rs Provider 实现

系统 SHALL 实现 LlamaProvider，通过 llama-cpp-rs bindings 加载 GGUF 格式模型进行本地推理，支持跨平台 GPU 加速。

#### Scenario: 加载 GGUF 模型

- **WHEN** 配置指定本地 GGUF 模型路径
- **THEN** 系统 SHALL 使用 LlamaModel::load_from_file 加载模型
- **AND** 根据平台自动启用 GPU 加速（Metal/Vulkan/CUDA）

#### Scenario: iOS Metal GPU 加速

- **WHEN** 在 iOS 设备上运行
- **THEN** LlamaModelParams SHALL 设置 n_gpu_layers=1000
- **AND** 所有模型层 SHALL 通过 Metal GPU 推理

#### Scenario: Android Vulkan GPU 加速

- **WHEN** 在 Android 设备上运行
- **THEN** LlamaModelParams SHALL 设置 n_gpu_layers=1000
- **AND** 所有模型层 SHALL 通过 Vulkan GPU 推理

#### Scenario: 创建推理上下文

- **WHEN** 模型加载成功
- **THEN** 系统 SHALL 创建 LlamaContext
- **AND** n_ctx SHALL 设置为 4096（决策上下文长度）
- **AND** n_threads SHALL 根据设备 CPU 核心数自动配置

#### Scenario: Token 生成

- **WHEN** 调用 LlamaProvider.generate()
- **THEN** 系统 SHALL 使用 LlamaSampler.chain_simple 进行采样
- **AND** 采样链 SHALL 包含：temperature -> top_k -> top_p -> dist
- **AND** 生成的 tokens SHALL 通过 model.token_to_piece 解码为文本

### Requirement: 推理性能指标

本地推理 SHALL 满足移动端性能要求，确保决策延迟在可接受范围内。

#### Scenario: 骁龙 8Gen3 性能基准

- **WHEN** 使用 Qwen3.5-2B-Q4_K_M.gguf 模型在骁龙 8Gen3 设备上推理
- **THEN** 首 token 延迟 SHALL < 100ms
- **AND** 生成 60 tokens SHALL 在 150ms 内完成（用于决策）

#### Scenario: 内存占用限制

- **WHEN** 2B INT4 模型加载运行
- **THEN** 内存占用 SHALL < 2GB
- **AND** 系统 SHALL 检测可用内存，不足时拒绝加载并提示用户

#### Scenario: 内存不足降级

- **WHEN** 本地推理时检测到内存不足（OOM）
- **THEN** 系统 SHALL 自动切换至远程 API Provider
- **AND** 记录 OOM 事件到日志
- **AND** 向用户显示提示"内存不足，已切换至远程 API"

### Requirement: Provider 接口兼容

LlamaProvider SHALL 实现 LlmProvider trait，与现有 Provider 保持接口一致。

#### Scenario: generate 方法实现

- **WHEN** 调用 LlamaProvider.generate(request)
- **THEN** 返回值 SHALL 为 Result<LlmResponse, LlmError>
- **AND** LlmResponse.raw_text SHALL 包含生成的文本
- **AND** LlmResponse.provider_name SHALL 返回 "llama_local"

#### Scenario: JSON 输出格式

- **WHEN** LlamaProvider 用于决策推理
- **THEN** 输出 SHALL 尝试生成 JSON 格式文本
- **AND** JSON 解析失败 SHALL 调用 parse_action_json 多层降级解析

#### Scenario: Provider 名称标识

- **WHEN** 查询 Provider 信息
- **THEN** name() SHALL 返回 "llama_local"
- **AND** is_available() SHALL 返回模型是否已加载且内存充足

### Requirement: 跨平台编译配置

Cargo.toml SHALL 配置 llama-cpp-rs 的平台特定 feature，支持不同 GPU 后端。

#### Scenario: iOS 编译

- **WHEN** 为 iOS 编译
- **THEN** SHALL 启用 feature: metal
- **AND** target SHALL 为 aarch64-apple-ios

#### Scenario: Android 编译

- **WHEN** 为 Android 编译
- **THEN** SHALL 启用 feature: vulkan
- **AND** target SHALL 为 aarch64-linux-android

#### Scenario: macOS 编译

- **WHEN** 为 macOS 编译
- **THEN** SHALL 启用 feature: metal
- **AND** target SHALL 为 aarch64-apple-darwin 或 x86_64-apple-darwin

#### Scenario: Windows/Linux 编译

- **WHEN** 为 Windows 或 Linux 编译
- **THEN** SHALL 启用 feature: vulkan
- **AND** 可选启用 feature: cuda（如果有 NVIDIA GPU）