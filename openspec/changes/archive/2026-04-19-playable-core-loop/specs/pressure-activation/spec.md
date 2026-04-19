# 功能规格说明 — pressure-activation

## ADDED Requirements

### Requirement: 压力事件自动生成

世界 SHALL 每40-80 tick随机生成一个压力事件。事件类型从[干旱, 丰饶, 瘟疫]中随机选择。

#### Scenario: 定期生成事件

- **WHEN** 世界tick到达下次事件触发点（40-80 tick随机间隔）
- **THEN** 生成一个随机类型的新PressureEvent并加入pressure_pool

#### Scenario: 事件不重叠过多

- **WHEN** pressure_pool中已有3个活跃事件
- **THEN** 不生成新事件，推迟到下次检查

### Requirement: 干旱事件

干旱事件 SHALL 使所有Water类型ResourceNode的产出效率降低50%，持续30 tick。事件描述为"干旱来袭，水源产出减半"。

#### Scenario: 干旱影响水资源

- **WHEN** 干旱事件激活
- **THEN** 所有Water ResourceNode的gather产出减半

#### Scenario: 干旱结束后恢复

- **WHEN** 干旱事件remaining_ticks减至0
- **THEN** 所有Water ResourceNode恢复正常产出

### Requirement: 丰饶事件

丰饶事件 SHALL 使所有Food类型ResourceNode的当前量翻倍（不超过max_amount），持续20 tick。事件描述为"丰收季节，食物产出大增"。

#### Scenario: 丰饶增加食物

- **WHEN** 丰饶事件激活
- **THEN** 所有Food ResourceNode的current_amount翻倍（不超过max_amount）

#### Scenario: 丰饶结束后不回退

- **WHEN** 丰饶事件结束
- **THEN** 已增加的食物量保持不变（不回退），但不再自动翻倍

### Requirement: 瘟疫事件

瘟疫事件 SHALL 使随机1-3个存活Agent立即损失20 HP。事件为单次触发（duration=1 tick）。

#### Scenario: 瘟疫影响随机Agent

- **WHEN** 瘟疫事件激活
- **THEN** 随机1-3个Agent各HP -20（不低于0）

#### Scenario: 瘟疫可致死

- **WHEN** 瘟疫事件激活
- **AND** 受影响Agent HP ≤ 20
- **THEN** Agent HP变为0，触发死亡流程

### Requirement: 压力事件注入决策Prompt

当前活跃的压力事件 SHALL 注入Agent决策Prompt的感知段，格式为"当前世界事件：{事件描述}"。

#### Scenario: Prompt包含压力信息

- **WHEN** pressure_pool中有活跃事件
- **AND** Agent进行LLM决策
- **THEN** Prompt感知段包含所有活跃事件描述

#### Scenario: 无事件时不注入

- **WHEN** pressure_pool为空
- **THEN** Prompt不包含压力事件信息

### Requirement: 压力事件叙事推送

压力事件的生成和结束 SHALL 推送NarrativeEvent到Godot客户端，在叙事流中展示。

#### Scenario: 事件生成时推送

- **WHEN** 新压力事件生成
- **THEN** 推送PressureStarted NarrativeEvent，包含事件类型和描述

#### Scenario: 事件结束时推送

- **WHEN** 压力事件remaining_ticks减至0
- **THEN** 推送PressureEnded NarrativeEvent