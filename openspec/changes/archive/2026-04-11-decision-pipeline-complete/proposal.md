## Why

当前决策管道 (`crates/core/src/decision.rs:96`) 仅有框架结构，未实现完整的五阶段决策流程。Agent 使用简化的随机决策逻辑 (`bridge/src/lib.rs:228-260`)，无法验证 LLM 驱动的自主决策是否能涌现合作、冲突和文明演进。实现完整的决策管道是 MVP 验证的核心前提。

## What Changes

- **新增** 完整五阶段决策管道实现：硬约束过滤 → 上下文构建 → LLM 生成 → 规则校验 → 动机加权选择
- **新增** `ActionCandidate` 结构体，承载 LLM 生成的候选动作（含动机对齐度自评）
- **修改** `DecisionPipeline` 从空结构体变为功能完整的决策引擎
- **修改** `RuleEngine` 增加完整的动作校验逻辑和资源/范围检查
- **修改** `PromptBuilder` 增加 token 计数和截断逻辑，确保≤2500 tokens
- **修改** `World::apply_action` 调用决策管道替代当前的简单随机决策
- **新增** LLM Provider 调用集成，支持 OpenAI/Anthropic API 和降级链

## Capabilities

### New Capabilities

- `decision-pipeline`: 五阶段决策管道完整实现，包括硬约束过滤、Prompt 构建、LLM 调用、规则校验、动机加权选择
- `llm-integration`: LLM Provider 调用集成，支持多 provider 降级链、JSON 解析、超时重试

### Modified Capabilities

- `rule-engine`: 增加完整的动作校验逻辑（资源检查、范围检查、目标存在性检查）

## Impact

- **affected crates**: `core` (决策管道、规则引擎), `ai` (LLM 调用集成)
- **dependencies**: `tokio` (异步运行时), `serde_json` (JSON 解析)
- **breaking changes**: 无，当前决策管道为空实现
- **integration points**: `World::apply_action` 需改为调用决策管道；`bridge` 需等待决策完成后发送快照
