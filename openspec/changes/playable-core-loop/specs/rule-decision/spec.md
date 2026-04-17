# 功能规格说明 — rule-decision (修改)

## MODIFIED Requirements

### Requirement: NPC生存需求决策

NPC Agent通过RuleEngine决策时 SHALL 优先检查satiety和hydration状态，低于阈值时优先选择满足饮食需求的行为。

#### Scenario: 饥饿时优先采集食物

- **WHEN** NPC satiety ≤ 30
- **AND** 附近有Food ResourceNode
- **THEN** RuleEngine选择Move前往食物节点 + Gather

#### Scenario: 口渴时优先采集水

- **WHEN** NPC hydration ≤ 30
- **AND** 附近有Water ResourceNode
- **THEN** RuleEngine选择Move前往水源 + Gather

#### Scenario: 饥饿且背包有食物

- **WHEN** NPC satiety ≤ 30
- **AND** 背包有Food ≥ 1
- **THEN** RuleEngine选择Wait（消耗食物恢复饱食度）

#### Scenario: 生存需求优先于其他动机

- **WHEN** NPC satiety = 0 或 hydration = 0
- **THEN** 即使最高动机维度非生存，也优先满足饮食需求