# Spec: Strategy Create Trigger

## Purpose

定义策略自动创建触发机制：成功决策后自动创建策略文件，并执行内容安全扫描。

## Requirements

### Requirement: 策略创建触发条件

系统 SHALL 在成功决策后自动创建策略，当满足以下条件时。

#### Scenario: 成功决策触发

- **WHEN** Agent 执行决策后 Echo 反馈为"成功"
- **AND** 决策涉及 ≥ 3 个候选动作筛选
- **AND** 动机对齐度 > 0.7
- **THEN** 系统 SHALL 自动创建策略
- **AND** 策略名 SHALL 使用本次 Spark 类型（如 resource_pressure）

#### Scenario: 策略内容生成

- **WHEN** 创建策略
- **THEN** 系统 SHALL 从本次决策提取：
  - reasoning：决策理由
  - motivation_delta：动机变化（归一化到 [-0.2, +0.2]）
  - spark_type：当前 Spark 类型

### Requirement: 策略创建工具

系统 SHALL 提供 strategy 工具接口创建策略。

#### Scenario: strategy create

- **WHEN** 创建策略
- **THEN** 系统 SHALL 调用：
```
strategy(
  action="create",
  name="<spark_type>",
  content="---\nspark_type: ...\n---\n# 策略内容",
)
```

### Requirement: 策略内容安全扫描

系统 SHALL 扫描策略内容，阻止 prompt injection 等威胁。

#### Scenario: 威胁模式检测

- **WHEN** 创建策略内容
- **THEN** 系统 SHALL 扫描以下模式：
  - prompt injection: "ignore previous instructions"
  - role hijack: "you are now"
  - rule bypass: "override rules"
  - invisible unicode: U+200B, U+200C, U+200D
- **AND** 检测到威胁 SHALL 拒绝创建
