# Bridge API

## Purpose

定义 WorldSnapshot 数据填充、序列化以及 Godot 端事件分发的完整实现规范。

## Requirements

### Requirement: WorldSnapshot 数据填充

`World::snapshot()` SHALL 填充完整的快照数据，包括地图变更、事件、遗产和压力信息。

#### Scenario: Agent 事件填充

- **WHEN** 调用 `World::snapshot()`
- **THEN** 快照的 `events` 字段 SHALL 包含本 tick 内所有 Agent 执行的事件记录
- **AND** 每个事件 SHALL 包含：tick 编号、Agent 名称、事件类型、描述文本

#### Scenario: 遗产事件填充

- **WHEN** 本 tick 内有 Agent 死亡
- **THEN** 快照的 `legacies` 字段 SHALL 包含死亡 Agent 的遗产记录
- **AND** 遗产记录 SHALL 包含：遗产 ID、原 Agent 名称、遗产内容摘要、位置

#### Scenario: 压力事件填充

- **WHEN** 本 tick 内触发了环境压力事件
- **THEN** 快照的 `pressures` 字段 SHALL 包含当前活跃的压力状态
- **AND** 压力状态 SHALL 包含：压力类型、影响区域、强度、持续时间

#### Scenario: 地图变更填充

- **WHEN** 本 tick 内有建筑放置或资源变化
- **THEN** 快照的 `map_changes` 字段 SHALL 包含变更的格子坐标和变更类型
- **AND** 无变更时该字段 SHALL 为空数组（非 null）

### Requirement: WorldSnapshot 序列化

WorldSnapshot SHALL 提供 `to_json()` 方法将快照序列化为 JSON 字符串，供 Godot GDExtension 桥接层使用。

#### Scenario: 序列化完整快照

- **WHEN** 调用 `WorldSnapshot::to_json()`
- **THEN** 返回的 JSON 字符串 SHALL 包含：tick、agents、map_changes、events、legacies、pressures 所有字段
- **AND** JSON SHALL 可被 `serde_json::from_str` 反向解析

#### Scenario: 空数据序列化

- **WHEN** 快照中某些字段为空（如无事件、无遗产）
- **THEN** 序列化 SHALL 输出空数组 `[]` 而非省略字段
- **AND** JSON 结构 SHALL 保持完整

### Requirement: Godot 端事件分发

SimulationBridge GDExtension SHALL 将 WorldSnapshot 中的事件、遗产、压力数据转换为 Godot 信号发送。

#### Scenario: 叙事事件信号

- **WHEN** 快照中包含叙事事件
- **THEN** SimulationBridge SHALL 为每个事件发射 `narrative_event` 信号
- **AND** 信号参数 SHALL 为 Dictionary：`{tick, agent_name, event_type, description}`

#### Scenario: 遗产信号

- **WHEN** 快照中包含遗产记录
- **THEN** SimulationBridge SHALL 为每个遗产发射 `legacy_created` 信号
- **AND** 信号参数 SHALL 为 Dictionary：`{legacy_id, agent_name, position, description}`

#### Scenario: 压力信号

- **WHEN** 快照中包含压力状态
- **THEN** SimulationBridge SHALL 发射压力更新信号
- **AND** Godot 端的 PressureList 控件 SHALL 更新显示
