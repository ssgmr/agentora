# 功能规格说明：本地 GGUF Provider

## ADDED Requirements

### Requirement: mistralrs 集成

系统 SHALL 使用 mistralrs crate 加载 GGUF 模型进行本地推理。

#### Scenario: 模型加载

- **WHEN** 初始化本地 Provider
- **THEN** 系统 SHALL 使用 mistralrs 加载 GGUF 模型
- **AND** 支持 CPU/Metal/CUDA 后端选择

#### Scenario: 内存检查

- **WHEN** 加载模型前
- **THEN** 系统 SHALL 检查可用内存
- **AND** 内存不足时 SHALL 回退到 API Provider

### Requirement: 本地推理

系统 SHALL 使用加载的模型进行推理。

#### Scenario: 生成响应

- **WHEN** 调用 generate 方法
- **THEN** 系统 SHALL 使用 mistralrs 进行推理
- **AND** 返回结构化 JSON 格式

#### Scenario: 超时处理

- **WHEN** 推理超过 30 秒
- **THEN** 系统 SHALL 取消推理
- **AND** 返回 Timeout 错误

### Requirement: 降级到 API

系统 SHALL 在本地推理失败时降级到 API Provider。

#### Scenario: OOM 降级

- **WHEN** 本地推理返回 OOM 错误
- **THEN** 系统 SHALL 卸载模型释放内存
- **AND** 调用 API Provider 重试

## REMOVED Requirements

无
