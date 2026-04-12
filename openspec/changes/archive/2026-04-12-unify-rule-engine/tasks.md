# 实施任务清单

## 1. 删除 ai::rule_engine 模块

从 ai crate 移除简陋版规则引擎。

- [x] 1.1 删除 `crates/ai/src/rule_engine.rs` 文件
  - 文件: `crates/ai/src/rule_engine.rs` — 整个文件删除
  - 依赖: 无

- [x] 1.2 从 `crates/ai/src/lib.rs` 移除模块声明和 re-export
  - 文件: `crates/ai/src/lib.rs`
  - 删除: `pub mod rule_engine;`
  - 删除: `pub use rule_engine::{FallbackAction, SimpleActionType, SimplePosition, fallback_decision};`
  - 依赖: 1.1

## 2. 简化 FallbackChain

移除 `FallbackChain` 的规则引擎兜底逻辑。

- [x] 2.1 从 `FallbackChain` 结构体删除 `use_rule_engine_fallback` 字段
  - 文件: `crates/ai/src/fallback.rs`
  - 依赖: 1.1

- [x] 2.2 修改 `FallbackChain::new()` 签名
  - 文件: `crates/ai/src/fallback.rs`
  - 参数从 `(providers, use_rule_engine_fallback)` 改为 `(providers)`
  - 依赖: 2.1

- [x] 2.3 简化 `generate_with_fallback()` 方法
  - 文件: `crates/ai/src/fallback.rs`
  - 删除 `if self.use_rule_engine_fallback { ... }` 分支
  - 所有 Provider 失败后直接返回 `Err(LlmError::ProviderUnavailable(...))`
  - 依赖: 2.1

- [x] 2.4 删除 `generate_rule_engine_fallback()` 方法
  - 文件: `crates/ai/src/fallback.rs`
  - 删除整个方法（约 30 行）
  - 依赖: 2.3

- [x] 2.5 更新 `fallback.rs` 中的测试
  - 文件: `crates/ai/src/fallback.rs`
  - 删除: `test_fallback_chain_rule_engine_fallback` 测试
  - 更新: `test_fallback_chain_all_fail_without_rule_engine` 的 `new()` 调用参数
  - 更新: 其他测试中 `FallbackChain::new()` 的调用（去掉 bool 参数）
  - 依赖: 2.2

## 3. 更新 bridge 层调用

更新 bridge 创建 `FallbackChain` 的代码。

- [x] 3.1 更新 `create_llm_provider()` 中 `FallbackChain::new()` 调用
  - 文件: `crates/bridge/src/lib.rs`
  - 从 `FallbackChain::new(vec![Box::new(openai)], true)` 改为 `FallbackChain::new(vec![Box::new(openai)])`
  - 依赖: 2.2

## 4. 更新文档

- [x] 4.1 更新 `CLAUDE.md` 中关于 `SimpleActionType` 的过时描述
  - 文件: `CLAUDE.md`
  - 删除 AI Crate 描述中的 `- **\`SimpleActionType\`** — 规则引擎动作类型（移动/交互/建造/社交）` 行
  - 依赖: 1.1

## 5. 测试与验证

- [x] 5.1 编译验证 `cargo build` 通过
- [x] 5.2 运行单元测试 `cargo test` 通过
- [x] 5.3 运行单个测试文件验证 ai crate 测试通过 `cargo test -p agentora-ai`
- [x] 5.4 运行 Godot 客户端验证 LLM 失败时 Player Agent 使用 `core::RuleEngine` 兜底

## 任务依赖关系

```
1.1 (删除 rule_engine.rs)
  │
  └─→ 1.2 (更新 lib.rs)
        │
        └─→ 2.1 (删除 FallbackChain 字段)
              │
              ├─→ 2.2 (修改 new() 签名)
              │     │
              │     ├─→ 2.5 (更新测试)
              │     │
              │     └─→ 3.1 (更新 bridge 调用)
              │
              └─→ 2.3 (简化 generate_with_fallback)
                    │
                    └─→ 2.4 (删除 generate_rule_engine_fallback)

4.1 (更新文档) ── 依赖 1.1
5.x (测试验证) ── 依赖所有 1-4 阶段完成
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.1, 1.2 | 删除 ai::rule_engine 模块 |
| 阶段二 | 2.1-2.5 | 简化 FallbackChain + 更新测试 |
| 阶段三 | 3.1 | 更新 bridge 调用 |
| 阶段四 | 4.1 | 更新文档 |
| 阶段五 | 5.1-5.4 | 编译、测试、验证 |

## 文件结构总览

```
crates/
├── ai/src/
│   ├── rule_engine.rs              ← 删除: 整个文件
│   ├── fallback.rs                 ← 修改: 简化 FallbackChain
│   └── lib.rs                      ← 修改: 移除 rule_engine 模块
├── bridge/src/
│   └── lib.rs                      ← 修改: 更新 FallbackChain::new() 调用
└── core/src/
    └── rule_engine.rs              ← 不变: 完整保留

CLAUDE.md                           ← 修改: 删除 SimpleActionType 描述
```
