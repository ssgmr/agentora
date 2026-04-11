# 需求说明书

## 背景概述

当前记忆系统（`crates/core/src/memory/`）中所有关键参数均为硬编码的编译时常量，包括 `TOTAL_LIMIT = 1800`、`CHRONICLE_BUDGET = 800`、`SHORT_TERM_CAPACITY = 5` 等 15 处常量。`llm.toml` 中虽存在 `[memory_compression]` 和 `[decision]` 配置 section，但代码从未读取这些值，导致配置文件形同虚设。

此外，`token_budget.rs` 使用字节数（`s.len()`）而非字符数进行截断，对于中文内容（1字符=3字节）会导致实际记忆容量远低于预期。

## 变更目标

- 将记忆系统所有硬编码值迁移到 `llm.toml` 的单一 `[memory]` section，支持运行时配置
- 统一计量单位为字符数（`.chars().count()`），消除中英文差异
- 新增配置校验规则：总预算不超过 Prompt 上限、子预算之和不超总预算、阈值合法等
- 保持向后兼容：所有配置项有默认值（等于当前硬编码常量），不传配置行为不变
- 废弃旧的 `[memory_compression]` section（静默忽略，不报错）

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `configurable-memory` | 记忆系统参数可配置化，支持通过 llm.toml `[memory]` section 调整预算、容量、检索等所有参数 |

### 修改功能

无现有规格变更。

## 影响范围

- **代码模块**：
  - `crates/ai/src/config.rs` — 新增 `MemoryConfig` 结构体及 `validate()` 校验
  - `crates/core/src/memory/token_budget.rs` — `from_config()` 构造函数，字节截断改字符截断
  - `crates/core/src/memory/chronicle_store.rs` — `from_config()` 构造函数
  - `crates/core/src/memory/chronicle_db.rs` — `from_config()` 构造函数
  - `crates/core/src/memory/short_term.rs` — `from_config()` 构造函数
  - `crates/core/src/memory/mod.rs` — `MemorySystem::new()` 接受 `MemoryConfig` 参数
  - `crates/core/src/prompt.rs` — `PromptBuilder` 从配置读取 `max_tokens`
  - `crates/core/src/decision.rs` — `DecisionPipeline` 初始化传入配置
- **配置文件**：`config/llm.toml` — 新增 `[memory]` section
- **依赖组件**：无新增外部依赖

## 验收标准

- [ ] `llm.toml` 新增 `[memory]` section，包含预算/存储/检索/容量/Prompt 约束共 11 个参数
- [ ] 配置校验逻辑生效：总预算 ≤ Prompt 上限、子预算之和 ≤ 总预算、阈值范围合法
- [ ] 记忆截断改用字符数计量（`.chars().count()`），中文内容容量正确
- [ ] `MemoryConfig` 实现 `Default`，不传配置时使用默认值（等于当前硬编码值），行为不变
- [ ] 旧的 `[memory_compression]` section 被静默忽略，不影响运行
- [ ] `cargo build` 和 `cargo test` 全部通过
