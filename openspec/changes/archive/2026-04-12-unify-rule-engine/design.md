# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

项目存在两套规则引擎实现：

- **`crates/core/src/rule_engine.rs`** — 完整版 `RuleEngine`，提供硬约束过滤、动作校验、6 维动机规则决策和 fallback。被 `DecisionPipeline` 和 bridge 层 NPC 决策使用。
- **`crates/ai/src/rule_engine.rs`** — 简陋版，仅 3 种动作（Wait/Move/Explore），被 `FallbackChain` 在所有 LLM Provider 失败时用作兜底。

### 1.2 现状分析

当前存在三处问题：

1. **代码冗余**：两套规则引擎维护相同的动机→动作映射逻辑，ai 版能力受限（仅 3 种动作）
2. **无效兜底**：`FallbackChain` 的规则引擎兜底硬编码位置 `(0,0)` 和动机 `[0.5; 6]`，无法反映 Agent 真实状态
3. **职责混乱**：ai crate 定位是 "LLM 接入层"，不应包含领域逻辑（规则决策）；且 fallback 路径与 `DecisionPipeline` 已有的完整兜底路径重叠

### 1.3 依赖关系

```
当前:  ai ← core    (core 依赖 ai，ai 是底层)
       sync ← core, network
       bridge ← core, ai
```

关键约束：**ai 不能依赖 core**，否则破坏现有分层。

### 1.4 关键干系人

- `FallbackChain`（ai crate）：当前使用 `ai::rule_engine` 做兜底
- `DecisionPipeline`（core crate）：已有完整的 `core::RuleEngine::fallback_action()` 路径
- NPC 循环（bridge 层）：已使用 `core::RuleEngine::rule_decision()`
- `RuleEngine`（core crate）：完整的规则决策和兜底能力

## 2. 设计目标

### 目标

- 删除 `ai::rule_engine`，消除冗余
- 简化 `FallbackChain`：LLM 全部失败时返回错误，不自行兜底
- 统一由 `core::RuleEngine` 作为唯一的规则决策和兜底入口
- 保持 `core → ai` 单向依赖不变

### 非目标

- 不修改 `core::RuleEngine` 的任何逻辑
- 不改变 NPC 的决策频率或调度方式
- 不引入新的 crate 或外部依赖
- 不修改 `ActionType` 枚举定义

## 3. 整体架构

### 3.1 架构对比

```
变更前:
┌───────────────────────────────────────────────────────────────┐
│                       LLM 调用链                               │
│                                                                │
│  FallbackChain (ai)               DecisionPipeline (core)     │
│  ┌────────────────────┐           ┌──────────────────────┐    │
│  │ Provider 1         │──成功──▶   │                      │    │
│  │ Provider 2         │           │  LLM结果 → 校验→选择  │    │
│  │                    │           │                      │    │
│  │ 全部失败 + flag?   │──简单兜底──▶│  解析JSON → 再处理    │    │
│  │  ai::rule_engine   │  (JSON)   │  (能力受限)          │    │
│  │  3种动作,硬编码    │           │                      │    │
│  └────────────────────┘           │  LLM失败 → 完整兜底   │    │
│                                     │  core::RuleEngine     │    │
│                                     │  (6维动机,完整动作)    │    │
│                                     └──────────────────────┘    │
│                                                                  │
│  问题: 两条兜底路径共存，ai 版能力弱且数据无效                     │
└───────────────────────────────────────────────────────────────┘

变更后:
┌───────────────────────────────────────────────────────────────┐
│                       LLM 调用链                               │
│                                                                │
│  FallbackChain (ai)               DecisionPipeline (core)     │
│  ┌────────────────────┐           ┌──────────────────────┐    │
│  │ Provider 1         │──成功──▶   │                      │    │
│  │ Provider 2         │           │  LLM结果 → 校验→选择  │    │
│  │                    │           │                      │    │
│  │ 全部失败 → 返回错误 │──错误──▶   │  捕获错误 → 完整兜底   │    │
│  │  (不再自行兜底)     │           │  core::RuleEngine     │    │
│  └────────────────────┘           │  (6维动机,完整动作)    │    │
│                                     └──────────────────────┘    │
│                                                                  │
│  优势: 单一兜底路径，职责清晰，能力完整                            │
└───────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件变更

| 组件 | 变更 | 说明 |
| --- | --- | --- |
| `ai::rule_engine` 模块 | **删除** | 整个文件移除 |
| `FallbackChain` | **简化** | 移除 `use_rule_engine_fallback` 和 `generate_rule_engine_fallback()` |
| `core::RuleEngine` | **不变** | 已有的完整能力保留 |
| `DecisionPipeline` | **不变** | 已有的 fallback 路径不变 |

### 3.3 数据流设计

**LLM 失败降级流程（变更后）**:

```
Provider 1 失败
  │
  ▼
Provider 2 失败
  │
  ▼
FallbackChain: 返回 ProviderUnavailable 错误
  │
  ▼
DecisionPipeline::call_llm(): 捕获错误
  │
  ▼
DecisionPipeline::execute(): LLM 分支失败
  │
  ▼
core::RuleEngine::fallback_action(motivation, world_state)
  │
  ▼（委托）
core::RuleEngine::rule_decision(motivation, world_state)
  │
  ├─ 计算 6 维动机值
  ├─ 找出最高动机维度（平局用位置哈希打破）
  ├─ 查表获取对应动作、理由、动机变化
  └─ 返回 Action → 转换为 ActionCandidate
  │
  ▼
返回有意义的动机驱动兜底动作
```

## 4. 详细设计

### 4.1 FallbackChain 简化

**变更前签名：**
```rust
pub struct FallbackChain {
    providers: Vec<Box<dyn LlmProvider>>,
    use_rule_engine_fallback: bool,  // ← 删除
}

impl FallbackChain {
    pub fn new(providers: Vec<Box<dyn LlmProvider>>, use_rule_engine_fallback: bool) -> Self
}
```

**变更后签名：**
```rust
pub struct FallbackChain {
    providers: Vec<Box<dyn LlmProvider>>,
}

impl FallbackChain {
    pub fn new(providers: Vec<Box<dyn LlmProvider>>) -> Self
}
```

**`generate_with_fallback()` 变更：**
```rust
pub async fn generate_with_fallback(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
    for provider in &self.providers {
        // ... 尝试每个 provider（不变）
    }

    // 删除: if self.use_rule_engine_fallback { ... }
    // 改为直接返回错误
    Err(LlmError::ProviderUnavailable("所有 Provider 都失败".to_string()))
}
```

**删除的方法：**
- `generate_rule_engine_fallback()` — 整个方法移除（约 30 行）

### 4.2 ai::rule_engine 模块删除

删除 `crates/ai/src/rule_engine.rs` 整个文件（约 69 行）。

更新 `crates/ai/src/lib.rs`：
```rust
// 删除:
// pub mod rule_engine;
// pub use rule_engine::{FallbackAction, SimpleActionType, SimplePosition, fallback_decision};
```

### 4.3 Bridge 层调用更新

**变更前：**
```rust
let fallback = FallbackChain::new(vec![Box::new(openai)], true);
//                                                ^^^^ 不再需要
```

**变更后：**
```rust
let fallback = FallbackChain::new(vec![Box::new(openai)]);
```

### 4.4 FallbackChain 测试更新

`crates/ai/src/fallback.rs` 中的测试需要更新：

- `test_fallback_chain_all_fail_without_rule_engine` — 保留，行为不变（返回错误）
- `test_fallback_chain_rule_engine_fallback` — **删除**，该测试依赖已移除的规则引擎兜底逻辑

## 5. 技术决策

### 决策1：ai 层不注入规则引擎回调

- **选型方案**：`FallbackChain` 接受 `Fn` 回调让调用方注入自定义兜底逻辑
- **选择理由**：保持 ai crate 纯粹性，不引入领域概念
- **备选方案**：回调注入、独立 rules crate
- **放弃原因**：回调增加接口复杂度；独立 crate 过度设计（代码量太小）

### 决策2：保留 DecisionPipeline 的完整兜底路径

- **选型方案**：`DecisionPipeline` 已有的 `core::RuleEngine::fallback_action()` 路径完全保留
- **选择理由**：这是正确的兜底位置——有完整的 WorldState 和 MotivationVector 上下文
- **备选方案**：无（该路径已存在且正确工作）

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| `FallbackChain` 删除兜底后 LLM 全部失败时行为变化 | **无风险** | `DecisionPipeline` 已有完整的 `core::RuleEngine::fallback_action()` 路径，兜底能力反而更强（从 3 种动作变为完整 13+ 种） |
| `FallbackChain::new()` 签名变更影响调用方 | 低 | 仅 bridge 的 `create_llm_provider()` 一处调用，更新一行代码即可 |
| 其他 crate 依赖了 `ai::rule_engine` 的 re-export | 低 | 仅 ai crate 自身和根测试可能引用，搜索确认无外部依赖 |

## 7. 迁移方案

### 7.1 实施步骤

1. 删除 `crates/ai/src/rule_engine.rs`
2. 更新 `crates/ai/src/lib.rs` 移除模块声明和 re-export
3. 简化 `crates/ai/src/fallback.rs` 的 `FallbackChain`
4. 更新 `crates/bridge/src/lib.rs` 的 `FallbackChain::new()` 调用
5. 运行 `cargo build` 和 `cargo test` 验证
6. 更新 `CLAUDE.md` 中的过时描述

### 7.2 回滚方案

若出现问题，恢复 `ai::rule_engine.rs` 和 `FallbackChain` 原始代码即可回滚，不影响 `core::RuleEngine`。

## 8. 待定事项

无
