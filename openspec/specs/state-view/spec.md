# 功能规格说明 - 状态视图统一

## Purpose

定义 WorldState 自动构建方式，从 World 自动构建决策所需的状态快照，消除数据重复。

## ADDED Requirements

### Requirement: WorldState 自动构建

系统 SHALL 提供统一的 WorldState 构建方式，从 World 自动构建决策所需的状态快照。

WorldState 构建器 SHALL：
- 从 World 获取 Agent 基本信息（位置、库存、饱食度、水分度）
- 调用 scan_vision() 获取视野内信息
- 提取压力事件列表
- 转换为 RuleEngine 和 DecisionPipeline 可用的格式

#### Scenario: WorldState 从 World 构建

- **WHEN** 调用 WorldStateBuilder.build(world, agent_id, vision_radius)
- **THEN** 返回完整的 WorldState
- **AND** WorldState 包含所有决策所需字段

#### Scenario: WorldState 字段完整

- **WHEN** WorldState 构建完成
- **THEN** SHALL 包含：
  - agent_position, agent_inventory, agent_satiety, agent_hydration
  - terrain_at, resources_at
  - nearby_agents, nearby_structures, nearby_legacies
  - active_pressures, temp_preferences
  - agent_personality

### Requirement: 消除数据重复

系统 SHALL 删除 WorldState 的手动构建逻辑（当前在 agent_loop.rs 80+ 行）。

agent_loop.rs SHALL 使用：
```rust
let world_state = WorldStateBuilder::build(&world, &agent_id, vision_radius);
```

#### Scenario: agent_loop 使用 WorldStateBuilder

- **WHEN** agent_loop.rs 开始决策
- **THEN** 调用 WorldStateBuilder::build()
- **AND** 不手动组装 HashMap 和 Vec

### Requirement: WorldState 与 World 同步保证

系统 SHALL 保证 WorldState 数据与 World 一致：
- WorldState 构建 SHALL 在持有 World 锁时进行
- 构建后 SHALL 不依赖 World 后续修改

#### Scenario: WorldState 构建时机正确

- **WHEN** WorldStateBuilder.build() 被调用
- **THEN** 在同一锁周期内获取所有数据
- **AND** 数据一致性得到保证