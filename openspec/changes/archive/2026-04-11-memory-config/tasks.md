# 实施任务清单

## 1. 配置结构定义

- [x] 1.1 在 `crates/ai/src/config.rs` 中新增 `MemoryConfig` 结构体
  - 文件: `crates/ai/src/config.rs`
  - 包含 11 个字段，全部使用 `#[serde(default)]`
  - 实现 `Default` trait，默认值等于当前硬编码常量

- [x] 1.2 在 `crates/ai/src/config.rs` 的 `LlmConfig` 中添加 `memory: MemoryConfig` 字段
  - 文件: `crates/ai/src/config.rs`
  - 使用 `#[serde(default)]` 注解

- [x] 1.3 在 `crates/ai/src/config.rs` 中实现 `MemoryConfig::validate()` 校验方法
  - 文件: `crates/ai/src/config.rs`
  - 校验规则：预算值 > 0、阈值范围 (0,1]、子预算之和 <= 总预算、总预算 <= Prompt 上限
  - 返回 `Result<(), String>`

## 2. 记忆模块改造

- [x] 2.1 改造 `TokenBudget` 支持从配置初始化
  - 文件: `crates/core/src/memory/token_budget.rs`
  - 新增 `from_config(config: &MemoryConfig) -> Self` 构造函数
  - 新增 `with_defaults() -> Self` 作为向后兼容入口
  - 保留 `new()` 作为 `with_defaults()` 的别名

- [x] 2.2 修改 `TokenBudget` 截断方法从字节数改为字符数
  - 文件: `crates/core/src/memory/token_budget.rs`
  - 将 `truncate_to(s: &str, max_len: usize) -> &str` 改为 `truncate_to_chars(s: &str, max_chars: usize) -> String`
  - 更新 `allocate()` 和 `dynamic_allocate()` 中所有调用点

- [x] 2.3 改造 `ChronicleStore` 支持从配置初始化
  - 文件: `crates/core/src/memory/chronicle_store.rs`
  - 将 `CHRONICLE_LIMIT`、`WORLD_SEED_LIMIT` 改为实例字段
  - 新增 `from_config(config: &MemoryConfig) -> Self` 构造函数

- [x] 2.4 改造 `ChronicleDB` 支持从配置初始化
  - 文件: `crates/core/src/memory/chronicle_db.rs`
  - 将 `IMPORTANCE_THRESHOLD`、`SEARCH_DEFAULT_LIMIT`、`SNIPPET_MAX_CHARS` 改为实例字段
  - 修改 `search_for_prompt()` 使用配置的 `snippet_max_chars` 替代硬编码值
  - 新增 `from_config(path: &str, config: &MemoryConfig) -> Result<Self>` 构造函数

- [x] 2.5 改造 `ShortTerm` 支持从配置初始化
  - 文件: `crates/core/src/memory/short_term.rs`
  - 将 `SHORT_TERM_CAPACITY` 改为实例字段
  - 新增 `from_config(config: &MemoryConfig) -> Self` 构造函数

## 3. 记忆系统入口改造

- [x] 3.1 改造 `MemorySystem` 支持从配置初始化
  - 文件: `crates/core/src/memory/mod.rs`
  - 新增 `from_config(config: &MemoryConfig) -> Result<Self>` 方法
  - 在内部调用各子模块的 `from_config()`
  - 保留 `new()` 作为向后兼容入口（使用默认配置）

- [x] 3.2 更新 `memory/mod.rs` 中 `get_summary()` 调用 `db.search_for_prompt()` 传入配置的搜索限制
  - 文件: `crates/core/src/memory/mod.rs`
  - 使用 `ChronicleDB` 实例的 `search_limit` 配置值替代硬编码的 600

## 4. Prompt 和决策管道改造

- [x] 4.1 改造 `PromptBuilder` 支持从配置初始化
  - 文件: `crates/core/src/prompt.rs`
  - 将 `max_tokens: 2500` 改为从配置读取
  - 新增 `from_config(config: &MemoryConfig) -> Self` 构造函数
  - 保留 `new()` 作为向后兼容入口

- [x] 4.2 更新 `DecisionPipeline` 初始化时传入配置
  - 文件: `crates/core/src/decision.rs`
  - 修改 `DecisionPipeline::new()` 接受 `&MemoryConfig` 或使用 `PromptBuilder::from_config()`
  - 更新 `build_prompt_with_memory()` 方法使用配置化的 PromptBuilder

## 5. 调用方适配

- [x] 5.1 更新 `World` 初始化传入记忆配置
  - 文件: `crates/core/src/world.rs`
  - `World` 创建 `MemorySystem` 时传入配置
  - 确认所有 `WorldBuilder` 调用点

- [x] 5.2 更新 `Agent` 初始化传入记忆配置
  - 文件: `crates/core/src/agent.rs`
  - 确认 `Agent` 创建 `MemorySystem` 或 `DecisionPipeline` 时传入配置

- [x] 5.3 更新集成测试调用点
  - 文件: `tests/` 目录下所有使用 `MemorySystem::new()` 的测试
  - 改用 `MemorySystem::from_config()` 或保持 `new()`（使用默认配置）

## 6. 配置文件更新

- [x] 6.1 在 `config/llm.toml` 中新增 `[memory]` section
  - 文件: `config/llm.toml`
  - 包含全部 11 个参数，使用当前默认值
  - 添加中文注释说明各参数含义

- [x] 6.2 更新 `CLAUDE.md` 中关于记忆配置的说明
  - 文件: `CLAUDE.md`
  - 更新 Configuration section，说明 `[memory]` 配置项
  - 注明旧的 `[memory_compression]` 已废弃

## 7. 测试与验证

- [x] 7.1 单元测试 - `MemoryConfig::validate()` 校验逻辑
  - 文件: `crates/ai/src/config.rs` 或 `tests/` 目录
  - 覆盖：合法配置、子预算超限、总预算超限、阈值越界、零值

- [x] 7.2 单元测试 - `truncate_to_chars()` 字符截断
  - 文件: `crates/core/src/memory/token_budget.rs`
  - 覆盖：纯中文、纯英文、中英文混合、边界字符（emoji 等）

- [x] 7.3 单元测试 - `TokenBudget::from_config()` 配置初始化
  - 覆盖：自定义配置值生效、默认配置值正确

- [x] 7.4 集成测试 - 完整初始化流程
  - 验证 `MemorySystem::from_config()` 能正确从配置初始化所有子模块

- [x] 7.5 回归测试 - 运行全部 `cargo test`
  - 确认所有现有测试通过，行为不变

## 任务依赖关系

```
1.x (配置结构) ──▶ 2.x (记忆模块) ──▶ 3.x (系统入口) ──▶ 4.x (Prompt/决策)
                                                              │
                                                              ▼
                                                        5.x (调用方适配)
                                                              │
                                                              ▼
                                                        6.x (配置文件) + 7.x (测试)
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.1, 1.2, 1.3 | 定义配置结构体和校验，不依赖其他代码 |
| 阶段二 | 2.1 ~ 2.5 | 逐个改造记忆子模块，可并行 |
| 阶段三 | 3.1, 3.2 | 组装 MemorySystem，依赖阶段二 |
| 阶段四 | 4.1, 4.2 | 改造 Prompt/决策，依赖阶段一 |
| 阶段五 | 5.1 ~ 5.3 | 适配调用方，依赖阶段三、四 |
| 阶段六 | 6.1, 6.2 | 更新配置文件和文档 |
| 阶段七 | 7.1 ~ 7.5 | 测试验证，依赖以上所有阶段 |

## 文件结构总览

```
agentora/
├── crates/
│   ├── ai/src/
│   │   └── config.rs              ← 新增 MemoryConfig + validate()
│   └── core/src/
│       ├── memory/
│       │   ├── mod.rs             ← 修改 MemorySystem 初始化
│       │   ├── token_budget.rs    ← 修改截断 + 新增 from_config
│       │   ├── chronicle_store.rs ← 新增 from_config
│       │   ├── chronicle_db.rs    ← 新增 from_config
│       │   └── short_term.rs      ← 新增 from_config
│       ├── prompt.rs              ← 新增 from_config
│       ├── decision.rs            ← 修改 DecisionPipeline 初始化
│       ├── world.rs               ← 修改调用点
│       └── agent.rs               ← 修改调用点
├── config/
│   └── llm.toml                   ← 新增 [memory] section
├── tests/                         ← 适配调用点
└── CLAUDE.md                      ← 更新文档
```
