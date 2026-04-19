# 功能规格说明 — survival-consumption

## ADDED Requirements

### Requirement: Agent饱食度与水分度

每个Agent SHALL 持有两个生存指标：satiety（饱食度，0-100）和 hydration（水分度，0-100），初始值均为100。

#### Scenario: Agent出生时生存指标满值

- **WHEN** 创建新Agent
- **THEN** satiety = 100, hydration = 100

#### Scenario: 生存指标不超过上限

- **WHEN** Agent饱食度或水分度已达100
- **AND** Agent消耗食物或水
- **THEN** 指标 SHALL 不超过100（截断至上限）

### Requirement: 生存指标自动衰减

每个世界tick，所有存活Agent的satiety SHALL 减少2点，hydration SHALL 减少2.5点。衰减后最低为0，不会变为负数。

#### Scenario: 正常衰减

- **WHEN** 世界advance_tick执行
- **THEN** 每个存活Agent的satiety减少2，hydration减少2.5

#### Scenario: 衰减不低于零

- **WHEN** Agent的satiety为1
- **AND** 世界advance_tick执行
- **THEN** satiety变为0（不取负值）

### Requirement: 饥饿与口渴导致HP下降

当Agent的satiety为0时，每tick SHALL 扣除2点HP。当Agent的hydration为0时，每tick SHALL 扣除3点HP。两者可叠加：同时为0时每tick共扣5HP。

#### Scenario: 仅饥饿掉血

- **WHEN** Agent satiety = 0, hydration > 0
- **AND** 世界advance_tick执行
- **THEN** Agent HP减少2

#### Scenario: 仅口渴掉血

- **WHEN** Agent satiety > 0, hydration = 0
- **AND** 世界advance_tick执行
- **THEN** Agent HP减少3

#### Scenario: 饥渴叠加掉血

- **WHEN** Agent satiety = 0, hydration = 0
- **AND** 世界advance_tick执行
- **THEN** Agent HP减少5

#### Scenario: 生存指标正常时不掉血

- **WHEN** Agent satiety > 0, hydration > 0
- **AND** 世界advance_tick执行
- **THEN** Agent HP不因饥饿/口渴减少

### Requirement: Wait动作改为专注饮食

Wait动作 SHALL 尝试消耗背包中的1单位Food恢复30点satiety，消耗1单位Water恢复25点hydration。两者可以同时消耗。Wait动作 SHALL 不再直接恢复HP。

#### Scenario: 同时有食物和水

- **WHEN** Agent执行Wait动作
- **AND** 背包有Food ≥ 1且Water ≥ 1
- **THEN** 消耗1 Food, satiety +30; 消耗1 Water, hydration +25; HP不变

#### Scenario: 仅有食物

- **WHEN** Agent执行Wait动作
- **AND** 背包有Food ≥ 1且Water = 0
- **THEN** 消耗1 Food, satiety +30; hydration不变; HP不变

#### Scenario: 仅有水

- **WHEN** Agent执行Wait动作
- **AND** 背包Food = 0且Water ≥ 1
- **THEN** 消耗1 Water, hydration +25; satiety不变; HP不变

#### Scenario: 无食物也无水

- **WHEN** Agent执行Wait动作
- **AND** 背包Food = 0且Water = 0
- **THEN** satiety和hydration均不变; Wait动作仍可执行（仅休息）

### Requirement: 生存压力感知

当satiety ≤ 30或hydration ≤ 30时，Agent的生存动机维度(维度0) SHALL 临时增加0.3。satiety = 0或hydration = 0时 SHALL 临时增加0.5。

#### Scenario: 饥饿驱动生存动机

- **WHEN** Agent satiety = 20, hydration = 60
- **THEN** 生存动机维度额外 +0.3

#### Scenario: 极度饥渴驱动

- **WHEN** Agent satiety = 0, hydration = 0
- **THEN** 生存动机维度额外 +0.5

### Requirement: 生存状态序列化

Agent的satiety和hydration SHALL 包含在WorldSnapshot中，通过Bridge推送到Godot客户端。

#### Scenario: 快照包含生存指标

- **WHEN** 生成WorldSnapshot
- **THEN** 每个AgentSnapshot包含satiety和hydration字段