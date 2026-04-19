# Survival Consumption Spec

## Purpose

定义 Agent 的饱食度 (satiety) 和水分度 (hydration) 系统，包括自动衰减、饥饿掉血、Wait 动作饮食化、以及生存压力对动机的影响。

## Requirements

### Requirement: Agent 饱食度与水分度

每个 Agent SHALL 持有两个生存指标：satiety（饱食度，0-100）和 hydration（水分度，0-100），初始值均为 100。

#### Scenario: Agent 出生时生存指标满值

- **WHEN** 创建新 Agent
- **THEN** satiety = 100, hydration = 100

#### Scenario: 生存指标不超过上限

- **WHEN** Agent 饱食度或水分度已达 100
- **AND** Agent 消耗食物或水
- **THEN** 指标 SHALL 不超过 100（截断至上限）

### Requirement: 生存指标自动衰减

每个世界 tick，所有存活 Agent 的 satiety SHALL 减少 2 点，hydration SHALL 减少 2.5 点。衰减后最低为 0，不会变为负数。

#### Scenario: 正常衰减

- **WHEN** 世界 advance_tick 执行
- **THEN** 每个存活 Agent 的 satiety 减少 2，hydration 减少 2.5

#### Scenario: 衰减不低于零

- **WHEN** Agent 的 satiety 为 1
- **AND** 世界 advance_tick 执行
- **THEN** satiety 变为 0（不取负值）

### Requirement: 饥饿与口渴导致 HP 下降

当 Agent 的 satiety 为 0 时，每 tick SHALL 扣除 2 点 HP。当 Agent 的 hydration 为 0 时，每 tick SHALL 扣除 3 点 HP。两者可叠加：同时为 0 时每 tick 共扣 5HP。

#### Scenario: 仅饥饿掉血

- **WHEN** Agent satiety = 0, hydration > 0
- **AND** 世界 advance_tick 执行
- **THEN** Agent HP 减少 2

#### Scenario: 仅口渴掉血

- **WHEN** Agent satiety > 0, hydration = 0
- **AND** 世界 advance_tick 执行
- **THEN** Agent HP 减少 3

#### Scenario: 饥渴叠加掉血

- **WHEN** Agent satiety = 0, hydration = 0
- **AND** 世界 advance_tick 执行
- **THEN** Agent HP 减少 5

#### Scenario: 生存指标正常时不掉血

- **WHEN** Agent satiety > 0, hydration > 0
- **AND** 世界 advance_tick 执行
- **THEN** Agent HP 不因饥饿/口渴减少

### Requirement: Wait 动作改为专注饮食

Wait 动作 SHALL 尝试消耗背包中的 1 单位 Food 恢复 30 点 satiety，消耗 1 单位 Water 恢复 25 点 hydration。两者可以同时消耗。Wait 动作 SHALL 不再直接恢复 HP。

#### Scenario: 同时有食物和水

- **WHEN** Agent 执行 Wait 动作
- **AND** 背包有 Food ≥ 1 且 Water ≥ 1
- **THEN** 消耗 1 Food, satiety +30; 消耗 1 Water, hydration +25; HP 不变

#### Scenario: 仅有食物

- **WHEN** Agent 执行 Wait 动作
- **AND** 背包有 Food ≥ 1 且 Water = 0
- **THEN** 消耗 1 Food, satiety +30; hydration 不变; HP 不变

#### Scenario: 仅有水

- **WHEN** Agent 执行 Wait 动作
- **AND** 背包 Food = 0 且 Water ≥ 1
- **THEN** 消耗 1 Water, hydration +25; satiety 不变; HP 不变

#### Scenario: 无食物也无水

- **WHEN** Agent 执行 Wait 动作
- **AND** 背包 Food = 0 且 Water = 0
- **THEN** satiety 和 hydration 均不变; Wait 动作仍可执行（仅休息）

### Requirement: 生存压力感知

当 satiety ≤ 30 或 hydration ≤ 30 时，Agent 的生存动机维度 (维度 0) SHALL 临时增加 0.3。satiety = 0 或 hydration = 0 时 SHALL 临时增加 0.5。

#### Scenario: 饥饿驱动生存动机

- **WHEN** Agent satiety = 20, hydration = 60
- **THEN** 生存动机维度额外 +0.3

#### Scenario: 极度饥渴驱动

- **WHEN** Agent satiety = 0, hydration = 0
- **THEN** 生存动机维度额外 +0.5

### Requirement: 生存状态序列化

Agent 的 satiety 和 hydration SHALL 包含在 WorldSnapshot 中，通过 Bridge 推送到 Godot 客户端。

#### Scenario: 快照包含生存指标

- **WHEN** 生成 WorldSnapshot
- **THEN** 每个 AgentSnapshot 包含 satiety 和 hydration 字段
