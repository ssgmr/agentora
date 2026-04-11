# 功能规格说明：LLM 配置加载

## ADDED Requirements

### Requirement: 配置文件格式

系统 SHALL 支持从 config/llm.toml 加载 Provider 配置。

#### Scenario: TOML 配置结构

- **WHEN** 创建配置文件
- **THEN** 配置 SHALL 包含以下字段:
  - `providers.openai`: api_base, api_key, model, timeout, enabled
  - `providers.anthropic`: api_key, model, timeout, enabled
  - `providers.local`: model_path, backend, enabled

### Requirement: 配置加载器

系统 SHALL 实现配置加载器，解析 TOML 文件。

#### Scenario: 加载配置

- **WHEN** 系统启动时
- **THEN** 系统 SHALL 从 config/llm.toml 加载配置
- **AND** 文件不存在时 SHALL 返回默认配置

#### Scenario: 环境变量覆盖

- **WHEN** 环境变量设置 `LLM_API_KEY`
- **THEN** 系统 SHALL 使用环境变量覆盖配置文件中的 api_key

### Requirement: Provider 工厂

系统 SHALL 根据配置创建 Provider 实例。

#### Scenario: 创建 Provider 实例

- **WHEN** 配置加载完成
- **THEN** 系统 SHALL 按配置创建 Provider 实例
- **AND** enabled=false 的 Provider SHALL 不创建

## REMOVED Requirements

无
