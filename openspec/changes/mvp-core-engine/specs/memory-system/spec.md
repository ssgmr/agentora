# 功能规格说明：记忆系统

## ADDED Requirements

### Requirement: 三级记忆架构

系统 SHALL 为每个Agent维护三级记忆：短期记忆（最近5条事件完整文本）、中期记忆（LLM压缩摘要）、长期记忆（关键事件重要性评分索引）。

#### Scenario: 事件写入短期记忆

- **WHEN** Agent执行动作或遭受事件
- **THEN** 系统 SHALL 将事件写入短期记忆（含时间戳、类型、内容文本、情感标签）
- **AND** 短期记忆 SHALL 保留最近5条，超出时最旧的移入中期

### Requirement: 中期记忆压缩

系统 SHALL 当短期记忆溢出时，将旧记忆通过LLM压缩为摘要存入中期记忆。中期记忆总量控制在800 tokens内。

#### Scenario: 短期→中期压缩

- **WHEN** 短期记忆已有5条，新事件写入
- **THEN** 最旧1条 SHALL 被压缩为摘要
- **AND** 摘要 SHALL 包含关键事实、情感变化、动机影响

#### Scenario: 中期记忆溢出

- **WHEN** 中期记忆超过800 tokens
- **THEN** 系统 SHALL 对早期摘要执行二次压缩（摘要的摘要）
- **AND** 保留最重要的关键节点

### Requirement: 长期记忆索引

系统 SHALL 为重要性评分 > 0.7的事件建立长期记忆索引，用于Prompt构建时的关键事件检索。MVP阶段使用简单向量相似度搜索。

#### Scenario: 高重要性事件持久化

- **WHEN** 事件的重要性评分 > 0.7（如首次交易、遭受攻击、发现遗迹）
- **THEN** 系统 SHALL 将事件写入长期记忆索引

#### Scenario: 记忆检索

- **WHEN** 构建决策Prompt时
- **THEN** 系统 SHALL 从长期记忆中检索与当前Spark最相关的1~3条事件

### Requirement: 记忆总量控制

系统 SHALL 确保进入Prompt的记忆部分不超过1800 tokens，优先保留短期记忆和长期关键检索。

#### Scenario: 记忆截断

- **WHEN** 短期+中期+长期检索合计超过1800 tokens
- **THEN** 系统 SHALL 优先保留短期记忆完整和长期关键检索
- **AND** 截断中期记忆摘要

### Requirement: 记忆遗忘

系统 SHALL 对长期记忆执行时间衰减：每50个tick，重要性评分降低5%。低于0.3的长期记忆 SHALL 被丢弃。

#### Scenario: 时间衰减

- **WHEN** 每50个tick到达
- **THEN** 所有长期记忆的重要性评分 SHALL 乘以0.95

#### Scenario: 记忆淘汰

- **WHEN** 长期记忆重要性评分降至 < 0.3
- **THEN** 该记忆 SHALL 被删除