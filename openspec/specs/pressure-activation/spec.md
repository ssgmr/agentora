# Pressure Activation Spec

## Purpose

定义环境压力事件的自动生成、影响机制、决策 Prompt 注入和叙事推送。

## Requirements

### Requirement: 压力事件自动生成

世界 SHALL 每 40-80 tick 随机生成一个压力事件。事件类型从 [干旱，丰饶，瘟疫] 中随机选择。

#### Scenario: 定期生成事件

- **WHEN** 世界 tick 到达下次事件触发点（40-80 tick 随机间隔）
- **THEN** 生成一个随机类型的新 PressureEvent 并加入 pressure_pool

#### Scenario: 事件不重叠过多

- **WHEN** pressure_pool 中已有 3 个活跃事件
- **THEN** 不生成新事件，推迟到下次检查

### Requirement: 干旱事件

干旱事件 SHALL 使所有 Water 类型 ResourceNode 的产出效率降低 50%，持续 30 tick。事件描述为"干旱来袭，水源产出减半"。

#### Scenario: 干旱影响水资源

- **WHEN** 干旱事件激活
- **THEN** 所有 Water ResourceNode 的 gather 产出减半

#### Scenario: 干旱结束后恢复

- **WHEN** 干旱事件 remaining_ticks 减至 0
- **THEN** 所有 Water ResourceNode 恢复正常产出

### Requirement: 丰饶事件

丰饶事件 SHALL 使所有 Food 类型 ResourceNode 的当前量翻倍（不超过 max_amount），持续 20 tick。事件描述为"丰收季节，食物产出大增"。

#### Scenario: 丰饶增加食物

- **WHEN** 丰饶事件激活
- **THEN** 所有 Food ResourceNode 的 current_amount 翻倍（不超过 max_amount）

#### Scenario: 丰饶结束后不回退

- **WHEN** 丰饶事件结束
- **THEN** 已增加的食物量保持不变（不回退），但不再自动翻倍

### Requirement: 瘟疫事件

瘟疫事件 SHALL 使随机 1-3 个存活 Agent 立即损失 20 HP。事件为单次触发（duration=1 tick）。

#### Scenario: 瘟疫影响随机 Agent

- **WHEN** 瘟疫事件激活
- **THEN** 随机 1-3 个 Agent 各 HP -20（不低于 0）

#### Scenario: 瘟疫可致死

- **WHEN** 瘟疫事件激活
- **AND** 受影响 Agent HP ≤ 20
- **THEN** Agent HP 变为 0，触发死亡流程

### Requirement: 压力事件注入决策 Prompt

当前活跃的压力事件 SHALL 注入 Agent 决策 Prompt 的感知段，格式为"当前世界事件：{事件描述}"。

#### Scenario: Prompt 包含压力信息

- **WHEN** pressure_pool 中有活跃事件
- **AND** Agent 进行 LLM 决策
- **THEN** Prompt 感知段包含所有活跃事件描述

#### Scenario: 无事件时不注入

- **WHEN** pressure_pool 为空
- **THEN** Prompt 不包含压力事件信息

### Requirement: 压力事件叙事推送

压力事件的生成和结束 SHALL 推送 NarrativeEvent 到 Godot 客户端，在叙事流中展示。

#### Scenario: 事件生成时推送

- **WHEN** 新压力事件生成
- **THEN** 推送 PressureStarted NarrativeEvent，包含事件类型和描述

#### Scenario: 事件结束时推送

- **WHEN** 压力事件 remaining_ticks 减至 0
- **THEN** 推送 PressureEnded NarrativeEvent
