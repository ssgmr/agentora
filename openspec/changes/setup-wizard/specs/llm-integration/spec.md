# 功能规格说明 - LLM Integration（增量）

## ADDED Requirements

### Requirement: llama-cpp-rs Provider 集成

LlmProvider trait 的实现 SHALL 新增 LlamaProvider，通过 llama-cpp-rs bindings 支持本地 GGUF 推理。

#### Scenario: LlamaProvider 初始化

- **WHEN** 配置指定 mode = "local" 且 local_model_path 有效
- **THEN** FallbackChain SHALL 包含 LlamaProvider
- **AND** LlamaProvider SHALL 加载 GGUF 模型文件
- **AND** 根据平台启用 GPU 加速（Metal/Vulkan）

#### Scenario: 本地推理 Provider 名称

- **WHEN** LlamaProvider 实现完成
- **THEN** name() SHALL 返回 "llama_local"
- **AND** 与现有 OpenAI/Anthropic Provider 名称区分

### Requirement: 本地推理与 API 降级

本地推理失败时 SHALL 自动降级到 API Provider。

#### Scenario: OOM 降级

- **WHEN** 本地推理时内存不足
- **THEN** 系统 SHALL 捕获 OOM 错误
- **AND** 自动切换到 FallbackChain 中的下一个 Provider
- **AND** 记录降级事件到日志

#### Scenario: 推理超时降级

- **WHEN** 本地推理超过 30 秒未完成
- **THEN** 系统 SHALL 取消推理
- **AND** 降级到 API Provider
- **AND** 规则引擎作为最终兜底

### Requirement: 模型下载进度信号

SimulationBridge SHALL 新增模型下载进度信号，供 Godot UI 显示。

#### Scenario: download_progress 信号

- **WHEN** 模型下载进行中
- **THEN** Bridge SHALL 定期发射 download_progress 信号
- **AND** 信号参数 SHALL 包含：
  - downloaded_mb: 已下载量（MB）
  - total_mb: 总大小（MB）
  - speed_mbps: 当前速度（MB/s）

#### Scenario: model_download_complete 信号

- **WHEN** 模型下载完成
- **THEN** Bridge SHALL 发射 model_download_complete 信号
- **AND** 参数 SHALL 包含模型文件路径

#### Scenario: model_download_failed 信号

- **WHEN** 模型下载失败
- **THEN** Bridge SHALL 发射 model_download_failed 信号
- **AND** 参数 SHALL 包含错误描述字符串