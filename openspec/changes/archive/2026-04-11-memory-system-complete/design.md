## Context

当前记忆系统实现状态：
- `ChronicleStore` 有安全扫描和框架，但文件加载/截断/原子写入均为 TODO
- `ChronicleDB` 有表结构和 FTS5 触发器，但检索结果未集成到决策流程
- `MemorySystem` 仅有短期记忆，db_loaded 标记未使用
- `MemoryEvent` 已定义，但无写入 ChronicleDB 的逻辑
- 设计要求的 1800 chars 总量控制未实现

MVP 验证需求：Agent 需要基于历史记忆做出个性化决策，验证记忆压缩和检索机制是否有效。

## Goals / Non-Goals

**Goals:**
- 实现 ChronicleStore 完整文件 I/O（加载、添加 entry、截断、原子写入）
- 实现 ChronicleDB 检索集成（按 Spark 类型查询，注入 Prompt）
- 实现记忆总量控制（≤1800 chars，优先级分配）
- 实现记忆围栏保护（`<chronicle-context>`标签）
- 集成到决策管道，替换当前的空记忆摘要

**Non-Goals:**
- 多 Agent 记忆共享（MVP 后实现）
- 向量相似度搜索（FTS5 已足够）
- 记忆压缩 LLM 摘要（直接存储原始片段）

## Decisions

### Decision 1: 文件路径和目录结构

```
~/.agentora/agents/<agent_id>/
├── CHRONICLE.md      # Agent 编年史（≤1800 chars）
├── WORLD_SEED.md     # 世界认知（≤500 chars）
└── chronicle.db      # SQLite 数据库
```

**理由**: 与设计文档一致，每个 Agent 独立的记忆目录

### Decision 2: 原子写入策略

- 使用临时文件 + rename 实现原子性
- 先写入 `.tmp` 文件，成功后 rename 覆盖原文件
- 崩溃恢复：启动时检查 `.tmp` 文件并删除

**理由**: 防止进程崩溃导致文件部分损坏

### Decision 3: 截断策略

- CHRONICLE.md 超限 1800 chars 时，删除最旧的 entry（按 `§` 分隔符）
- WORLD_SEED.md 超限 500 chars 时，截断最后一条记录
- 优先保留最近的 entry

### Decision 4: FTS5 查询构建

根据 Spark 类型构建查询：
- `ResourcePressure` → `'resource AND (gather OR trade OR explore)'`
- `SocialPressure` → `'(alliance OR trade OR trust) AND NOT attack'`
- `Explore` → `'discover OR explore OR find'`

**理由**: Spark 类型与查询关键词映射，提高检索相关性

### Decision 5: 记忆预算分配

| 部分 | 预算 | 说明 |
|------|------|------|
| ChronicleStore 快照 | 800 chars | 固定，包含 CHRONICLE.md + WORLD_SEED.md |
| ChronicleDB 检索 | 600 chars | 动态，返回 top 3-5 片段 |
| StrategyHub 策略 | 400 chars | 动态，仅 metadata 或详情 |
| **总计** | **≤1800 chars** | 硬截断 |

## Risks / Trade-offs

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| 文件 I/O 性能 | 每 tick 写入可能阻塞 | 异步写入、批量写入（每 10 tick） |
| SQLite 连接管理 | 多 Agent 时连接数过多 | 每 Agent 一个连接，使用连接池 |
| FTS5 查询不准确 | 检索结果与 Spark 不相关 | 调整查询关键词、增加 emotion_tag 过滤 |
| 记忆截断丢失关键信息 | 重要记忆被删除 | 重要性评分 >0.8 的记忆不删除 |

## Migration Plan

### 部署步骤

1. 实现 ChronicleStore 文件 I/O（load/add_entry/truncate/atomic_write）
2. 实现 ChronicleDB 检索集成（search 方法 + Prompt 注入）
3. 实现 TokenBudget 总量控制
4. 修改 MemorySystem 集成各组件
5. 修改 PromptBuilder 使用记忆摘要
6. 运行单 Agent 测试验证记忆累积

### 回滚策略

- git tag 标记当前状态
- 若记忆系统失败，回退到空记忆摘要
- 保留 ChronicleStore 文件用于问题诊断

## Open Questions

- [ ] `~/.agentora` 路径在 Windows/macOS/Linux的兼容性
- [ ] FTS5 查询的中文分词问题（是否需要额外配置）
- [ ] 重要性评分的初始值设定规则
