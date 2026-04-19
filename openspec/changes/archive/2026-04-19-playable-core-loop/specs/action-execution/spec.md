# 功能规格说明 — action-execution (修改)

## MODIFIED Requirements

### Requirement: Wait动作行为变更

Wait动作 SHALL 从"恢复5 HP"改为"专注饮食"：尝试消耗1 Food恢复30 satiety，消耗1 Water恢复25 hydration。HP不再通过Wait直接恢复。

#### Scenario: Wait有食物和水

- **WHEN** Agent执行Wait动作
- **AND** 背包Food ≥ 1, Water ≥ 1
- **THEN** consume 1 Food, satiety +30; consume 1 Water, hydration +25; HP不变

#### Scenario: Wait无资源

- **WHEN** Agent执行Wait动作
- **AND** 背包无Food也无Water
- **THEN** satiety不变, hydration不变, HP不变（纯休息）

### Requirement: Move动作新增Fence碰撞

Move动作判断 SHALL 新增Fence阻挡检查：目标格如果有Fence且Agent与Fence所有者为敌对关系，则移动被拒绝。

#### Scenario: 敌对Agent被Fence阻挡

- **WHEN** Agent尝试Move到Fence所在格
- **AND** Agent与Fence所有者为Enemy关系
- **THEN** 动作被Blocked，返回错误叙事

#### Scenario: 非敌对Agent不被阻挡

- **WHEN** Agent尝试Move到Fence所在格
- **AND** Agent与Fence所有者非Enemy关系
- **THEN** 移动正常执行