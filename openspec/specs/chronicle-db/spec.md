# ChronicleDB 长期记忆索引

## Purpose

定义基于SQLite + FTS5的长期记忆索引系统，支持高重要性事件持久化、全文检索、重要性衰减和短期记忆迁移。与chronicle-store-io（文件编年史）和token-budget（预算分配）互补。

## Requirements

### Requirement: ChronicleDB SQLite表结构

系统 SHALL 使用SQLite为每个Agent建立长期记忆索引，存储重要性评分>0.5的事件片段。

#### Scenario: 初始化表结构

- **WHEN** 初始化ChronicleDB
- **THEN** 系统 SHALL 创建以下表:

```sql
CREATE TABLE memory_fragments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tick INTEGER NOT NULL,
    text_summary TEXT NOT NULL,
    emotion_tag TEXT NOT NULL,
    event_type TEXT NOT NULL,
    importance REAL NOT NULL DEFAULT 0.5,
    compression_level TEXT NOT NULL DEFAULT 'none',
    created_at INTEGER NOT NULL
);
```

### Requirement: FTS5全文索引

系统 SHALL 使用FTS5虚拟表为memory_fragments建立全文索引，支持关键词和情感标签组合查询。

#### Scenario: FTS5虚拟表和触发器

- **WHEN** 初始化ChronicleDB
- **THEN** 系统 SHALL 创建:

```sql
CREATE VIRTUAL TABLE memory_fts USING fts5(
    text_summary,
    emotion_tag,
    event_type,
    content='memory_fragments',
    content_rowid=id
);

CREATE TRIGGER memory_fts_insert AFTER INSERT ON memory_fragments BEGIN
    INSERT INTO memory_fts(rowid, text_summary, emotion_tag, event_type)
    VALUES (new.id, new.text_summary, new.emotion_tag, new.event_type);
END;
```

### Requirement: 高重要性事件持久化

系统 SHALL 将重要性评分>0.5的事件写入memory_fragments表，同时更新FTS5索引。重要性阈值从0.5开始（低于此值不写入长期记忆）。

#### Scenario: 写入高重要性事件

- **WHEN** 事件的重要性评分>0.5（如首次交易、遭受攻击、发现遗迹）
- **THEN** 系统 SHALL 将事件写入memory_fragments表
- **AND** 同时更新FTS5索引

### Requirement: FTS5检索流程

系统 SHALL 在构建决策Prompt时通过FTS5检索相关历史记忆。

#### Scenario: FTS5检索流程

- **WHEN** 构建决策Prompt需要检索历史记忆
- **THEN** 系统 SHALL 执行以下流程:
  1. 根据Spark类型构建FTS5查询（关键词+emotion_tag）
  2. 执行`SELECT ... WHERE memory_fts MATCH 'query' ORDER BY importance DESC LIMIT 5`
  3. 截断结果到max_chars（围绕匹配词）
  4. 返回原始片段（不需要LLM摘要）

#### Scenario: 资源压力检索

- **WHEN** Spark为"resource_pressure"
- **THEN** 系统 SHALL 执行查询:
```sql
SELECT text_summary, importance FROM memory_fragments
WHERE memory_fts MATCH 'resource AND (trade OR gather OR explore)'
ORDER BY importance DESC LIMIT 3;
```

#### Scenario: 社交压力检索

- **WHEN** Spark为"social_pressure"
- **THEN** 系统 SHALL 执行查询:
```sql
SELECT text_summary, importance FROM memory_fragments
WHERE memory_fts MATCH '(alliance OR trade OR trust) AND NOT attack'
ORDER BY importance DESC LIMIT 3;
```

### Requirement: 记忆重要性衰减

系统 SHALL 定期对所有记忆片段执行重要性衰减，并清理低重要性记录。

#### Scenario: 定期衰减

- **WHEN** 每50 tick到达
- **THEN** 系统 SHALL 执行: `UPDATE memory_fragments SET importance = importance * 0.95;`
- **AND** 删除importance < 0.3的记录

### Requirement: 记忆压缩级别

系统 SHALL 对记忆片段应用压缩级别，控制存储和检索时的文本长度。

#### Scenario: 压缩级别应用

- **WHEN** 记忆片段token数过多时
- **THEN** 系统 SHALL 应用压缩级别:
  - `none`: 原始完整文本
  - `light`: 保留关键实体和情感
  - `heavy`: 仅保留事件类型和结果
- **AND** 压缩后更新text_summary和compression_level

### Requirement: 短期记忆缓存

系统 SHALL 维护最近5条事件作为短期记忆，存储在内存中供当前决策使用。短期记忆不持久化，仅作为Echo反馈到ChronicleDB/ChronicleStore的中间缓存。

#### Scenario: 短期记忆写入

- **WHEN** Agent执行动作或遭受事件
- **THEN** 系统 SHALL 将事件写入短期记忆（含tick、类型、内容、情感标签）
- **AND** 短期记忆 SHALL 保留最近5条
- **AND** 超出时最旧的移入ChronicleDB（若importance > 0.5）

#### Scenario: 短期记忆到长期记忆迁移

- **WHEN** 短期记忆溢出且事件importance > 0.5
- **THEN** 系统 SHALL 将事件写入ChronicleDB memory_fragments
- **AND** 更新FTS5索引

### Requirement: 单进程简化设计

系统 SHALL 针对单Agent单进程场景简化并发处理，不需要多进程WAL+retry机制。

#### Scenario: 本地私有记忆

- **WHEN** Agent运行时
- **THEN** 记忆 SHALL 完全本地私有，不通过P2P同步
- **AND** 仅在Agent死亡时生成遗产包广播（摘要+哈希）

#### Scenario: 简化并发处理

- **WHEN** UI线程和决策线程需要访问记忆
- **THEN** 系统 SHALL 使用简单的Mutex保护
- **AND** 不需要复杂的WAL多writer模式
- **AND** SQLite连接 SHALL 使用普通模式而非WAL（除非需要UI不阻塞）
