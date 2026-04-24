# 功能规格说明 - 前端状态管理器

## Purpose

在 Godot 客户端创建 StateManager Autoload，作为唯一的状态分发中心，消除各组件对 SimulationBridge 的直接监听，统一状态管理。

## Requirements

### Requirement: 单一状态管理器

系统 SHALL 在 Godot 客户端创建 StateManager Autoload，作为唯一的状态分发中心。

StateManager SHALL：
- 接收 SimulationBridge 的 world_updated 信号
- 解析 snapshot 并分发到各组件
- 接收 agent_delta 信号并增量更新
- 提供 get_agent_data()、get_terrain_at() 等查询接口

#### Scenario: StateManager 接收 snapshot

- **WHEN** SimulationBridge 发射 world_updated
- **THEN** StateManager.on_world_updated(snapshot) 被调用
- **AND** StateManager 更新内部状态字典
- **AND** 发射 state_updated 信号通知各组件

#### Scenario: 各组件订阅 StateManager

- **WHEN** WorldRenderer._ready()
- **THEN** 订阅 StateManager.state_updated
- **AND** 不直接订阅 SimulationBridge.world_updated

### Requirement: 消除冗余监听

系统 SHALL 删除各组件对 SimulationBridge 的直接监听：
- main.gd SHALL 只订阅 StateManager
- world_renderer.gd SHALL 只订阅 StateManager
- agent_manager.gd SHALL 只订阅 StateManager

#### Scenario: world_renderer 不直接监听 Bridge

- **WHEN** 重构后
- **THEN** world_renderer.gd 不包含 `bridge.world_updated.connect()`
- **AND** 使用 `StateManager.on_terrain_changed` 回调

### Requirement: 统一配置获取

StateManager SHALL 提供统一的配置查询：
- map_size: 从 snapshot.terrain_width 获取
- tile_size: 系统常量
- camera_bounds: 从 StateManager 计算

#### Scenario: 相机边界从 StateManager 获取

- **WHEN** CameraController 初始化
- **THEN** 调用 StateManager.get_map_bounds()
- **AND** 设置相机限制

### Requirement: 增量更新支持

StateManager SHALL 支持 delta 增量更新：
- 接收 agent_delta 信号
- 只更新变化的 Agent/资源/建筑
- 发射具体的 change 信号（如 agent_position_changed）

#### Scenario: Agent 位置增量更新

- **WHEN** 接收到 AgentMoved delta
- **THEN** 只更新该 Agent 的 position
- **AND** 发射 agent_position_changed(agent_id, new_pos)
- **AND** 不触发全量 state_updated