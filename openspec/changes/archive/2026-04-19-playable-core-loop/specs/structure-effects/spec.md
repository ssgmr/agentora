# 功能规格说明 — structure-effects

## ADDED Requirements

### Requirement: Camp恢复HP效果

位于Camp建筑所在格或相邻格（曼哈顿距离≤1）的存活Agent SHALL 每 tick 恢复2点HP，不超过max_health。

#### Scenario: Agent在Camp格上回血

- **WHEN** Agent位于Camp同格
- **AND** Agent HP < max_health
- **THEN** Agent HP +2（不超过上限）

#### Scenario: Agent在Camp相邻格回血

- **WHEN** Agent与Camp曼哈顿距离=1
- **AND** Agent HP < max_health
- **THEN** Agent HP +2

#### Scenario: Agent远离Camp不回血

- **WHEN** Agent与最近Camp的曼哈顿距离≥2
- **THEN** Agent HP不因Camp效果恢复

#### Scenario: 满血时不回血

- **WHEN** Agent在Camp范围内
- **AND** Agent HP = max_health
- **THEN** HP不变

#### Scenario: 多个Camp效果不叠加

- **WHEN** Agent在两个Camp的覆盖范围内
- **THEN** Agent每tick仍只恢复2HP（不叠加）

### Requirement: Fence阻挡敌对Agent

Fence所在格 SHALL 阻挡与Fence所有者关系为Enemy的Agent通行。中立和盟友Agent可正常通行。

#### Scenario: 敌对Agent被阻挡

- **WHEN** 敌对Agent尝试Move进入Fence所在格
- **THEN** 移动被拒绝，Agent停留在原位

#### Scenario: 中立Agent可通过

- **WHEN** 中立Agent（关系为Neutral）尝试Move进入Fence所在格
- **THEN** 移动成功

#### Scenario: 盟友Agent可通过

- **WHEN** 盟友Agent尝试Move进入Fence所在格
- **THEN** 移动成功

#### Scenario: Fence所有者可通过

- **WHEN** Fence的建造者尝试Move进入Fence所在格
- **THEN** 移动成功

#### Scenario: 无所有者的Fence

- **WHEN** Fence无明确所有者
- **AND** 任意Agent尝试通过
- **THEN** 所有Agent可正常通行

### Requirement: Warehouse扩展库存

位于Warehouse建筑所在格或相邻格的Agent SHALL 获得库存上限+20（从20提升至40）。

#### Scenario: Agent在Warehouse旁采集

- **WHEN** Agent与Warehouse曼哈顿距离≤1
- **AND** Agent执行Gather动作
- **THEN** 库存上限为40而非20

#### Scenario: Agent离开Warehouse范围

- **WHEN** Agent从Warehouse旁移动到远处
- **THEN** 库存上限恢复为20
- **AND** 超出上限的资源保留但不能再采集

#### Scenario: 多个Warehouse不叠加

- **WHEN** Agent同时在多个Warehouse范围内
- **THEN** 库存上限仍为40（效果不叠加）

### Requirement: 建筑效果推送

建筑效果的触发 SHALL 通过Bridge Delta推送到Godot客户端，便于UI展示。

#### Scenario: Agent因Camp回血

- **WHEN** Agent在Camp范围内恢复HP
- **THEN** 推送HealedByCamp delta事件