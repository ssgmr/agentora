# 功能规格说明：LLM接入层

## ADDED Requirements

### Requirement: 统一Provider接口

系统 SHALL 定义统一LlmProvider trait，所有LLM后端实现同一接口：`generate(prompt, max_tokens, temperature, response_format) -> Result<LlmResponse>`。切换Provider SHALL 无需修改业务代码。

#### Scenario: 切换Provider

- **WHEN** 配置将Provider从OpenAI切换至Anthropic
- **THEN** 决策管道 SHALL 继续正常工作，无需代码改动

### Requirement: OpenAI兼容API

系统 SHALL 支持OpenAI兼容API（/v1/chat/completions），兼容Qwen/DeepSeek等厂商。支持JSON mode（response_format: json_object）。

#### Scenario: 正常API调用

- **WHEN** 发送请求至OpenAI兼容端点
- **THEN** 系统 SHALL 返回LLM生成的文本

#### Scenario: JSON mode输出

- **WHEN** 请求设置response_format为json_object
- **THEN** 系统 SHALL 尽力返回合法JSON

#### Scenario: API超时处理

- **WHEN** API调用超过10秒未返回
- **THEN** 系统 SHALL 取消请求并降级为规则引擎决策

#### Scenario: API限流处理

- **WHEN** API返回429限流错误
- **THEN** 系统 SHALL 等待Retry-After时间后重试
- **AND** 若重试2次仍失败，SHALL 降级为规则引擎

### Requirement: Anthropic API

系统 SHALL 支持Anthropic Messages API（/v1/messages），使用prefill trick（assistant: "{"）引导JSON输出。

#### Scenario: Anthropic JSON输出

- **WHEN** 向Anthropic API发送请求
- **THEN** 系统 SHALL 使用prefill trick引导JSON格式输出

#### Scenario: Anthropic API错误降级

- **WHEN** Anthropic API调用失败
- **THEN** 系统 SHALL 降级为规则引擎决策

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

### Requirement: 多层JSON兼容解析

系统 SHALL 对LLM返回的文本执行多层JSON解析：(1)serde_json直接解析→(2)提取首个{...}块→(3)修复常见错误(尾逗号/单引号/注释)→(4)全部失败则降级规则引擎。

#### Scenario: 合法JSON直接解析

- **WHEN** LLM返回合法JSON
- **THEN** 系统 SHALL 直接解析成功

#### Scenario: JSON外层有文本包裹

- **WHEN** LLM返回"这是我的决策：{...}"
- **THEN** 系统 SHALL 提取{...}部分并解析

#### Scenario: JSON含尾逗号

- **WHEN** LLM返回的JSON含非法尾逗号
- **THEN** 系统 SHALL 移除尾逗号后重新解析

#### Scenario: 完全无法解析

- **WHEN** 所有解析尝试均失败
- **THEN** 系统 SHALL 降级为规则引擎兜底决策

### Requirement: Provider自动降级链

系统 SHALL 支持配置Provider降级链，如：本地GGUF → OpenAI API → 规则引擎。当前Provider失败时自动切换下一个。

#### Scenario: 本地推理失败自动切API

- **WHEN** 本地GGUF推理失败（OOM/超时）
- **THEN** 系统 SHALL 自动切换至API Provider
- **AND** 决策 SHALL 继续执行

#### Scenario: 全部Provider失败

- **WHEN** 所有配置的Provider均失败
- **THEN** 系统 SHALL 使用规则引擎生成安全动作