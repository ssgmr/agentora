## Why

LLM Provider 当前已有 HTTP 请求框架（openai.rs/anthropic.rs），但缺少：配置加载、完整的重试逻辑、本地 GGUF 推理集成、规则引擎兜底。这导致无法在不同 LLM 后端之间灵活切换，也无法在 API 不可用时降级到本地推理。

## What Changes

- **新增** LLM 配置加载（从 config/llm.toml 加载 Provider 配置）
- **新增** 429 限流重试逻辑（Retry-After 后重试，最多 2 次）
- **新增** 本地 GGUF Provider（mistralrs 集成）
- **新增** 规则引擎兜底（LLM 全部失败时生成安全动作）
- **修改** FallbackChain 支持动态配置 Provider 顺序

## Capabilities

### New Capabilities

- `llm-config-loader`: 从配置文件加载 Provider 配置（端点、API Key、模型、超时）
- `llm-rate-limit-retry`: 429 限流重试逻辑，Retry-After 后重试最多 2 次
- `local-gguf-provider`: 本地 GGUF 推理（mistralrs 集成），CPU/Metal 后端选择
- `rule-engine-fallback`: LLM 全部失败时的规则引擎兜底决策

### Modified Capabilities

- `fallback-chain`: 支持动态配置 Provider 顺序，从配置文件读取

## Impact

- **affected crates**: `ai` (Provider 实现), `core` (规则引擎)
- **dependencies**: `reqwest` (HTTP), `mistralrs` (本地推理), `toml` (配置解析)
- **breaking changes**: 无，当前 Provider 为框架实现
- **integration points**: DecisionPipeline 调用 FallbackChain；配置文件加载
