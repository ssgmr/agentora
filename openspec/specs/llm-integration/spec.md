# LLM Integration

## Purpose

定义 LLM Provider 统一接口、调用链、多层 JSON 兼容解析、Provider 降级链、本地 GGUF 推理和规则引擎兜底决策。

## Requirements

### Requirement: 统一Provider接口

系统 SHALL 定义统一LlmProvider trait，所有LLM后端实现同一接口：`generate(prompt, max_tokens, temperature, response_format) -> Result<LlmResponse>`。切换Provider SHALL 无需修改业务代码。

#### Scenario: 切换Provider

- **WHEN** 配置将Provider从OpenAI切换至Anthropic
- **THEN** 决策管道 SHALL 继续正常工作，无需代码改动

### Requirement: LLM Provider 调用链

系统 SHALL 实现完整的 LLM Provider 调用链，支持 OpenAI 兼容 API、Anthropic API 和本地 GGUF 推理。

#### Scenario: Provider 配置加载

- **WHEN** 系统启动时
- **THEN** 系统 SHALL 从配置文件加载 Provider 列表和 API Key
- **AND** 配置 SHALL 包含：端点 URL、API Key、模型名称、超时时间

#### Scenario: OpenAI Provider 调用

- **WHEN** 调用 OpenAI 兼容 Provider
- **THEN** 系统 SHALL 发送 POST 请求到 `/v1/chat/completions`
- **AND** 请求 SHALL 包含：messages 数组、max_tokens、temperature、response_format
- **AND** response_format SHALL 设置为 JSON mode

#### Scenario: Anthropic Provider 调用

- **WHEN** 调用 Anthropic Provider
- **THEN** 系统 SHALL 发送 POST 请求到 `/v1/messages`
- **AND** 请求 SHALL 包含：messages 数组、max_tokens、temperature
- **AND** 请求头 SHALL 包含：x-api-key、anthropic-version

#### Scenario: 超时处理

- **WHEN** LLM 请求超过 10 秒无响应
- **THEN** 系统 SHALL 取消请求并返回 Timeout 错误
- **AND** 系统 SHALL 尝试降级到下一个 Provider

#### Scenario: 限流重试

- **WHEN** Provider 返回 429 Too Many Requests
- **THEN** 系统 SHALL 等待 Retry-After 指定的时间后重试
- **AND** 重试次数 SHALL 不超过 2 次
- **AND** 仍失败 SHALL 降级到下一个 Provider

### Requirement: 多层 JSON 兼容解析

系统 SHALL 实现多层降级 JSON 解析，处理 LLM 输出的非标准 JSON 格式。

#### Scenario: Layer 1 直接解析

- **WHEN** LLM 返回标准 JSON
- **THEN** 系统 SHALL 使用 serde_json 直接解析
- **AND** 解析成功 SHALL 返回 ActionCandidate 列表

#### Scenario: Layer 2 提取 JSON 块

- **WHEN** Layer 1 解析失败（文本包含额外内容）
- **THEN** 系统 SHALL 使用正则提取第一个{...}块
- **AND** 提取后 SHALL 重试解析

#### Scenario: Layer 3 修复常见错误

- **WHEN** Layer 2 解析失败（JSON 格式错误）
- **THEN** 系统 SHALL 尝试修复：
  - 移除尾随逗号
  - 单引号替换为双引号
  - 移除 JavaScript 风格注释
- **AND** 修复后 SHALL 重试解析

#### Scenario: 全部失败降级

- **WHEN** Layer 1/2/3 全部解析失败
- **THEN** 系统 SHALL 返回 ParseError
- **AND** 调用规则引擎生成兜底动作

### Requirement: 本地GGUF推理

系统 SHALL 支持加载GGUF格式模型进行本地推理，作为离线/低延迟/零成本的LLM后端。支持CPU和Metal(CUDA)后端。

#### Scenario: 加载本地模型

- **WHEN** 配置指定本地GGUF模型路径
- **THEN** 系统 SHALL 加载模型并可用于推理

#### Scenario: 本地推理速度

- **WHEN** 使用2B参数INT4量化模型在骁龙8Gen3/M2级别设备上推理
- **THEN** 首token延迟 SHALL < 500ms
- **AND** 生成60 tokens SHALL 在5秒内完成

#### Scenario: 本地推理内存

- **WHEN** 2B INT4模型加载运行
- **THEN** 内存占用 SHALL < 2GB

#### Scenario: 本地模型加载失败

- **WHEN** 指定的模型文件不存在或格式错误
- **THEN** 系统 SHALL 回退至API模式
- **AND** 记录加载失败日志

### Requirement: Provider 降级链

系统 SHALL 实现 Provider 降级链，当前 Provider 失败时自动切换到下一个。

#### Scenario: 降级链配置

- **WHEN** 配置 Provider 列表
- **THEN** 系统 SHALL 按优先级排序：OpenAI → Anthropic → 本地 GGUF
- **AND** 配置 SHALL 支持禁用某些 Provider

#### Scenario: 自动切换

- **WHEN** 当前 Provider 返回错误（超时/429/5xx/解析失败）
- **THEN** 系统 SHALL 自动尝试列表中的下一个 Provider
- **AND** 已尝试的 Provider SHALL 不再重复尝试

#### Scenario: 本地推理失败自动切API

- **WHEN** 本地GGUF推理失败（OOM/超时）
- **THEN** 系统 SHALL 自动切换至API Provider
- **AND** 决策 SHALL 继续执行

#### Scenario: 全部失败处理

- **WHEN** 所有 Provider 都失败
- **THEN** 系统 SHALL 返回 FallbackError
- **AND** 调用规则引擎生成兜底动作
- **AND** 记录错误日志用于后续分析

### Requirement: 规则引擎兜底决策

系统 SHALL 在 LLM 全部失败时生成安全的兜底动作。

#### Scenario: 资源压力兜底

- **WHEN** LLM 失败且 Agent 存在资源缺口
- **THEN** 系统 SHALL 生成"向最近资源格移动"的动作
- **AND** 资源类型优先级：食物 > 木材 > 铁矿 > 石材

#### Scenario: 无压力兜底

- **WHEN** LLM 失败且 Agent 无明显资源缺口
- **THEN** 系统 SHALL 生成"原地等待"的动作
- **AND** 等待动作 SHALL 不消耗资源

#### Scenario: 兜底动作记录

- **WHEN** 执行兜底动作
- **THEN** 系统 SHALL 记录到日志：
  - LLM 失败原因
  - 选择的兜底动作类型
  - Agent 当前状态（动机向量、位置）
