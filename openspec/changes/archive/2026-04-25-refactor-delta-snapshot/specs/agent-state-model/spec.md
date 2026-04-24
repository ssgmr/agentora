# Agent State Model Spec

## Purpose

定义统一的 Agent 状态数据模型，替代当前分散的 AgentSnapshot 和 AgentDelta::AgentMoved，实现数据构建逻辑集中化。

## ADDED Requirements

### Requirement: AgentState 统一结构

系统 SHALL 使用单一的 `AgentState` 结构表示 Agent 状态，该结构 SHALL 同时用于 Snapshot 序列化、Delta 传输和 ShadowAgent 存储。

#### Scenario: AgentState 结构定义

- **WHEN** 定义 AgentState 结构
- **THEN** 结构 SHALL 包含以下字段：
  - id: String
  - name: String
  - position: (u32, u32)
  - health: u32, max_health: u32
  - satiety: u32, hydration: u32
  - age: u32, level: u32
  - is_alive: bool
  - inventory_summary: HashMap<String, u32>
  - current_action: String
  - action_result: String
  - reasoning: Option<String> (本地有，远程可选)
- **AND** 结构 SHALL 实现 Serialize/Deserialize trait

#### Scenario: 本地 Agent 转换

- **WHEN** 本地 Agent 执行完动作
- **THEN** 系统 SHALL 通过 `Agent::to_state()` 方法生成 AgentState
- **AND** reasoning 字段 SHALL 包含 LLM 决策思考内容

#### Scenario: 远程 Agent 接收

- **WHEN** 收到远程 Delta
- **THEN** 系统 SHALL 通过 `AgentState::from_delta()` 方法解析
- **AND** reasoning 字段 SHALL 为 None 或空字符串

### Requirement: AgentState 转换方法

AgentState SHALL 提供统一的转换方法，替代当前分散的数据构建逻辑。

#### Scenario: to_delta 方法

- **WHEN** 调用 `agent_state.to_delta()`
- **THEN** SHALL 返回 `AgentStateChanged { state: AgentState, change_hint: ChangeHint }`
- **AND** change_hint SHALL 根据上下文推断（Spawned/Moved/Died等）

#### Scenario: to_snapshot 方法

- **WHEN** 调用 `agent_state.to_snapshot()`
- **THEN** SHALL 返回 AgentSnapshot（兼容现有客户端）
- **AND** 字段 SHALL 一一对应，无信息丢失

#### Scenario: to_godot_dict 方法

- **WHEN** 调用 `agent_state.to_godot_dict()`
- **THEN** SHALL 返回 Godot Dictionary（用于 Bridge 信号）
- **AND** position SHALL 转为 Vector2 格式

### Requirement: Agent 状态构建集中化

所有 Agent 状态构建 SHALL 统一调用 Agent/ShadowAgent 的 `to_state()` 方法。

#### Scenario: DeltaEmitter 使用统一方法

- **WHEN** DeltaEmitter 发送 Agent 状态变化
- **THEN** SHALL 调用 `agent.to_state(current_action, action_result, reasoning).to_delta()`
- **AND** 不再手动构建 AgentDelta::AgentMoved 的13个字段

#### Scenario: Snapshot 使用统一方法

- **WHEN** World.snapshot() 生成快照
- **THEN** SHALL 调用 `agent.to_state(...)` 和 `shadow.to_state()`
- **AND** 不再手动构建 AgentSnapshot 结构

#### Scenario: ShadowAgent 使用统一结构

- **WHEN** ShadowAgent 存储远程 Agent 状态
- **THEN** ShadowAgent 结构 SHALL 直接使用 AgentState
- **AND** 不再维护独立的字段集合