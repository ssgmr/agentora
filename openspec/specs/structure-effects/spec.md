# Structure Effects Spec

## Purpose

定义建筑（Camp、Fence、Warehouse）对周围 Agent 产生的被动效果，以及效果事件的 Delta 推送。

## Requirements

### Requirement: Camp 恢复 HP 效果

位于 Camp 建筑所在格或相邻格（曼哈顿距离 ≤1）的存活 Agent SHALL 每 tick 恢复 2 点 HP，不超过 max_health。

#### Scenario: Agent 在 Camp 格上回血

- **WHEN** Agent 位于 Camp 同格
- **AND** Agent HP < max_health
- **THEN** Agent HP +2（不超过上限）

#### Scenario: Agent 在 Camp 相邻格回血

- **WHEN** Agent 与 Camp 曼哈顿距离 =1
- **AND** Agent HP < max_health
- **THEN** Agent HP +2

#### Scenario: Agent 远离 Camp 不回血

- **WHEN** Agent 与最近 Camp 的曼哈顿距离 ≥2
- **THEN** Agent HP 不因 Camp 效果恢复

#### Scenario: 满血时不回血

- **WHEN** Agent 在 Camp 范围内
- **AND** Agent HP = max_health
- **THEN** HP 不变

#### Scenario: 多个 Camp 效果不叠加

- **WHEN** Agent 在两个 Camp 的覆盖范围内
- **THEN** Agent 每 tick 仍只恢复 2HP（不叠加）

### Requirement: Fence 阻挡敌对 Agent

Fence 所在格 SHALL 阻挡与 Fence 所有者关系为 Enemy 的 Agent 通行。中立和盟友 Agent 可正常通行。

#### Scenario: 敌对 Agent 被阻挡

- **WHEN** 敌对 Agent 尝试 Move 进入 Fence 所在格
- **THEN** 移动被拒绝，Agent 停留在原位

#### Scenario: 中立 Agent 可通过

- **WHEN** 中立 Agent（关系为 Neutral）尝试 Move 进入 Fence 所在格
- **THEN** 移动成功

#### Scenario: 盟友 Agent 可通过

- **WHEN** 盟友 Agent 尝试 Move 进入 Fence 所在格
- **THEN** 移动成功

#### Scenario: Fence 所有者可通过

- **WHEN** Fence 的建造者尝试 Move 进入 Fence 所在格
- **THEN** 移动成功

#### Scenario: 无所有者的 Fence

- **WHEN** Fence 无明确所有者
- **AND** 任意 Agent 尝试通过
- **THEN** 所有 Agent 可正常通行

### Requirement: Warehouse 扩展库存

位于 Warehouse 建筑所在格或相邻格的 Agent SHALL 获得库存上限 +20（从 20 提升至 40）。

#### Scenario: Agent 在 Warehouse 旁采集

- **WHEN** Agent 与 Warehouse 曼哈顿距离 ≤1
- **AND** Agent 执行 Gather 动作
- **THEN** 库存上限为 40 而非 20

#### Scenario: Agent 离开 Warehouse 范围

- **WHEN** Agent 从 Warehouse 旁移动到远处
- **THEN** 库存上限恢复为 20
- **AND** 超出上限的资源保留但不能再采集

#### Scenario: 多个 Warehouse 不叠加

- **WHEN** Agent 同时在多个 Warehouse 范围内
- **THEN** 库存上限仍为 40（效果不叠加）

### Requirement: 建筑效果推送

建筑效果的触发 SHALL 通过 Bridge Delta 推送到 Godot 客户端，便于 UI 展示。

#### Scenario: Agent 因 Camp 回血

- **WHEN** Agent 在 Camp 范围内恢复 HP
- **THEN** 推送 HealedByCamp delta 事件
