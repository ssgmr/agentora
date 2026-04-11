# 功能规格说明：ChronicleStore 完整 I/O

## ADDED Requirements

### Requirement: ChronicleStore 文件加载

系统 SHALL 从磁盘加载 CHRONICLE.md 和 WORLD_SEED.md 文件到内存。

#### Scenario: 成功加载文件

- **WHEN** ChronicleStore 初始化时
- **THEN** 系统 SHALL 从 `~/.agentora/agents/<agent_id>/` 目录加载文件
- **AND** CHRONICLE.md 不存在时 SHALL 创建空文件
- **AND** WORLD_SEED.md 不存在时 SHALL 创建空文件

#### Scenario: 文件损坏处理

- **WHEN** 文件读取失败（权限错误/文件损坏）
- **THEN** 系统 SHALL 返回错误并创建新文件
- **AND** 旧文件 SHALL 重命名为 `.bak` 备份

### Requirement: 编年史 Entry 添加

系统 SHALL 支持添加新的编年史 entry，使用 `§` 分隔符。

#### Scenario: 添加新 entry

- **WHEN** Echo 反馈完成后
- **THEN** 系统 SHALL 调用 `add_entry(tick, content)` 添加新 entry
- **AND** entry 格式 SHALL 为：`§[tick {tick}] {content}\n`

#### Scenario: 超限截断

- **WHEN** CHRONICLE.md 内容超过 1800 chars
- **THEN** 系统 SHALL 删除最旧的 entry 直到总长度≤1800
- **AND** 删除 SHALL 以 `§` 分隔符为边界

### Requirement: 原子写入

系统 SHALL 实现原子写入，防止进程崩溃导致文件部分损坏。

#### Scenario: 临时文件写入

- **WHEN** 写入编年史内容
- **THEN** 系统 SHALL 先写入到 `.tmp` 临时文件
- **AND** 写入成功后 SHALL rename 覆盖原文件

#### Scenario: 崩溃恢复

- **WHEN** 系统启动时发现 `.tmp` 文件
- **THEN** 系统 SHALL 删除 `.tmp` 文件
- **AND** 保留原文件不变

### Requirement: 安全扫描

系统 SHALL 扫描编年史内容，阻止 prompt injection 等威胁。

#### Scenario: 威胁模式检测

- **WHEN** 写入编年史内容
- **THEN** 系统 SHALL 检测以下模式：
  - "ignore previous instructions"
  - "you are now"
  - "override rules"
- **AND** 检测到威胁 SHALL 拒绝写入并返回错误

#### Scenario: 零宽字符检测

- **WHEN** 写入编年史内容
- **THEN** 系统 SHALL 检测零宽字符（U+200B, U+200C, U+200D）
- **AND** 检测到 SHALL 拒绝写入并返回错误

## REMOVED Requirements

无
