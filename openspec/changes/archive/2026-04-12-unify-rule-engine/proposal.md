# 提案：统一规则引擎

## 背景概述

当前项目中规则引擎相关逻辑存在**三处**独立实现：

1. **`crates/core/src/rule_engine.rs`** — `RuleEngine` 结构体，提供完整的硬约束过滤、动作校验、6 维动机规则决策和 fallback。被 `DecisionPipeline` 使用。
2. **`crates/bridge/src/lib.rs`** — `npc_rule_decision()` 函数，NPC 的规则决策逻辑已迁移到调用 `core::RuleEngine::rule_decision()`。
3. **`crates/ai/src/rule_engine.rs`** — 简陋版规则引擎，仅有 3 种动作（Wait/Move/Explore），被 `FallbackChain` 在所有 LLM Provider 失败时用作兜底。

问题在于第 3 处：`ai::rule_engine` 是 `core::RuleEngine` 的子集能力，且因为 ai crate 不能依赖 core（当前依赖关系为 `core → ai`），它只能用最简化的类型和逻辑。这导致：

- **冗余代码**：两套规则引擎维护相同的动机→动作映射逻辑，但 ai 版只有 3 种动作且硬编码位置 `(0,0)` 和默认动机 `[0.5; 6]`，实际无意义
- **职责混乱**：ai crate 定位是"LLM 接入层"，不应该包含领域逻辑（规则决策）
- **fallback 路径重叠**：`FallbackChain` 的简陋 fallback 生成的 JSON 响应被 `DecisionPipeline` 再次解析后处理，而 `DecisionPipeline` 自身已有完整的 `core::RuleEngine::fallback_action()` 兜底路径

## 变更目标

- **删除** `crates/ai/src/rule_engine.rs`，消除冗余
- **简化** `FallbackChain`：移除 `use_rule_engine_fallback` 参数和规则引擎兜底逻辑，所有 LLM 失败时返回错误，由上层 `DecisionPipeline` 统一通过 `core::RuleEngine` 做规则兜底
- **保持** `core → ai` 的单向依赖不变
- **保持** `core::RuleEngine` 作为唯一的规则决策和兜底入口

## 功能范围

### 删除功能

| 功能标识 | 功能描述 |
| --- | --- |
| `ai-rule-engine` | 删除 `crates/ai/src/rule_engine.rs` 及其导出 |
| `ai-fallback-chain-param` | 移除 `FallbackChain` 的 `use_rule_engine_fallback` 参数和 `generate_rule_engine_fallback()` 方法 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `fallback-chain` | `FallbackChain` 所有 Provider 失败时直接返回 `ProviderUnavailable` 错误，不再自行兜底 |
| `bridge-create-llm` | bridge 创建 `FallbackChain` 时不再传入 `use_rule_engine_fallback` 参数 |
| `claude-md` | 更新 CLAUDE.md 中关于 `SimpleActionType` 的过时描述 |

### 保持不变

| 功能标识 | 说明 |
| --- | --- |
| `core-rule-engine` | `crates/core/src/rule_engine.rs` 完整保留，包括 `rule_decision()`、`fallback_action()` 等 |
| `decision-pipeline` | `DecisionPipeline` 已有的 LLM 失败 → `core::RuleEngine::fallback_action()` 路径不变 |

## 影响范围

- **代码模块**：
  - `crates/ai/src/rule_engine.rs` — **删除整个文件**
  - `crates/ai/src/lib.rs` — 移除 `rule_engine` 模块声明和 re-export
  - `crates/ai/src/fallback.rs` — 简化 `FallbackChain`，删除规则引擎兜底相关代码和测试
  - `crates/bridge/src/lib.rs` — 更新 `FallbackChain::new()` 调用
  - `CLAUDE.md` — 更新架构描述
- **API 接口**：`FallbackChain::new()` 签名变更（减少一个参数）
- **依赖组件**：无变化，保持 `core → ai` 单向依赖
- **关联系统**：LLM 失败降级流程（统一到 core 层，行为更完善）

## 验收标准

- [ ] `crates/ai/src/rule_engine.rs` 文件不存在
- [ ] `crates/ai/src/lib.rs` 不再导出 `rule_engine` 模块
- [ ] `FallbackChain` 不再有 `use_rule_engine_fallback` 字段
- [ ] `cargo build` 编译通过
- [ ] `cargo test` 全部通过
- [ ] LLM 失败时 Player Agent 仍可通过 `core::RuleEngine::fallback_action()` 获得有意义的 6 维动机驱动兜底动作
