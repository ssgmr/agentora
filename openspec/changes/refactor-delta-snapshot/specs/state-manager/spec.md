# State Manager Spec (Modified)

## Purpose

客户端 StateManager 统一通过 Delta 接收数据，简化处理逻辑，不再区分 Delta 和 Snapshot 事件处理。

## MODIFIED Requirements

### Requirement: StateManager 数据接收

StateManager SHALL 统一通过 agent_delta 信号接收数据，不再通过 world_updated 处理 events。

#### Scenario: _on_world_updated 简化

- **WHEN** Bridge 发送 world_updated 信号
- **THEN** StateManager._on_world_updated(snapshot) SHALL 仅处理：
  - terrain_grid (初始化时)
  - agents (兜底更新)
  - structures/resources/pressures
- **AND** SHALL 不处理 snapshot 中的 events/milestones

#### Scenario: _on_agent_delta 统一处理

- **WHEN** Bridge 发送 agent_delta 信号
- **THEN** StateManager._on_delta(delta) SHALL 处理所有事件类型：
  - AgentStateChanged → 更新 _agents[agent_id]
  - WorldEvent(StructureCreated) → 更新 _structures[pos]
  - WorldEvent(ResourceChanged) → 更新 _resources[pos]
  - WorldEvent(MilestoneReached) → 追加 _milestones
  - WorldEvent(AgentNarrative) → 追加 _narratives
- **AND** SHALL 不再调用独立的 _on_narrative_event

### Requirement: StateManager 存储结构调整

StateManager 内部存储 SHALL 使用 HashMap 提高查询效率。

#### Scenario: 存储结构调整

- **WHEN** 定义 StateManager 存储结构
- **THEN** 存储 SHALL 包含：
  - _agents: Dictionary (agent_id → AgentState)
  - _terrain_grid: PackedByteArray (仅初始化时设置)
  - _terrain_width/height: int
  - _structures: Dictionary (pos_key → StructureInfo)
  - _resources: Dictionary (pos_key → ResourceInfo)
  - _pressures: Array
  - _milestones: Array
  - _narratives: Array
- **AND** _narratives SHALL 包含 channel 和 agent_source 字段

### Requirement: 叙事过滤接口

StateManager SHALL 提供叙事过滤接口，支持 NarrativeFeed 按频道和 Agent 筛选。

#### Scenario: get_filtered_narratives 接口

- **WHEN** NarrativeFeed 调用 StateManager.get_filtered_narratives(channel, agent_id)
- **THEN** 返回结果 SHALL 按以下逻辑过滤：
  - 若 channel 不为 null → filter by channel
  - 若 agent_id 不为 null → filter by agent_id
  - 若两者都为 null → 返回全部
- **AND** 过滤 SHALL 组合生效（channel AND agent_id）

## ADDED Requirements

### Requirement: 叙事过滤状态存储

StateManager SHALL 维护叙事过滤状态。

#### Scenario: 过滤状态存储

- **WHEN** 用户切换叙事频道或选择 Agent
- **THEN** StateManager SHALL 存储：
  - _narrative_channel: NarrativeChannel
  - _narrative_agent_filter: String or null
- **AND** 状态 SHALL 通过信号通知 NarrativeFeed 更新