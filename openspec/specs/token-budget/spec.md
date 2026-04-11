# 功能规格说明：记忆总量控制

## Purpose

定义记忆系统进入 Prompt 的预算分配和截断策略，确保总记忆内容不超过设定的 token 限制。

## Requirements

### Requirement: 记忆预算分配

系统 SHALL 确保进入 Prompt 的记忆部分不超过 1800 chars，按优先级分配空间。

#### Scenario: 预算分配

- **WHEN** 构建决策 Prompt
- **THEN** 记忆空间 SHALL 按以下优先级分配:
  - ChronicleStore 快照：800 chars（固定）
  - ChronicleDB FTS5 检索：600 chars（动态）
  - StrategyHub 策略摘要：400 chars（动态）

### Requirement: 截断策略

系统 SHALL 在总记忆内容超过 1800 chars 时执行截断。

#### Scenario: 优先级截断

- **WHEN** 总记忆内容超过 1800 chars
- **THEN** 系统 SHALL 按以下顺序截断:
  1. 截断 ChronicleDB 检索结果（保留 top 1）
  2. 截断 StrategyHub 策略（保留 metadata only）
  3. 截断 ChronicleStore（保留最近 3 entries）

#### Scenario: ChronicleDB 截断

- **WHEN** ChronicleDB 检索结果超过 600 chars
- **THEN** 系统 SHALL 围绕匹配词截断每个片段
- **AND** 每个片段 SHALL 不超过 200 chars

### Requirement: TokenBudget 控制器

系统 SHALL 实现 TokenBudget 控制器，管理记忆预算分配。

#### Scenario: 预算计算

- **WHEN** 添加记忆内容到 Prompt
- **THEN** 系统 SHALL 跟踪已使用预算
- **AND** 超预算时 SHALL 拒绝添加或执行截断

#### Scenario: 预算重置

- **WHEN** 新 tick 开始
- **THEN** 系统 SHALL 重置预算计数器
- **AND** 重新计算各部分预算
