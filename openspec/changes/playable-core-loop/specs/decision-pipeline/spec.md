# 功能规格说明 — decision-pipeline (修改)

## MODIFIED Requirements

### Requirement: Prompt注入生存状态

决策Prompt感知段 SHALL 包含Agent当前的satiety和hydration数值及状态描述（"饥饿中"/"口渴中"/"正常"）。

#### Scenario: 正常状态

- **WHEN** Agent satiety > 30, hydration > 30
- **THEN** Prompt包含"饱食度: {satiety}/100, 水分度: {hydration}/100, 状态: 正常"

#### Scenario: 饥饿状态

- **WHEN** Agent satiety ≤ 30
- **THEN** Prompt包含"饱食度: {satiety}/100, 状态: 饥饿中！需要寻找食物"

#### Scenario: 口渴状态

- **WHEN** Agent hydration ≤ 30
- **THEN** Prompt包含"水分度: {hydration}/100, 状态: 口渴中！需要寻找水源"

### Requirement: Prompt注入压力事件

决策Prompt感知段 SHALL 包含当前所有活跃压力事件的描述。

#### Scenario: 有活跃压力事件

- **WHEN** pressure_pool中有干旱事件
- **THEN** Prompt包含"当前世界事件: 干旱来袭，水源产出减半"

#### Scenario: 无压力事件

- **WHEN** pressure_pool为空
- **THEN** Prompt不包含压力事件信息