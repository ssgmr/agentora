# 功能规格说明

## ADDED Requirements

### Requirement: adjust_motivation 实际执行

`SimulationBridge::adjust_motivation(agent_id, dimension, value)` SHALL 实际修改 World 中对应 Agent 的动机向量，而非仅打印日志。

#### Scenario: 调整单个动机维度

- **WHEN** Godot 调用 `adjust_motivation("agent_0", 2, 0.8)`
- **THEN** World 中 `agent_0` 的动机向量第 3 个维度（认知）SHALL 设为 0.8
- **AND** 下一 tick 的决策 SHALL 使用更新后的动机值

#### Scenario: 调整不存在的 Agent

- **WHEN** 调用的 agent_id 在 World 中不存在
- **THEN** 方法 SHALL 静默忽略，不报错
- **AND** 系统 SHALL 在调试日志中记录警告

### Requirement: inject_preference 实际执行

`SimulationBridge::inject_preference(agent_id, dimension, boost, duration)` SHALL 向 Agent 注入临时偏好，在指定 tick 数内提升动机权重。

#### Scenario: 注入临时偏好

- **WHEN** Godot 调用 `inject_preference("agent_0", 2, 0.3, 10)`
- **THEN** Agent 的认知维度 SHALL 在接下来 10 个 tick 内额外 +0.3
- **AND** 临时偏好 SHALL 与基础动机值叠加（非替换）

#### Scenario: 临时偏好到期衰减

- **WHEN** 注入的临时偏好 tick 数到期
- **THEN** 该维度的额外加成 SHALL 移除
- **AND** 动机值 SHALL 恢复为基础值 + 惯性衰减

#### Scenario: 多个临时偏好叠加

- **WHEN** 同一 Agent 同时存在多个临时偏好
- **THEN** 各偏好的加成 SHALL 累加
- **AND** 各偏好的衰减计时 SHALL 独立

### Requirement: set_tick_interval 实际执行

`SimulationBridge::set_tick_interval(seconds)` SHALL 实际修改模拟线程的 tick 间隔。

#### Scenario: 设置 tick 间隔

- **WHEN** Godot 调用 `set_tick_interval(0.4)`
- **THEN** 模拟线程 SHALL 将 tick 间等待时间设为 0.4 秒
- **AND** 后续 tick 的执行频率 SHALL 相应加快

### Requirement: toggle_pause 实际执行

`SimulationBridge::toggle_pause()` SHALL 通过 SimCommand 通道通知模拟线程切换暂停状态。

#### Scenario: 暂停模拟

- **WHEN** Godot 调用 `toggle_pause()` 且当前为运行状态
- **THEN** SimulationBridge SHALL 发送 `SimCommand::Pause`
- **AND** 模拟线程 SHALL 停止推进 tick
- **AND** 模拟线程 SHALL 继续 poll 命令通道以响应恢复指令

#### Scenario: 恢复模拟

- **WHEN** Godot 调用 `toggle_pause()` 且当前为暂停状态
- **THEN** SimulationBridge SHALL 发送 `SimCommand::Start`
- **AND** 模拟线程 SHALL 恢复 tick 循环

### Requirement: get_agent_count 返回真实值

`SimulationBridge::get_agent_count()` SHALL 返回 World 中实际存活的 Agent 数量，而非硬编码值。

#### Scenario: 获取 Agent 数量

- **WHEN** Godot 调用 `get_agent_count()`
- **THEN** 返回值 SHALL 等于 World 中 `agents` 集合的大小
- **AND** 已死亡的 Agent SHALL 不计入总数

### Requirement: get_agent_data 返回真实数据

`SimulationBridge::get_agent_data(agent_id)` SHALL 从 World 中获取 Agent 的实时数据。

#### Scenario: 获取 Agent 数据

- **WHEN** Godot 调用 `get_agent_data("agent_0")`
- **THEN** 返回 Dictionary SHALL 包含：id、name、position、motivation、health、max_health、age、is_alive、current_action、inventory
- **AND** position SHALL 为 Vector2 格式（x, y）
- **AND** motivation SHALL 为 6 元素浮点数组

#### Scenario: Agent 不存在

- **WHEN** 调用的 agent_id 不存在
- **THEN** 方法 SHALL 返回空 Dictionary `{}`
