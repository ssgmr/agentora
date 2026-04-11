# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

Agentora 记忆系统采用三层架构（ChronicleStore / ChronicleDB / ShortTerm），通过 TokenBudget 进行预算分配。所有参数以编译时常量形式硬编码在 `crates/core/src/memory/` 各模块中。

`llm.toml` 配置文件定义了 `[memory_compression]` 和 `[decision]` section，但 `config.rs` 解析后这些值从未被传递到记忆系统各模块。

### 1.2 现状分析

当前存在以下硬编码常量（15 处）：

| 常量 | 值 | 文件 | 问题 |
|------|-----|------|------|
| `TOTAL_LIMIT` | 1800 | token_budget.rs:5 | 无法运行时调整 |
| `CHRONICLE_BUDGET` | 800 | token_budget.rs:6 | 同上 |
| `CHRONICLE_DB_BUDGET` | 600 | token_budget.rs:7 | 同上 |
| `STRATEGY_BUDGET` | 400 | token_budget.rs:8 | 同上 |
| `CHRONICLE_LIMIT` | 1800 | chronicle_store.rs:9 | 同上 |
| `WORLD_SEED_LIMIT` | 500 | chronicle_store.rs:10 | 同上 |
| `IMPORTANCE_THRESHOLD` | 0.5 | chronicle_db.rs:8 | 同上 |
| `SEARCH_DEFAULT_LIMIT` | 5 | chronicle_db.rs:9 | 同上 |
| `SNIPPET_MAX_CHARS` | 200 | chronicle_db.rs:10 | 同上 |
| `SHORT_TERM_CAPACITY` | 5 | short_term.rs:5 | 同上 |
| `max_tokens` | 2500 | prompt.rs:15 | PromptBuilder 硬编码 |

此外 `truncate_to()` 使用 `s.len()`（字节数）截断，中文 1 字符 = 3 字节，导致实际容量仅为预期的 1/3。

### 1.3 关键干系人

- 核心引擎开发（`crates/core/`）
- AI 接入层配置（`crates/ai/`）
- 最终用户（通过修改 `config/llm.toml` 调整记忆行为）

## 2. 设计目标

### 目标

- 将 11 个记忆参数统一归入 `llm.toml` 的单一 `[memory]` section
- 统一计量单位为字符数（`.chars().count()`）
- 实现配置校验逻辑，拦截非法配置
- 保持向后兼容，默认值等于当前硬编码值

### 非目标

- 不引入动态热重载配置（需重启生效）
- 不新增数据库 schema 变更
- 不修改 LLM 调用参数（max_tokens、temperature 等保持现有决策配置）

## 3. 整体架构

### 3.1 架构概览

```
config/llm.toml
┌────────────────────────────┐
│ [memory]                    │
│   total_budget = 1800       │
│   chronicle_budget = 800    │
│   db_budget = 600           │
│   strategy_budget = 400     │
│   chronicle_limit = 1800    │
│   world_seed_limit = 500    │
│   importance_threshold = 0.5│
│   search_limit = 5          │
│   snippet_max_chars = 200   │
│   short_term_capacity = 5   │
│   prompt_max_tokens = 2500  │
└────────┬───────────────────┘
         │ 解析
         ▼
┌────────────────────────────┐
│ ai::config.rs              │
│ LlmConfig { memory:        │
│   MemoryConfig }           │
│   .validate() → Result     │
└────────┬───────────────────┘
         │ 传递
         ▼
┌─────────────────────────────────────────────────────┐
│ MemorySystem::new(config: &MemoryConfig)            │
│                                                      │
│  ┌──────────────┐  ┌─────────────┐  ┌────────────┐  │
│  │ TokenBudget  │  │ChronicleStore│  │ChronicleDB │  │
│  │ from_config  │  │ from_config  │  │ from_config│  │
│  └──────────────┘  └─────────────┘  └────────────┘  │
│                                                      │
│  ┌──────────────┐  ┌─────────────┐                   │
│  │ ShortTerm    │  │PromptBuilder│                   │
│  │ from_config  │  │ from_config  │                   │
│  └──────────────┘  └─────────────┘                   │
└─────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件名 | 职责说明 |
| --- | --- |
| `MemoryConfig` (ai/config.rs) | 配置结构体定义、serde 解析、validate() 校验 |
| `TokenBudget` (core/memory) | 预算分配器，改为接收配置参数，截断改用 chars_count |
| `ChronicleStore` (core/memory) | 编年史文件读取，从配置读取 limit |
| `ChronicleDB` (core/memory) | SQLite 记忆索引，从配置读取阈值/限制 |
| `ShortTerm` (core/memory) | 短期记忆缓存，从配置读取容量 |
| `MemorySystem` (core/memory) | 记忆系统总入口，组装各子模块并传递配置 |
| `PromptBuilder` (core/prompt) | 从配置读取 prompt_max_tokens |

### 3.3 数据流设计

```
初始化流程:

WorldBuilder/Agent 创建
    │
    ▼
加载 llm.toml ──▶ parse::<LlmConfig>()
    │
    ▼
config.memory.validate()
    │
    ├── 校验失败 ──▶ 返回错误，终止初始化
    │
    ▼ 校验通过
MemorySystem::new(&config.memory)
    │
    ├──▶ TokenBudget::from_config(&config.memory)
    ├──▶ ChronicleStore::from_config(&config.memory)
    ├──▶ ChronicleDB::from_config(&config.memory)
    ├──▶ ShortTerm::from_config(&config.memory)
    └──▶ PromptBuilder::new(config.memory.prompt_max_tokens)
```

## 4. 详细设计

### 4.1 接口设计

#### 配置结构体：MemoryConfig

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryConfig {
    // 预算层
    pub total_budget: usize,
    pub chronicle_budget: usize,
    pub db_budget: usize,
    pub strategy_budget: usize,
    // 存储层
    pub chronicle_limit: usize,
    pub world_seed_limit: usize,
    // 检索层
    pub importance_threshold: f32,
    pub search_limit: usize,
    pub snippet_max_chars: usize,
    // 容量层
    pub short_term_capacity: usize,
    // Prompt约束
    pub prompt_max_tokens: usize,
}
```

`MemoryConfig` 实现 `Default`，所有字段使用当前硬编码常量值。

#### 校验方法：validate()

```rust
impl MemoryConfig {
    pub fn validate(&self) -> Result<(), String> {
        // 预算值必须 > 0
        // importance_threshold 必须在 (0.0, 1.0]
        // 子预算之和 <= total_budget
        // total_budget <= prompt_max_tokens
        // search_limit, short_term_capacity > 0
    }
}
```

#### 各模块构造函数

```rust
// TokenBudget
impl TokenBudget {
    pub fn from_config(config: &MemoryConfig) -> Self { ... }
    pub fn with_defaults() -> Self { Self::from_config(&MemoryConfig::default()) }
}

// ChronicleStore
impl ChronicleStore {
    pub fn from_config(config: &MemoryConfig) -> Self { ... }
}

// ChronicleDB
impl ChronicleDB {
    pub async fn from_config(path: &str, config: &MemoryConfig) -> Result<Self> { ... }
}

// ShortTerm
impl ShortTerm {
    pub fn from_config(config: &MemoryConfig) -> Self { ... }
}

// PromptBuilder
impl PromptBuilder {
    pub fn from_config(config: &MemoryConfig) -> Self { ... }
}

// MemorySystem
impl MemorySystem {
    pub fn from_config(config: &MemoryConfig) -> Result<Self> { ... }
}
```

### 4.3 数据模型

无数据库 schema 变更。

### 4.5 核心算法

#### 配置校验逻辑

```
validate(config):
  1. total_budget > 0                    else → "预算值必须大于 0"
  2. chronicle_budget > 0               else → "预算值必须大于 0"
  3. db_budget > 0                      else → "预算值必须大于 0"
  4. strategy_budget > 0                else → "预算值必须大于 0"
  5. importance_threshold in (0, 1.0]   else → "重要性阈值必须在 (0.0, 1.0] 范围内"
  6. search_limit > 0                   else → "容量值必须大于 0"
  7. short_term_capacity > 0            else → "容量值必须大于 0"
  8. sub_budget_sum = chronicle + db + strategy
     sub_budget_sum <= total_budget     else → "子预算之和({sum})超过总预算({total})"
  9. total_budget <= prompt_max_tokens  else → "记忆总预算({total})超过Prompt上限({prompt})"
```

#### 字符数截断（替代字节数截断）

```
// 旧代码
fn truncate_to(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len { return s; }
    // 按字节截断，需保证不截断 UTF-8 边界
    &s[..max_len]
}

// 新代码
fn truncate_to_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
```

关键变化：
- 返回值从 `&str` 改为 `String`（因为 `chars().take().collect()` 产生 owned String）
- 调用方需适配，但逻辑更清晰安全

#### 动态降级逻辑（保持不变，改用配置值）

```
dynamic_allocate(chronicle, db, strategy, total):
  sub_sum = chronicle + db + strategy
  if sub_sum <= total:
    return (chronicle, db, strategy)
  else:
    // 降级
    strategy' = min(strategy, 200)
    db' = min(db, 300)
    chronicle' = chronicle  // 最高优先级，不降级
    return (chronicle', db', strategy')
```

降级阈值 200 和 300 保持硬编码（属于降级策略的内部常量，不需要用户配置）。

### 4.6 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| `llm.toml` 不存在 | 使用 `MemoryConfig::default()`，不报错 |
| `[memory]` section 不存在 | 使用 `MemoryConfig::default()`，不报错 |
| 部分参数缺失 | 缺失参数使用默认值（serde default），不报错 |
| 校验失败 | 返回 `Err(String)`，阻止 MemorySystem 初始化，由调用方决定是否 panic |
| `[memory_compression]` 与 `[memory]` 同时存在 | 仅使用 `[memory]`，旧 section 静默忽略 |
| 配置文件解析失败（格式错误） | 向上传播 serde 错误，由初始化入口统一处理 |

## 5. 技术决策

### 决策 1：配置计量单位统一为字符数

- **选型方案**：使用 `.chars().count()` 按 Unicode 标量值计数
- **选择理由**：中文 1 字符 = 3 字节，原字节数计量导致中文内容实际容量仅为 1/3；字符数计量对不同语言内容一视同仁
- **备选方案**：按 token 数计量（如 tiktoken 估算）
- **放弃原因**：引入 token 估算库增加依赖，且端侧场景性能敏感；字符数已足够解决核心问题

### 决策 2：配置结构使用 serde `default` 而非 Option

- **选型方案**：`#[serde(default)]` 注解字段，缺失时自动取 `Default::default()`
- **选择理由**：调用方无需处理 `Option<T>`，代码更简洁；部分配置 + 部分默认值场景天然支持
- **备选方案**：所有字段用 `Option<T>`，手动填充默认值
- **放弃原因**：样板代码多，且 `MemoryConfig` 本身就有完整默认值

### 决策 3：保持向后兼容的构造函数命名

- **选型方案**：新增 `from_config()` + `with_defaults()`，保留现有 `new()` 作为 `with_defaults()` 的别名
- **选择理由**：现有调用 `TokenBudget::new()` 的代码无需修改即可编译通过
- **备选方案**：直接修改 `new()` 签名接受配置参数
- **放弃原因**：会破坏所有现有调用点，改动面过大

### 决策 4：降级阈值保持硬编码

- **选型方案**：降级时 strategy → 200、db → 300 这两个阈值保持硬编码常量
- **选择理由**：降级是异常兜底策略，不属于用户需要调节的参数；增加配置项反而增加理解成本
- **备选方案**：将降级阈值也加入 MemoryConfig
- **放弃原因**：过度配置，降级阈值是算法内部参数，用户无调节需求

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| `truncate_to` 返回值从 `&str` 改为 `String`，调用方需适配 | 中 | 逐文件检查调用点，确保无借用生命周期问题 |
| 用户配置子预算之和超过总预算导致动态降级行为不一致 | 低 | 校验阶段拦截，配置层面保证不会触发降级 |
| serde 解析 `f32` 精度问题 | 低 | `importance_threshold` 取值范围简单，float 精度不影响行为 |
| 旧 `[memory_compression]` section 被静默忽略，用户不知情 | 低 | 可在 CLAUDE.md 或配置注释中注明已废弃 |

## 7. 迁移方案

### 7.1 部署步骤

1. 在 `config/llm.toml` 中新增 `[memory]` section（使用默认值，行为不变）
2. 更新代码：`ai/config.rs` 新增 `MemoryConfig` + 校验
3. 更新代码：`core/memory/` 各模块新增 `from_config()` 构造函数
4. 更新代码：`core/memory/mod.rs` 的 `MemorySystem::new()` 接受配置参数
5. 更新代码：`core/prompt.rs` 的 `PromptBuilder` 从配置读取
6. 更新代码：调用方（WorldBuilder、Agent 等）传递配置
7. `cargo build` + `cargo test` 验证

### 7.3 回滚方案

由于所有配置项有默认值且等于当前硬编码常量，回滚只需：
1. 从 `llm.toml` 删除 `[memory]` section
2. 回退代码即可，无数据迁移风险

## 8. 待定事项

- [ ] 降级阈值（200/300）是否需要配置化？（设计决策 4）
- [ ] `llm.toml` 中旧的 `[memory_compression]` section 是否需要打印一次 warning 提示用户迁移？
