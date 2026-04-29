# 功能规格说明 - llama-provider

## ADDED Requirements

### Requirement: GGUF 模型加载

系统 SHALL 使用 llama-cpp-2 加载 GGUF 格式模型文件，支持跨平台 GPU 加速。

#### Scenario: 加载模型成功

- **WHEN** 配置指定本地 GGUF 模型路径
- **AND** 模型文件存在
- **THEN** 系统 SHALL 使用 LlamaModel::load_from_file 加载模型
- **AND** 根据检测到的 GPU 后端配置 n_gpu_layers

#### Scenario: 模型文件不存在

- **WHEN** 配置指定本地 GGUF 模型路径
- **AND** 模型文件不存在
- **THEN** 系统 SHALL 返回 ConfigError
- **AND** 记录日志"模型文件不存在: {path}"
- **AND** 降级到规则引擎模式

#### Scenario: 模型加载内存不足

- **WHEN** 尝试加载模型
- **AND** 可用内存小于模型需求
- **THEN** 系统 SHALL 拒绝加载
- **AND** 返回 ProviderUnavailable 错误"内存不足"
- **AND** 降级到规则引擎模式

### Requirement: 推理生成响应

系统 SHALL 使用加载的模型进行文本推理，生成决策响应。

#### Scenario: 创建推理上下文

- **WHEN** 模型加载成功
- **THEN** 系统 SHALL 创建 LlamaContext
- **AND** n_ctx SHALL 设置为 4096
- **AND** n_threads SHALL 根据平台 CPU 核心数配置

#### Scenario: Prompt Tokenization

- **WHEN** 调用 generate 方法
- **THEN** 系统 SHALL 使用 model.str_to_token 将 prompt 转换为 tokens
- **AND** 使用 AddBos::Always 添加 BOS token

#### Scenario: Token 生成循环

- **WHEN** 推理开始
- **THEN** 系统 SHALL 使用 LlamaSampler.chain_simple 采样
- **AND** 采样链 SHALL 包含：temperature -> top_k(40) -> top_p(0.95) -> dist
- **AND** 生成直到遇到 EOG token 或达到 max_tokens

#### Scenario: 推理超时

- **WHEN** 推理超过配置的超时时间（默认 30 秒）
- **THEN** 系统 SHALL 取消推理
- **AND** 返回 Timeout 错误
- **AND** 降级到规则引擎

### Requirement: LlmProvider trait 实现

LlamaProvider SHALL 实现 LlmProvider trait，与现有 Provider 保持接口一致。

#### Scenario: generate 方法返回值

- **WHEN** 调用 LlamaProvider.generate(request)
- **THEN** 返回值 SHALL 为 Result<LlmResponse, LlmError>
- **AND** LlmResponse.raw_text SHALL 包含生成的文本
- **AND** LlmResponse.provider_name SHALL 返回 "llama_local"

#### Scenario: is_available 检查

- **WHEN** 调用 LlamaProvider.is_available()
- **THEN** 系统 SHALL 返回 true 如果模型已加载且内存充足
- **AND** 返回 false 如果模型未加载或内存不足

#### Scenario: JSON 输出格式

- **WHEN** LlamaProvider 用于决策推理
- **THEN** 输出 SHALL 尝试生成 JSON 格式文本
- **AND** JSON 解析失败 SHALL 调用 parse_action_json 多层降级解析

### Requirement: 推理性能要求

本地推理 SHALL 满足性能基准，确保决策延迟在可接受范围内。

#### Scenario: 骁龙 8Gen3 性能基准

- **WHEN** 使用 Qwen3.5-2B-Q4_K_M.gguf 模型
- **AND** 在骁龙 8Gen3 设备上推理（Vulkan GPU）
- **THEN** 首 token 延迟 SHALL < 100ms
- **AND** 生成 60 tokens SHALL 在 150ms 内完成

#### Scenario: 内存占用限制

- **WHEN** 2B INT4 模型加载运行
- **THEN** 内存占用 SHALL < 2GB
- **AND** 系统 SHALL 检测可用内存
- **AND** 内存不足时拒绝加载并提示用户