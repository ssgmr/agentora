# P2P Gossip Spec (Modified)

## Purpose

P2P Gossip 区域订阅实际生效，支持叙事按区域广播，实现视野范围内的远程 Agent 状态同步。

## MODIFIED Requirements

### Requirement: 区域 Topic 订阅生效

RegionTopicManager SHALL 实际订阅区域 topic，而非当前空实现。

#### Scenario: 订阅区域 Topic

- **WHEN** Agent 进入新区域
- **THEN** 系统 SHALL 订阅 `region_<id>` topic
- **AND** MessageHandler SHALL 实际处理收到的消息
- **AND** 不再使用 NullMessageHandler 空实现

#### Scenario: 邻区订阅

- **WHEN** Agent 进入区域 R
- **THEN** 系统 SHALL 同时订阅 R 及其邻区（上下左右）
- **AND** 系统 SHALL 退订超出视野范围的区域

### Requirement: Delta 按区域广播

AgentStateChanged SHALL 通过 Agent 当前区域的 topic 广播。

#### Scenario: AgentStateChanged 广播

- **WHEN** 本地 Agent 状态变化
- **THEN** 系统 SHALL 通过 `region_<当前位置区域>` topic 广播
- **AND** 广播内容 SHALL 为 DeltaEnvelope { delta, source_peer_id, tick }

### Requirement: 叙事按区域广播

NarrativeEvent SHALL 根据 channel 决定广播范围。

#### Scenario: Nearby 叙事广播

- **WHEN** 产生 Nearby 频道叙事
- **THEN** 系统 SHALL 通过 `region_<Agent当前区域>` topic 广播
- **AND** NarrativeEnvelope SHALL 包含 source_peer_id

#### Scenario: World 叙事广播

- **WHEN** 产生 World 频道叙事
- **THEN** 系统 SHALL 通过 `world_events` topic 广播
- **AND** 所有订阅该 topic 的客户端 SHALL 接收

## ADDED Requirements

### Requirement: world_events Topic

系统 SHALL 新增 `world_events` 全局 topic，用于世界级事件广播。

#### Scenario: 订阅 world_events

- **WHEN** Simulation 启动（P2P 模式）
- **THEN** 系统 SHALL 自动订阅 `world_events` topic
- **AND** 系统 SHALL 接收 Milestone、Pressure 等全局事件

#### Scenario: 广播 Milestone

- **WHEN** 达成里程碑
- **THEN** 系统 SHALL 通过 `world_events` topic 广播
- **AND** 广播内容 SHALL 为 WorldEvent(MilestoneReached)

### Requirement: NarrativeEnvelope 结构

叙事广播 SHALL 使用 NarrativeEnvelope 包装。

#### Scenario: NarrativeEnvelope 定义

- **WHEN** 定义 NarrativeEnvelope 结构
- **THEN** 结构 SHALL 包含：
  - narrative: NarrativeEvent
  - source_peer_id: String
  - region_id: Option<u32> (Nearby 频道有，World 频道无)
- **AND** 结构 SHALL 实现 Serialize/Deserialize