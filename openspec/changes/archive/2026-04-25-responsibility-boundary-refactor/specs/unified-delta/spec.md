# 功能规格说明 - 统一 Delta 类型

## ADDED Requirements

### Requirement: 单一 Delta 类型定义

系统 SHALL 统一 Delta 类型定义，删除 snapshot.rs 中的 WorldDelta，只保留 simulation/delta.rs 的 AgentDelta。

AgentDelta SHALL 包含所有增量事件类型：
- AgentMoved, AgentDied, AgentSpawned
- StructureCreated, StructureDestroyed
- ResourceChanged
- TradeCompleted, AllianceFormed, AllianceBroken
- MilestoneReached, PressureStarted, PressureEnded

#### Scenario: 删除 WorldDelta 类型

- **WHEN** 重构后
- **THEN** snapshot.rs 不包含 WorldDelta enum
- **AND** 所有代码使用 simulation::AgentDelta

### Requirement: Delta 字段统一

AgentDelta 各变体的字段命名 SHALL 统一：
- 位置使用 `(u32, u32)` tuple 格式（与 WorldSnapshot 一致）
- 不使用 `x/y` 分离字段

#### Scenario: AgentMoved 字段统一

- **WHEN** 创建 AgentDelta::AgentMoved
- **THEN** 使用 `position: (u32, u32)` 字段
- **AND** 不使用 `x: u32, y: u32` 分离字段

### Requirement: P2P 广播接口预留

AgentDelta SHALL 支持两种模式：
- **LocalMode**: 用于前端渲染，包含所有细节
- **BroadcastMode**: 用于 P2P 网络广播，包含最小必要信息

系统 SHALL 提供 `AgentDelta::for_broadcast()` 方法生成网络传输格式。

#### Scenario: Delta 广播模式

- **WHEN** 调用 delta.for_broadcast()
- **THEN** 返回精简版本（不含 owner_name、color_code 等渲染字段）
- **AND** 可序列化为紧凑 JSON

#### Scenario: Delta 本地模式

- **WHEN** 发送到前端
- **THEN** 包含完整字段（name、position、color_code）
- **AND** 通过 conversion.rs 转换为 GDScript Dictionary

### Requirement: Delta 序列化统一

系统 SHALL 为 AgentDelta 提供统一的序列化：
- JSON 序列化用于 P2P 网络传输
- Binary 序列化用于本地存储

#### Scenario: Delta JSON 序列化

- **WHEN** 需要网络传输
- **THEN** AgentDelta 可序列化为 JSON
- **AND** JSON 格式符合 P2P Codec 规范

### Requirement: Delta 源字段

AgentDelta SHALL 包含 `source: PeerId` 字段（预留）：
- 标识 Delta 来源节点
- 用于 P2P 合并冲突解决

#### Scenario: Delta 包含来源信息

- **WHEN** 创建 AgentDelta
- **THEN** 包含可选的 source: Option<PeerId>
- **AND** 本地模式为 None
- **AND** P2P 模式为本节点 PeerId