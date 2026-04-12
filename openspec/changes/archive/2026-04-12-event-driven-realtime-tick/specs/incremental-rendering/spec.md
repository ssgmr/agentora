# 功能规格说明

## ADDED Requirements

### Requirement: AgentDelta 增量事件处理

Godot 客户端必须支持接收和处理 AgentDelta 增量事件，在收到事件后立即更新对应 Agent 的渲染状态，不等待完整快照。

#### Scenario: 收到移动 delta 事件
- **WHEN** 收到 AgentDelta 事件，类型为 Move
- **THEN** 更新对应 agent_id 的 sprite 位置
- **AND** 如果 sprite 不存在则创建新 sprite
- **AND** 播放移动动画（如有）

#### Scenario: 收到状态变化 delta 事件
- **WHEN** 收到 AgentDelta 事件，类型为状态变化（health/motivation/inventory）
- **THEN** 更新对应 agent_id 的内部状态数据
- **AND** 更新 UI 显示（如雷达图/信息面板）

#### Scenario: 收到死亡 delta 事件
- **WHEN** 收到 AgentDelta 事件，类型为 Death
- **THEN** 移除对应 agent_id 的 sprite
- **AND** 在死亡位置创建 Legacy 实体
- **AND** 在叙事流面板添加死亡事件

#### Scenario: 未知 Agent 的 delta 事件
- **WHEN** 收到 agent_id 不存在的 AgentDelta 事件
- **THEN** 创建新的 Agent sprite 并渲染
- **AND** 缓存该 Agent 的信息

### Requirement: WorldChanged 事件处理

Godot 客户端必须支持接收 WorldChanged 事件，用于渲染世界状态变化（资源刷新/建筑放置/地形改变）。

#### Scenario: 收到资源变化事件
- **WHEN** 收到 WorldChanged 事件，类型为 ResourceSpawn 或 ResourceDeplete
- **THEN** 在对应位置添加/移除资源 sprite

#### Scenario: 收到建筑事件
- **WHEN** 收到 WorldChanged 事件，类型为 StructureBuilt 或 StructureDestroyed
- **THEN** 在对应位置添加/移除建筑 sprite
- **AND** 在叙事流面板添加对应事件

### Requirement: Snapshot 一致性校验

当收到完整的 WorldSnapshot 时，Godot 客户端必须将其与当前渲染状态进行对比，修复不一致。

#### Scenario: 快照与本地状态一致
- **WHEN** 收到 WorldSnapshot
- **AND** 快照中的 Agent 列表与本地渲染一致
- **THEN** 仅更新状态数据，不重建 sprite

#### Scenario: 快照发现缺失 Agent
- **WHEN** 收到 WorldSnapshot
- **AND** 快照中存在本地未渲染的 Agent
- **THEN** 创建缺失的 Agent sprite

#### Scenario: 快照发现幽灵 Agent
- **WHEN** 收到 WorldSnapshot
- **AND** 本地渲染了快照中不存在的 Agent
- **THEN** 移除多余的 Agent sprite

### Requirement: 事件优先级调度

Godot 端必须对不同类型事件设置处理优先级，确保实时性。

#### Scenario: 事件处理优先级
- **WHEN** 同一帧存在多种事件待处理
- **THEN** AgentDelta 事件优先处理（实时渲染）
- **AND** WorldChanged 事件次优先处理
- **AND** WorldSnapshot 最后处理（一致性校验）

## MODIFIED Requirements

### Requirement: Agent 渲染模式

**FROM**：收到 WorldSnapshot 后清空所有 Agent 并重建
**TO**：收到 AgentDelta 后增量更新单个 Agent，收到 WorldSnapshot 后仅做一致性校验和修复

#### Scenario: 增量更新替代全量重建
- **WHEN** Agent 位置从 (x1, y1) 变为 (x2, y2)
- **THEN** 仅更新该 Agent sprite 的 position 属性
- **AND** 不触碰其他 Agent 的 sprite

#### Scenario: 首次初始化仍使用全量渲染
- **WHEN** 首次收到 WorldSnapshot（模拟刚启动）
- **THEN** 根据快照创建所有 Agent sprite
- **AND** 后续切换到增量更新模式
