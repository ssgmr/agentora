# World Event System Spec

## Purpose

定义世界事件系统，用于里程碑、压力、建筑、资源等非 Agent 状态变化的事件广播，与 AgentStateChanged 分离，实现清晰的事件分类。

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

### Requirement: Delta 统一结构

Delta SHALL 简化为 AgentStateChanged + WorldEvent 两类，替代当前14种变体。

#### Scenario: Delta 结构定义

- **WHEN** 定义 Delta 枚举
- **THEN** 枚举 SHALL 仅包含：
  - AgentStateChanged { agent_id, state: AgentState, change_hint: ChangeHint }
  - WorldEvent(WorldEvent)
- **AND** 不再存在 AgentMoved、AgentDied、AgentSpawned 等独立变体

#### Scenario: AgentStateChanged 替代 AgentMoved

- **WHEN** Agent 状态发生变化
- **THEN** 发送 `AgentStateChanged { state: AgentState, change_hint: Moved }`
- **AND** 不再发送 AgentDelta::AgentMoved

#### Scenario: AgentStateChanged 替代 AgentDied

- **WHEN** Agent 死亡
- **THEN** 发送 `AgentStateChanged { state: AgentState { is_alive: false }, change_hint: Died }`
- **AND** 不再发送 AgentDelta::AgentDied

#### Scenario: AgentStateChanged 替代 AgentSpawned

- **WHEN** 新 Agent 首次出现
- **THEN** 发送 `AgentStateChanged { state: AgentState, change_hint: Spawned }`
- **AND** 不再发送 AgentDelta::AgentSpawned

### Requirement: ChangeHint 变化标记

AgentStateChanged SHALL 包含 change_hint 字段，标记状态变化原因。

#### Scenario: ChangeHint 枚举定义

- **WHEN** 定义 ChangeHint 枚举
- **THEN** 枚举 SHALL 包含：
  - Spawned (新 Agent 首次出现)
  - Moved (位置变化)
  - ActionExecuted (动作执行后)
  - Died (死亡)
  - SurvivalLow (生存状态警告)
  - Healed (营地治愈)

#### Scenario: change_hint 用于客户端 UI

- **WHEN** 客户端收到 AgentStateChanged
- **THEN** change_hint SHALL 用于触发 UI 特效：
  - Spawned → 诞生动画
  - Died → 死亡动画/灰化
  - SurvivalLow → 状态警告提示
- **AND** 客户端 SHALL 不需要从 state 字段推断变化类型

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