## Why

记忆系统当前仅有框架实现，多个关键功能为 TODO 状态：ChronicleStore 文件加载/截断/原子写入未实现，ChronicleDB FTS5 检索结果未与决策 Prompt 集成，记忆总量控制仅有框架。这导致 Agent 无法积累历史记忆，无法验证"记忆压缩注入 Prompt"的设计假设。

## What Changes

- **新增** ChronicleStore 完整文件 I/O 操作（加载、截断、原子写入）
- **新增** ChronicleDB 检索集成（从数据库检索结果注入 Prompt）
- **新增** 记忆总量控制器（TokenBudget，按优先级分配 1800 chars）
- **修改** MemorySystem 集成 ChronicleDB 和 ChronicleStore
- **修改** PromptBuilder 使用记忆系统提供的摘要
- **新增** 记忆围栏保护（`<chronicle-context>` 标签包裹）

## Capabilities

### New Capabilities

- `chronicle-store-io`: ChronicleStore 完整文件 I/O，包括加载、截断、原子写入、安全扫描
- `chronicle-db-integration`: ChronicleDB FTS5 检索结果注入决策 Prompt
- `token-budget`: 记忆总量控制，按优先级分配空间（ChronicleStore 800 + ChronicleDB 600 + StrategyHub 400）

### Modified Capabilities

- `memory-system`: 集成 ChronicleDB 和 ChronicleStore，实现完整三层记忆协调

## Impact

- **affected crates**: `core` (memory 模块), `ai` (Prompt 构建)
- **dependencies**: `rusqlite` (已存在), `std::fs` (文件操作)
- **breaking changes**: 无，MemorySystem 当前为占位实现
- **integration points**: PromptBuilder 需调用记忆系统获取摘要；World::apply_action 需写入记忆
