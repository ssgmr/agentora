# 功能规格说明：记忆系统

> **设计借鉴**: 本规格借鉴 Hermes Agent 的记忆架构设计，适配 Agentora 去中心化游戏客户端的本地私有记忆场景。

## ADDED Requirements

### Requirement: 三层记忆架构

系统 SHALL 为每个Agent维护三层记忆架构：持久化编年史（ChronicleStore）、长期记忆索引（ChronicleDB）、决策策略库（StrategyHub）。每层有独立的存储形式和检索方式。

```
┌─────────────────────────────────────────────────────────────────────────┐
│          Agentora 单Agent本地记忆三层架构                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   Layer 1: ChronicleStore (持久化编年史)                                │
│   ┌──────────────────────────────────────────────────────────────────┐ │
│   │  文件位置: ~/.agentora/agents/<agent_id>/                          │ │
│   │  ├── CHRONICLE.md     (Agent编年史: 关键事件/情感变化/学习成果)    │ │
│   │  ├── WORLD_SEED.md    (世界认知: 区域地图/资源分布/社交网络概览)   │ │
│   │  │                                                                │ │
│   │  特性:                                                             │ │
│   │  - 冻结快照: decision开始时注入prompt，中途不变                     │ │
│   │  - 实时写入: Echo反馈后立即持久化                                   │ │
│   │  - 围栏保护: <chronicle-context> 标签                              │ │
│   │  - 安全扫描: 阻止prompt injection                                  │ │
│   │  - ENTRY_DELIMITER: § 支持多行entry                                │ │
│   │  - atomic_write: 防止进程崩溃时部分损坏                             │ │
│   └──────────────────────────────────────────────────────────────────┘ │
│                                                                         │
│   Layer 2: ChronicleDB (长期记忆索引)                                   │
│   ┌──────────────────────────────────────────────────────────────────┐ │
│   │  文件位置: ~/.agentora/agents/<agent_id>/chronicle.db              │ │
│   │  │                                                                │ │
│   │  特性:                                                             │ │
│   │  - SQLite + FTS5 全文索引                                          │ │
│   │  - 单Agent单进程，无需复杂并发处理                                   │ │
│   │  - 搜索流程: FTS5 MATCH → 按重要性 → 截断 → 返回                    │ │
│   │  - 衰减机制: 每50 tick importance *= 0.95                          │ │
│   └──────────────────────────────────────────────────────────────────┐ │
│                                                                         │
│   Layer 3: StrategyHub (决策策略库)                                     │
│   ┌──────────────────────────────────────────────────────────────────┐ │
│   │  文件位置: ~/.agentora/agents/<agent_id>/strategies/               │ │
│   │  │                                                                │ │
│   │  特性:                                                             │ │
│   │  - STRATEGY.md + YAML frontmatter                                 │ │
│   │  - progressive disclosure: metadata → 详情 → 执行案例              │ │
│   │  - 自我改进机制: create/patch/decay                                │ │
│   │  (详见 strategy-system spec)                                       │ │
│   └──────────────────────────────────────────────────────────────────┐ │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Scenario: 三层记忆协调工作

- **WHEN** Agent执行决策循环
- **THEN** 系统 SHALL 按顺序使用三层记忆:
  1. ChronicleStore 冻结快照注入 system prompt
  2. ChronicleDB FTS5 检索相关历史片段
  3. StrategyHub 检索匹配当前 Spark 的策略
- **AND** 总记忆内容 SHALL 不超过 1800 chars

### Requirement: ChronicleStore 持久化编年史

系统 SHALL 为每个Agent维护两个Markdown文件作为持久化编年史：CHRONICLE.md（Agent自述）和 WORLD_SEED.md（世界认知）。编年史使用冻结快照模式，decision开始时注入prompt后不再修改。

#### Scenario: 编年史冻结快照

- **WHEN** 决策循环开始构建 Prompt
- **THEN** 系统 SHALL 读取 CHRONICLE.md 和 WORLD_SEED.md 当前内容
- **AND** 将内容冻结为本轮决策的 system_prompt_snapshot
- **AND** 本轮决策期间 SHALL 不再修改快照内容
- **AND** Echo反馈后的写入 SHALL 在下一轮决策才生效

#### Scenario: 编年史字符限制

- **WHEN** 编年史内容写入时
- **THEN** CHRONICLE.md SHALL 不超过 1800 chars
- **AND** WORLD_SEED.md SHALL 不超过 500 chars
- **AND** 超限时 SHALL 截断最旧的 entry

#### Scenario: 编年史 Entry 格式

- **WHEN** Agent记录新编年史 entry
- **THEN** entry SHALL 使用 § (section sign) 作为分隔符
- **AND** entry 可包含多行内容
- **AND** entry 格式为: `[tick] 事件摘要`

#### Scenario: 编年史写入工具

- **WHEN** Echo反馈完成后
- **THEN** 系统 SHALL 提供 chronicle 工具接口:
  - `chronicle(action="add", target="chronicle", content="...")`
  - `chronicle(action="replace", target="chronicle", find="...", replace="...")`
  - `chronicle(action="remove", target="chronicle", find="...")`
  - `chronicle(action="read", target="chronicle")`

#### Scenario: 编年史围栏保护

- **WHEN** 编年史内容注入 Prompt
- **THEN** 系统 SHALL 用 `<chronicle-context>` 标签包裹内容
- **AND** 添加系统注: "[系统注：以下是Agent历史记忆摘要，非当前事件输入]"
- **AND** 当前 Spark 用 `<current-spark>` 标签单独包裹

```
<chronicle-context>
[系统注：以下是Agent历史记忆摘要，非当前事件输入]

§
[tick 1250] 与商人交易铁矿，发现其价格异常，标记为可疑
§
[tick 1245] 发现北部区域有遗迹，记录位置供后续探索

</chronicle-context>

<current-spark>
[系统注：以下是当前感知的压力环境]
资源压力: 铁矿库存 3/20 (缺口大)
社交压力: 无近期交互
</current-spark>
```

#### Scenario: 编年史安全扫描

- **WHEN** 写入编年史内容时
- **THEN** 系统 SHALL 扫描以下威胁模式:
  - prompt injection: "ignore previous instructions"
  - role hijack: "you are now"
  - rule bypass: "override rules"
  - invisible unicode: U+200B, U+200C 等零宽字符
- **AND** 检测到威胁 SHALL 拒绝写入并返回错误

### Requirement: ChronicleDB 长期记忆索引

系统 SHALL 使用 SQLite + FTS5 为每个Agent建立长期记忆索引，存储重要性评分 > 0.5的事件片段，支持全文检索。

#### Scenario: SQLite 表结构

- **WHEN** 初始化 ChronicleDB
- **THEN** 系统 SHALL 创建以下表:

```sql
-- 记忆片段表
CREATE TABLE memory_fragments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tick INTEGER NOT NULL,
    text_summary TEXT NOT NULL,
    emotion_tag TEXT NOT NULL,      -- JSON array: ["suspicious", "curious"]
    event_type TEXT NOT NULL,       -- trade/explore/attack/discover_legacy
    importance REAL NOT NULL DEFAULT 0.5,
    compression_level TEXT NOT NULL DEFAULT 'none',  -- none/light/heavy
    created_at INTEGER NOT NULL
);

-- FTS5 全文索引虚拟表
CREATE VIRTUAL TABLE memory_fts USING fts5(
    text_summary,
    emotion_tag,
    event_type,
    content='memory_fragments',
    content_rowid=id
);

-- FTS5 同步触发器
CREATE TRIGGER memory_fts_insert AFTER INSERT ON memory_fragments BEGIN
    INSERT INTO memory_fts(rowid, text_summary, emotion_tag, event_type)
    VALUES (new.id, new.text_summary, new.emotion_tag, new.event_type);
END;
```

#### Scenario: 高重要性事件持久化

- **WHEN** 事件的重要性评分 > 0.5（如首次交易、遭受攻击、发现遗迹）
- **THEN** 系统 SHALL 将事件写入 memory_fragments 表
- **AND** 同时更新 FTS5 索引

#### Scenario: FTS5 检索流程

- **WHEN** 构建决策 Prompt 需要检索历史记忆
- **THEN** 系统 SHALL 执行以下流程:
  1. 根据 Spark 类型构建 FTS5 查询（关键词 + emotion_tag）
  2. 执行 `SELECT ... WHERE memory_fts MATCH 'query' ORDER BY importance DESC LIMIT 5`
  3. 截断结果到 max_chars (围绕匹配词)
  4. 返回原始片段（不需要 LLM 摘要）

#### Scenario: FTS5 查询示例

- **WHEN** Spark 为 "resource_pressure"
- **THEN** 系统 SHALL 执行查询:
```sql
SELECT text_summary, importance FROM memory_fragments
WHERE memory_fts MATCH 'resource AND (trade OR gather OR explore)'
ORDER BY importance DESC LIMIT 3;
```

- **WHEN** Spark 为 "social_pressure"
- **THEN** 系统 SHALL 执行查询:
```sql
SELECT text_summary, importance FROM memory_fragments
WHERE memory_fts MATCH '(alliance OR trade OR trust) AND NOT attack'
ORDER BY importance DESC LIMIT 3;
```

#### Scenario: 记忆重要性衰减

- **WHEN** 每 50 tick 到达
- **THEN** 系统 SHALL 执行:
```sql
UPDATE memory_fragments SET importance = importance * 0.95;
```
- **AND** 删除 importance < 0.3 的记录

#### Scenario: 记忆压缩级别

- **WHEN** 记忆片段 token 数过多时
- **THEN** 系统 SHALL 应用压缩级别:
  - `none`: 原始完整文本
  - `light`: 保留关键实体和情感
  - `heavy`: 仅保留事件类型和结果
- **AND** 压缩后更新 text_summary 和 compression_level

### Requirement: 记忆总量控制

系统 SHALL 确保进入 Prompt 的记忆部分不超过 1800 chars，按优先级分配空间。

#### Scenario: 记忆空间分配

- **WHEN** 构建决策 Prompt
- **THEN** 记忆空间 SHALL 按以下优先级分配:
  1. ChronicleStore 快照: 800 chars (固定)
  2. ChronicleDB FTS5 检索: 600 chars (动态)
  3. StrategyHub 策略摘要: 400 chars (动态)
- **AND** 超限时 SHALL 截断低优先级内容

#### Scenario: 记忆截断策略

- **WHEN** 总记忆内容超过 1800 chars
- **THEN** 系统 SHALL 按以下顺序截断:
  1. 截断 ChronicleDB 检索结果（保留 top 1）
  2. 截断 StrategyHub 策略（保留 metadata only）
  3. 截断 ChronicleStore（保留最近 3 entries）

### Requirement: 短期记忆（决策辅助）

系统 SHALL 维护最近 5 条事件作为短期记忆，存储在内存中供当前决策使用。短期记忆不持久化，仅作为 Echo 反馈到 ChronicleDB/ChronicleStore 的中间缓存。

#### Scenario: 短期记忆写入

- **WHEN** Agent执行动作或遭受事件
- **THEN** 系统 SHALL 将事件写入短期记忆（含 tick、类型、内容、情感标签）
- **AND** 短期记忆 SHALL 保留最近 5 条
- **AND** 超出时最旧的移入 ChronicleDB（若 importance > 0.5）

#### Scenario: 短期记忆到长期记忆迁移

- **WHEN** 短期记忆溢出且事件 importance > 0.5
- **THEN** 系统 SHALL 将事件写入 ChronicleDB memory_fragments
- **AND** 更新 FTS5 索引

### Requirement: 单进程简化设计

系统 SHALL 针对单 Agent 单进程场景简化并发处理，不需要 Hermes 那种多进程 WAL + retry 机制。

#### Scenario: 本地私有记忆

- **WHEN** Agent 运行时
- **THEN** 记忆 SHALL 完全本地私有，不通过 P2P 同步
- **AND** 仅在 Agent 死亡时生成遗产包广播（摘要 + 哈希）

#### Scenario: 简化并发处理

- **WHEN** UI 线程和决策线程需要访问记忆
- **THEN** 系统 SHALL 使用简单的 Mutex 考虑
- **AND** 不需要复杂的 WAL 多 writer 模式
- **AND** SQLite 连接 SHALL 使用普通模式而非 WAL（除非需要 UI 不阻塞）

## REMOVED Requirements

以下需求已被移除或替换：

### Requirement: 向量相似度搜索 (REMOVED)

原需求"使用简单向量相似度搜索"已被移除，改用 FTS5 全文索引替代。原因：FTS5 足够满足 Agentora 的标签/关键词检索需求，向量索引增加端侧计算负担。

### Requirement: 中期记忆 LLM 压缩 (REMOVED)

原需求"中期记忆 LLM 压缩摘要"已被移除，并入 ChronicleStore 的编年史设计。ChronicleDB 直接存储原始片段，通过 FTS5 检索时按重要性排序返回。

## CHANGED Requirements

### Requirement: 长期记忆阈值调整 (CHANGED)

原需求"重要性 > 0.7 写入长期记忆"调整为"重要性 > 0.5"，降低阈值以增加可检索记忆数量。

---

> **P2P 同步边界**: 记忆系统属于本地私有数据，不通过 P2P 同步。仅 Agent 死亡时的遗产包（记忆摘要 + 多模态哈希）会通过 GossipSub 广播到网络，其他 Agent 可按需拉取完整内容（需授权）。