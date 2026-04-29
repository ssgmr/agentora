# 功能规格说明 - llm-config-loader（增量）

## MODIFIED Requirements

### Requirement: Provider 创建逻辑

系统 SHALL 根据 UserConfig.llm.mode 决定创建哪种 Provider。

#### Scenario: local 模式创建 LlamaProvider

- **WHEN** UserConfig.llm.mode = "local"
- **AND** local_model_path 存在且有效
- **THEN** 系统 SHALL 创建 LlamaProvider
- **AND** 使用 GPU 后端检测选择最优后端
- **AND** 加载指定的 GGUF 模型

#### Scenario: local 模式模型不存在降级

- **WHEN** UserConfig.llm.mode = "local"
- **AND** local_model_path 为空或文件不存在
- **THEN** 系统 SHALL 记录警告日志
- **AND** 跳过 Provider 创建
- **AND** 使用规则引擎模式

#### Scenario: remote 模式创建 OpenAiProvider

- **WHEN** UserConfig.llm.mode = "remote"
- **AND** api_endpoint 有效
- **THEN** 系统 SHALL 创建 OpenAiProvider
- **AND** 使用配置的 endpoint、token、model

#### Scenario: rule_only 模式无 Provider

- **WHEN** UserConfig.llm.mode = "rule_only"
- **THEN** 系统 SHALL 不创建任何 LLM Provider
- **AND** 决策管道 SHALL 使用规则引擎

#### Scenario: UserConfig 集成到启动流程

- **WHEN** SimulationBridge.start_simulation() 调用
- **THEN** 系统 SHALL 加载 UserConfig（如果存在）
- **AND** 将 UserConfig 传递给 create_llm_provider()

## ADDED Requirements

### Requirement: UserConfig 与 LlmConfig 合并

系统 SHALL 支持 UserConfig 覆盖 LlmConfig 的默认值。

#### Scenario: remote 模式配置覆盖

- **WHEN** UserConfig.llm.mode = "remote"
- **THEN** UserConfig.llm.api_endpoint SHALL 覆盖 LlmConfig.primary.api_base
- **AND** UserConfig.llm.api_token SHALL 覆盖 LlmConfig.primary.api_key
- **AND** UserConfig.llm.model_name SHALL 覆盖 LlmConfig.primary.model