# 功能规格说明 - World 子系统拆分

## Purpose

将 World 结构体的职责拆分为多个专职子系统，每个子系统只负责单一领域，World 作为协调者。

## Requirements

### Requirement: World 模块职责边界明确

系统 SHALL 将 World 结构体的职责拆分为多个专职子系统，每个子系统 SHALL 只负责单一领域。

子系统划分：
- **WorldMap**: 地形查询、边界检查、地形类型管理
- **WorldAgents**: Agent 存储、位置索引、Agent 查询
- **WorldResources**: 资源节点管理、采集逻辑、资源刷新
- **WorldStructures**: 建筑管理、耐久度、效果范围
- **ActionExecutor**: 动作路由、执行、反馈生成

#### Scenario: WorldMap 独立查询地形

- **WHEN** 调用 WorldMap.terrain_at(pos)
- **THEN** 返回 TerrainType 且不依赖其他子系统

#### Scenario: WorldAgents 维护位置索引

- **WHEN** Agent 移动
- **THEN** WorldAgents 自动更新 agent_positions 反向索引

#### Scenario: ActionExecutor 调用子系统执行动作

- **WHEN** 执行 Gather 动作
- **THEN** ActionExecutor 调用 WorldResources.gather() 和 WorldAgents.update_inventory()

### Requirement: World 结构体作为协调者

系统 SHALL 将 World 改为协调者角色，只负责：
- 子系统初始化和持有
- advance_tick() 调用各子系统 tick 方法
- snapshot() 组合各子系统数据

World 结构体 SHALL 不直接执行业务逻辑。

#### Scenario: World.advance_tick 协调调用

- **WHEN** World.advance_tick() 被调用
- **THEN** 调用 survival_consumption_tick()、structure_effects_tick()、pressure_tick() 等子函数
- **AND** 不直接修改 Agent/Resource 状态

#### Scenario: World 模块行数限制

- **WHEN** 完成拆分后
- **THEN** world/mod.rs 行数 SHALL < 300

### Requirement: ActionExecutor 动作路由清晰

系统 SHALL 创建 ActionExecutor 模块负责动作执行路由：
- 接收 ActionType 和 AgentId
- 调用对应 handler
- 生成 ActionResult
- 更新子系统状态

#### Scenario: ActionExecutor 路由 Gather 动作

- **WHEN** ActionExecutor.execute(agent_id, Gather{resource})
- **THEN** 检查当前位置资源节点
- **AND** 更新资源存量（WorldResources）
- **AND** 更新 Agent 库存（WorldAgents）
- **AND** 返回 ActionResult::SuccessWithDetail

#### Scenario: ActionExecutor 处理失败动作

- **WHEN** 动作执行失败
- **THEN** 返回 ActionResult::Blocked(reason)
- **AND** 不修改任何子系统状态
