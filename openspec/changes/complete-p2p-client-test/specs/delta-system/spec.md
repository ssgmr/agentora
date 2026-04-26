# Delta System - P2P 广播集成

## Purpose

在 DeltaEmitter 发送 delta 后，触发 P2P 广播，使远端节点能实时感知本地 Agent 状态变化。

## ADDED Requirements

### Requirement: Delta P2P 广播触发

Agent 决策循环中，DeltaEmitter::emit_all() 后 SHALL 触发 P2P 广播。

#### Scenario: AgentStateChanged P2P 广播

- **WHEN** Agent 执行动作后 DeltaEmitter::emit_all() 发送了 AgentStateChanged delta
- **AND** 模拟运行在 P2P 模式下
- **THEN** 系统 SHALL 调用 Simulation::publish_delta_p2p() 将 delta 广播到对应区域 topic
- **AND** 广播内容 SHALL 包含 delta JSON、source_peer_id、当前 tick

#### Scenario: WorldEvent P2P 广播

- **WHEN** DeltaEmitter::emit_all() 发送了 WorldEvent delta（如 StructureCreated）
- **AND** 模拟运行在 P2P 模式下
- **THEN** 系统 SHALL 同样调用 publish_delta_p2p() 广播该 WorldEvent
- **AND** 广播到 `world_events` topic 或区域 topic（根据事件类型决定）

#### Scenario: 中心化模式不广播

- **WHEN** 模拟运行在 Centralized 模式下
- **AND** DeltaEmitter::emit_all() 发送 delta
- **THEN** 系统 SHALL 不执行任何 P2P 广播操作

### Requirement: 区域 ID 计算

P2P 广播时 SHALL 根据 Agent 当前位置计算所属区域 ID。

#### Scenario: Agent 在区域内广播

- **WHEN** Agent 位于 (x, y) 位置
- **AND** 区域大小为 32x32
- **THEN** 区域 ID SHALL = (y / 32) * (map_width / 32) + (x / 32)
- **AND** delta SHALL 广播到 `region_<id>` topic

#### Scenario: Agent 跨区域移动

- **WHEN** Agent 移动导致区域 ID 变化
- **THEN** 后续 delta SHALL 广播到新的区域 topic
- **AND** 退订旧区域 topic（如果不再在视野范围内）
