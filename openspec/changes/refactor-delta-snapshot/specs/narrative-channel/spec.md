# Narrative Channel System Spec

## Purpose

定义叙事频道系统，支持本地/附近/世界三个频道，叙事通过 P2P 按区域广播，用户可以看到视野范围内其他 Agent 的行为叙事。

## ADDED Requirements

### Requirement: NarrativeChannel 频道分类

系统 SHALL 将叙事事件按频道分类，支持本地、附近、世界三种频道类型。

#### Scenario: 频道类型定义

- **WHEN** 定义 NarrativeChannel 枚举
- **THEN** 枚举 SHALL 包含：
  - Local: 本地频道（不广播，仅本地显示）
  - Nearby: 附近频道（按区域 P2P 广播）
  - World: 世界频道（全局 P2P 广播）

#### Scenario: 本地频道叙事

- **WHEN** 本地 Agent 执行 Wait 动作
- **THEN** 叙事 SHALL 分配到 Local 频道
- **AND** 叙事 SHALL 不通过 P2P 广播
- **AND** 叙事 SHALL 仅在本地 narrative_tx 发送

#### Scenario: 附近频道叙事

- **WHEN** Agent 执行 Gather、Move、Talk、Trade、Attack 等动作
- **THEN** 叙事 SHALL 分配到 Nearby 频道
- **AND** 叙事 SHALL 通过 `region_<id>` topic P2P 广播
- **AND** 同区域的远程客户端 SHALL 能接收

#### Scenario: 世界频道叙事

- **WHEN** 发生 Milestone、PressureStart、PressureEnd、Death 等全局事件
- **THEN** 叙事 SHALL 分配到 World 频道
- **AND** 叙事 SHALL 通过 `world_events` topic P2P 广播
- **AND** 所有连接的客户端 SHALL 能接收

### Requirement: 叙事事件结构扩展

NarrativeEvent 结构 SHALL 扩展以支持频道和来源标记。

#### Scenario: NarrativeEvent 字段扩展

- **WHEN** 定义 NarrativeEvent 结构
- **THEN** 结构 SHALL 包含新增字段：
  - channel: NarrativeChannel
  - agent_source: AgentSource (Local/Remote{peer_id})
- **AND** 原有字段（tick, agent_id, agent_name, event_type, description, color_code）SHALL 保持不变

#### Scenario: 本地叙事标记

- **WHEN** 本地 Agent 产生叙事
- **THEN** agent_source SHALL 为 Local
- **AND** channel SHALL 根据事件类型自动分配

#### Scenario: 远程叙事标记

- **WHEN** 收到 P2P 广播的叙事
- **THEN** agent_source SHALL 为 Remote { peer_id: source_peer_id }
- **AND** 客户端 SHALL 能区分叙事来源

### Requirement: P2P 叙事广播

叙事 SHALL 通过 P2P GossipSub 按区域和全局 topic 广播。

#### Scenario: 区域叙事广播

- **WHEN** Agent 产生 Nearby 频道叙事
- **THEN** 系统 SHALL 将叙事编码为 NarrativeEnvelope
- **AND** 系统 SHALL 通过 `region_<当前位置区域>` topic 广播
- **AND** 订阅该 topic 的远程客户端 SHALL 接收

#### Scenario: 全局叙事广播

- **WHEN** 发生 World 频道叙事
- **THEN** 系统 SHALL 通过 `world_events` topic 广播
- **AND** 所有订阅该 topic 的客户端 SHALL 接收

#### Scenario: 本地叙事不广播

- **WHEN** Agent 产生 Local 频道叙事
- **THEN** 系统 SHALL 不通过 P2P 广播
- **AND** 叙事 SHALL 仅发送到本地 narrative_tx

### Requirement: 客户端频道切换

叙事面板 SHALL 支持用户切换频道，显示不同范围的叙事。

#### Scenario: Tab 切换本地频道

- **WHEN** 用户点击"本地"Tab
- **THEN** 叙事面板 SHALL 只显示 channel=Local 的叙事
- **AND** 叙事列表 SHALL 实时更新

#### Scenario: Tab 切换附近频道

- **WHEN** 用户点击"附近"Tab
- **THEN** 叙事面板 SHALL 显示 channel=Nearby 的叙事
- **AND** 叙事 SHALL 包含本地和远程 Agent 的行为

#### Scenario: Tab 切换世界频道

- **WHEN** 用户点击"世界"Tab
- **THEN** 叙事面板 SHALL 显示 channel=World 的叙事
- **AND** 叙事 SHALL 包含里程碑、压力事件等全局叙事