# Spec: Strategy Persistence

## Purpose

定义策略文件持久化机制：使用 Markdown + YAML frontmatter 格式存储策略，支持原子写入和加载。

## Requirements

### Requirement: 策略文件存储

系统 SHALL 为每个 Agent 维护策略文件目录，使用 Markdown + YAML frontmatter 格式。

#### Scenario: 策略目录结构

- **WHEN** 创建新策略
- **THEN** 系统 SHALL 创建目录 `~/.agentora/agents/<agent_id>/strategies/<spark_type>/`
- **AND** 目录名 SHALL 使用小写字母和下划线（如 resource_pressure）

#### Scenario: STRATEGY.md 格式

- **WHEN** 创建 STRATEGY.md 文件
- **THEN** 文件 SHALL 以 YAML frontmatter 开头（--- 包裹）
- **AND** frontmatter SHALL 包含必填字段：spark_type, success_rate, use_count, last_used_tick, created_tick
- **AND** frontmatter MAY 包含可选字段：deprecated, motivation_delta
- **AND** 正文 SHALL 包含策略内容（触发条件、推荐动作、成功条件）

### Requirement: 策略文件读取

系统 SHALL 从磁盘加载策略文件，解析 YAML frontmatter 和内容。

#### Scenario: 加载策略文件

- **WHEN** 检索策略时
- **THEN** 系统 SHALL 读取 STRATEGY.md 文件
- **AND** 解析 YAML frontmatter 为 Strategy 结构体
- **AND** 解析正文为 content 字段

#### Scenario: 文件不存在处理

- **WHEN** 策略文件不存在
- **THEN** 系统 SHALL 返回 None
- **AND** 不创建空文件

### Requirement: 策略文件写入

系统 SHALL 将策略保存到磁盘，使用原子写入防止损坏。

#### Scenario: 保存策略文件

- **WHEN** 创建或更新策略
- **THEN** 系统 SHALL 写入 STRATEGY.md 文件
- **AND** 使用临时文件 + rename 实现原子性
- **AND** YAML frontmatter 和正文 SHALL 一起写入
