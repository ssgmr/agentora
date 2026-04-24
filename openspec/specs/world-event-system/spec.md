# World Event System Spec

## Purpose

定义世界事件系统，用于里程碑、压力、建筑、资源、贸易、联盟等非 Agent 状态变化的事件广播，与 AgentStateChanged 分离，实现清晰的事件分类。

## ADDED Requirements

### Requirement: WorldEvent 分类

系统 SHALL 将世界级事件与 Agent 状态变化分离，定义独立的 WorldEvent 类型。

#### Scenario: WorldEvent 类型定义

- **WHEN** 定义 WorldEvent 枚举
- **THEN** 枚举 SHALL 包含：
  - StructureCreated { pos, structure_type, owner_id }
  - StructureDestroyed { pos, structure_type }
  - ResourceChanged { pos, resource_type, amount }
  - TradeCompleted { from_id, to_id, items }
  - AllianceFormed { id1, id2 }
  - AllianceBroken { id1, id2, reason }
  - MilestoneReached { name, display_name, tick }
  - PressureStarted { pressure_type, description, duration }
  - PressureEnded { pressure_type, description }
  - AgentNarrative { narrative: NarrativeEvent } (叙事也作为世界事件)

#### Scenario: 世界事件广播范围

- **WHEN** 发生世界事件
- **THEN** MilestoneReached、PressureStarted、PressureEnded SHALL 广播到 world_events topic
- **AND** StructureCreated、ResourceChanged、TradeCompleted SHALL 广播到 region topic
- **AND** AgentNarrative SHALL 根据 NarrativeEvent.channel 决定广播范围

### Requirement: 客户端事件处理统一

客户端 SHALL 统一通过 Delta 接收数据，不再区分 Delta 和 Snapshot 事件。

#### Scenario: StateManager 统一处理

- **WHEN** Bridge 发送 agent_delta 信号
- **THEN** StateManager._on_delta(delta) SHALL 处理：
  - AgentStateChanged → 更新 _agents[agent_id] = state
  - WorldEvent(StructureCreated) → 更新 _structures[pos]
  - WorldEvent(MilestoneReached) → 追加 _milestones
  - WorldEvent(AgentNarrative) → 追加 _narratives
- **AND** 不再调用 _on_world_updated 处理 Snapshot 中的 events
