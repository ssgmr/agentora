# Snapshot System Spec

## Purpose

Snapshot 退化为初始化（WorldInit）和兜底（StateSnapshot），不再每次发送完整状态，提高效率并简化客户端处理。

## ADDED Requirements

### Requirement: Snapshot 发送时机

Snapshot SHALL 仅在以下时机发送：
- 启动时发送 WorldInit（地形 + 初始 Agent）
- 每5秒发送 StateSnapshot 作为兜底

#### Scenario: 启动时发送 WorldInit

- **WHEN** Simulation 启动
- **THEN** 系统 SHALL 发送 WorldInit：
  - terrain_grid: Vec<u8> (256x256)
  - terrain_width, terrain_height
  - initial_agents: Vec<AgentState>
- **AND** 不再包含 events、legacies、pressures、milestones

#### Scenario: 5秒兜底发送 StateSnapshot

- **WHEN** snapshot_loop 每5秒触发
- **THEN** 系统 SHALL 发送 StateSnapshot：
  - tick: u64
  - agents: Vec<AgentState> (仅本地 Agent + 视野内远程 Agent)
  - structures: HashMap<(u32,u32), StructureInfo>
  - resources: HashMap<(u32,u32), ResourceInfo>
  - pressures: Vec<PressureInfo>
- **AND** 不再发送 terrain_grid（已在 WorldInit 发送）
- **AND** 不再发送 events、milestones（通过 Delta 接收）

### Requirement: Snapshot 结构简化

WorldSnapshot 结构 SHALL 简化，移除冗余字段。

#### Scenario: WorldSnapshot 字段调整

- **WHEN** 定义 WorldSnapshot 结构
- **THEN** 结构 SHALL 包含：
  - tick: u64
  - agents: Vec<AgentState> (替代 Vec<AgentSnapshot>)
  - terrain_grid: Option<Vec<u8>> (仅 WorldInit 包含)
  - structures: HashMap<String, StructureInfo> (替代 map_changes)
  - resources: HashMap<String, ResourceInfo>
  - pressures: Vec<PressureInfo>
- **AND** 结构 SHALL 不包含：
  - events (通过 narrative 信号接收)
  - legacies (通过 WorldEvent 接收)
  - milestones (通过 WorldEvent 接收)

### Requirement: conversion.rs 完整转换

conversion.rs SHALL 完整转换所有 Snapshot 字段，不再遗漏。

#### Scenario: snapshot_to_dict 完整转换

- **WHEN** 调用 snapshot_to_dict(snapshot)
- **THEN** 转换结果 SHALL 包含：
  - tick, agents, terrain_grid (如有)
  - structures (新增)
  - resources (新增)
  - pressures (新增)
- **AND** 不再遗漏任何字段

## REMOVED Requirements

### Requirement: events 字段

**原因**：events 通过 narrative 信号接收，不放在 Snapshot
**迁移方案**：客户端通过 bridge.narrative_event 信号接收叙事

### Requirement: legacies 字段

**原因**：legacies 通过 WorldEvent 接收，不放在 Snapshot
**迁移方案**：客户端通过 agent_delta 中 WorldEvent(LegacyCreated) 接收

### Requirement: milestones 字段

**原因**：milestones 通过 WorldEvent 接收，不放在 Snapshot
**迁移方案**：客户端通过 agent_delta 中 WorldEvent(MilestoneReached) 接收

### Requirement: map_changes 字段

**原因**：改用 structures/resources HashMap，更高效
**迁移方案**：客户端直接使用 structures/resources 字典，无需从数组构建
