# 功能规格说明：回响日志压缩

## ADDED Requirements

### Requirement: LLM 记忆压缩

系统 SHALL 在 Agent 死亡时使用 LLM 压缩最后 3 条短期记忆为回响日志。

#### Scenario: 压缩短期记忆

- **WHEN** Agent 死亡（生命≤0 或年龄≥200 tick）
- **THEN** 系统 SHALL 获取最后 3 条短期记忆
- **AND** 构建 Prompt 请求 LLM 压缩为摘要

#### Scenario: 提取情感标签

- **WHEN** 压缩记忆时
- **THEN** 系统 SHALL 从原始记忆中提取情感标签
- **AND** 情感标签 SHALL 反映 Agent 死亡时的情感状态

### Requirement: 回响日志格式

系统 SHALL 将回响日志保存到遗产实体。

#### Scenario: 回响日志结构

- **WHEN** 创建回响日志
- **THEN** 日志 SHALL 包含:
  - summary: 压缩摘要
  - emotion_tags: 情感标签数组
  - final_words: 遗言（可选）
  - key_memories: 关键记忆列表

## REMOVED Requirements

无
