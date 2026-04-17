# 功能规格说明 — world-model (修改)

## MODIFIED Requirements

### Requirement: advance_tick扩展

advance_tick SHALL 在现有逻辑之前新增生存消耗和建筑效果处理，在现有逻辑之后新增压力事件生成。

#### Scenario: advance_tick执行顺序

- **WHEN** advance_tick执行
- **THEN** 按顺序执行：1.生存消耗(satiety/hydration衰减+饥饿掉血) 2.建筑效果(Camp回血) 3.动机衰减(原有) 4.临时偏好衰减(原有) 5.压力tick(激活版) 6.死亡检查(原有) 7.遗产衰减(原有) 8.策略衰减(原有)

### Requirement: pressure_tick升级

pressure_tick SHALL 从TODO状态升级为实际生成压力事件，每40-80 tick随机触发，影响资源产出和Agent状态。

#### Scenario: 压力事件生成

- **WHEN** tick到达随机间隔点（40-80）
- **THEN** 生成随机类型压力事件（干旱/丰饶/瘟疫），加入pressure_pool

#### Scenario: 压力事件影响资源

- **WHEN** 干旱事件激活
- **THEN** Water节点gather产出乘以0.5

#### Scenario: 压力事件影响Agent

- **WHEN** 瘟疫事件激活
- **THEN** 随机Agent HP -20

## ADDED Requirements

### Requirement: 世界维护里程碑列表

World结构体 SHALL 新增 `milestones: Vec<Milestone>` 字段和 `next_pressure_tick: u64` 字段。

#### Scenario: 世界初始化

- **WHEN** 创建新World
- **THEN** milestones为空Vec, next_pressure_tick为40-80之间的随机值