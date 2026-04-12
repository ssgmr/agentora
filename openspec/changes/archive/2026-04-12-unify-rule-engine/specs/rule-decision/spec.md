# 需求说明书

## 背景概述

当前项目中规则引擎相关逻辑存在**三处**独立实现：`crates/core/src/rule_engine.rs` 中的 `RuleEngine` 负责 Player Agent 决策管道和 NPC 规则决策，而 `crates/ai/src/rule_engine.rs` 中的简陋版规则引擎被 `FallbackChain` 在所有 LLM Provider 失败时用作兜底。ai 版仅有 3 种动作（Wait/Move/Explore），硬编码位置和动机值，与 core 版的完整能力重叠且能力受限。同时 `DecisionPipeline` 已有 `core::RuleEngine::fallback_action()` 作为完整兜底路径，导致两条 fallback 路径共存、职责混乱。

## 变更目标

- **删除** `crates/ai/src/rule_engine.rs`，消除冗余代码
- **简化** `FallbackChain`：移除规则引擎兜底逻辑，LLM 全部失败时返回错误
- **保持** `core::RuleEngine` 作为唯一的规则决策和兜底入口（不变）
- **保持** `core → ai` 单向依赖不变

## 功能范围

### 删除功能

| 功能标识 | 功能描述 |
| --- | --- |
| `ai-rule-engine-removal` | 删除 `crates/ai/src/rule_engine.rs` 整个文件及所有 re-export |
| `ai-fallback-removal` | 从 `FallbackChain` 移除 `use_rule_engine_fallback` 参数和 `generate_rule_engine_fallback()` 方法 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `fallback-chain-api` | `FallbackChain::new()` 签名变更：移除 `use_rule_engine_fallback: bool` 参数 |

### 保持不变

| 功能标识 | 说明 |
| --- | --- |
| `core-rule-engine` | `crates/core/src/rule_engine.rs` 完整保留，所有方法不变 |
| `decision-pipeline-fallback` | `DecisionPipeline` 已有的 `core::RuleEngine::fallback_action()` 兜底路径不变 |

## 影响范围

- **代码模块**：
  - `crates/ai/src/rule_engine.rs` — **删除整个文件**
  - `crates/ai/src/lib.rs` — 移除 `pub mod rule_engine` 和 `pub use rule_engine::...`
  - `crates/ai/src/fallback.rs` — 简化 `FallbackChain`，删除规则引擎相关代码和测试
  - `crates/bridge/src/lib.rs` — 更新 `FallbackChain::new()` 调用参数
  - `CLAUDE.md` — 更新关于 `SimpleActionType` 的过时描述
- **API 接口**：`FallbackChain::new()` 签名变更
- **依赖组件**：无变化

## ADDED Requirements

### Requirement: ai crate 不包含规则引擎逻辑

ai crate SHALL 仅关注 LLM 接入层职责，不包含任何规则决策、动作映射或动机相关的领域逻辑。

#### Scenario: ai::rule_engine 模块不存在

- **WHEN** 查看 `crates/ai/src/` 目录
- **THEN** 系统 SHALL 不包含 `rule_engine.rs` 文件
- **AND** `lib.rs` SHALL 不声明 `pub mod rule_engine`
- **AND** `lib.rs` SHALL 不 re-export `FallbackAction`、`SimpleActionType`、`SimplePosition`、`fallback_decision`

### Requirement: FallbackChain 不执行规则兜底

`FallbackChain` SHALL 仅负责多 Provider 降级尝试，所有 Provider 失败时返回错误，不自行生成兜底动作。

#### Scenario: 所有 Provider 失败返回错误

- **WHEN** `FallbackChain` 中所有 Provider 的 `generate()` 调用均失败
- **THEN** 系统 SHALL 返回 `LlmError::ProviderUnavailable` 错误
- **AND** 错误信息 SHALL 包含 "所有 Provider 都失败"
- **AND** 系统 SHALL 不调用任何规则引擎生成兜底动作

#### Scenario: FallbackChain 不包含规则引擎字段

- **WHEN** 查看 `FallbackChain` 结构体定义
- **THEN** 系统 SHALL 不包含 `use_rule_engine_fallback` 字段
- **AND** 系统 SHALL 不包含 `generate_rule_engine_fallback()` 方法

#### Scenario: FallbackChain 构造函数简化

- **WHEN** 调用 `FallbackChain::new()`
- **THEN** 参数 SHALL 仅包含 `Vec<Box<dyn LlmProvider>>`
- **AND** 参数 SHALL 不包含 `use_rule_engine_fallback: bool`

### Requirement: core::RuleEngine 作为唯一兜底入口

`core::RuleEngine` SHALL 保持作为唯一的规则决策和 LLM 失败兜底入口，其所有方法 SHALL 保持不变。

#### Scenario: DecisionPipeline 使用 core::RuleEngine 兜底

- **WHEN** `DecisionPipeline::execute()` 中 LLM 调用失败
- **THEN** 系统 SHALL 调用 `core::RuleEngine::fallback_action()` 获取兜底动作
- **AND** 兜底动作 SHALL 基于 Agent 当前 6 维动机状态驱动
- **AND** 兜底动作 SHALL 支持完整 13+ 种 ActionType

## MODIFIED Requirements

### Requirement: bridge 创建 FallbackChain

bridge 层创建 `FallbackChain` 时 SHALL 不再传入 `use_rule_engine_fallback` 参数。

#### Scenario: bridge 创建 FallbackChain

- **WHEN** bridge 的 `create_llm_provider()` 创建 Provider 链
- **THEN** 系统 SHALL 使用 `FallbackChain::new(vec![Box::new(openai)])`
- **AND** 系统 SHALL 不传入第二个 bool 参数

## REMOVED Requirements

### Requirement: ai::rule_engine 兜底动作

`FallbackChain` 原有的使用 `ai::rule_engine::fallback_decision()` 生成简陋 JSON 兜底响应的逻辑 SHALL 被完全移除。
