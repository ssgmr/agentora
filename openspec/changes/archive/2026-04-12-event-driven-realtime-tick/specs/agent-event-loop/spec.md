# 功能规格说明

## ADDED Requirements

### Requirement: Agent 独立决策循环

每个 Agent 必须拥有独立的决策循环，不再由 World 统一 tick 遍历。Agent 按照自身的决策间隔独立执行决策、应用动作、推送事件。

#### Scenario: Agent 独立决策
- **WHEN** Agent 的决策计时器到期
- **THEN** Agent 执行决策管道（Spark → LLM/规则引擎 → 动作选择）
- **AND** 动作立即应用到 World 状态
- **AND** 生成 AgentDelta 事件发送至 Godot 通道
- **AND** 重置决策计时器

#### Scenario: 不同 Agent 决策间隔不同
- **WHEN** Agent A 的决策间隔为 2s，Agent B 的决策间隔为 3s
- **THEN** Agent A 每 2s 执行一次决策
- **AND** Agent B 每 3s 执行一次决策
- **AND** 两者互不阻塞

#### Scenario: 死亡 Agent 不再决策
- **WHEN** Agent 的 is_alive 标记为 false
- **THEN** 该 Agent 的决策循环终止
- **AND** 生成 LegacyDelta 事件发送至 Godot 通道

#### Scenario: LLM 调用失败时回退
- **WHEN** Agent 的 LLM 调用超时或返回错误
- **THEN** 决策管道自动回退到规则引擎 fallback
- **AND** 规则引擎生成的动作正常应用并推送 delta
- **AND** Agent 决策循环不中断

### Requirement: 事件流通道

模拟线程必须通过独立的 mpsc 通道向 Godot 推送 AgentDelta 事件，实现决策完成即推送，不等其他 Agent。

#### Scenario: AgentDelta 事件推送
- **WHEN** Agent 完成决策并应用动作
- **THEN** 通过 agent_delta_sender 发送 AgentDelta 事件
- **AND** 事件包含 agent_id、新位置、动作类型、状态变化

#### Scenario: WorldChanged 事件推送
- **WHEN** World 状态发生变化（资源消耗/建筑放置/地形改变）
- **THEN** 通过 agent_delta_sender 发送 WorldChanged 事件
- **AND** 事件包含变化类型、位置、相关数据

#### Scenario: 多事件并发不丢失
- **WHEN** 多个 Agent 几乎同时完成决策
- **THEN** 每个 Agent 的 delta 事件都进入通道队列
- **AND** Godot 端按接收顺序逐一处理

### Requirement: 定期快照兜底

保留定期 WorldSnapshot 推送机制，用于 Godot 端的一致性检查和存档，与增量 delta 并行运行。

#### Scenario: 定期发送完整快照
- **WHEN** snapshot_interval 计时器到期（默认 5s）
- **THEN** 生成完整 WorldSnapshot
- **AND** 通过 snapshot_sender 发送至 Godot
- **AND** 不影响 AgentDelta 的实时推送

#### Scenario: 快照与 delta 并行
- **WHEN** AgentDelta 和 WorldSnapshot 同时存在于通道
- **THEN** Godot 端优先处理 AgentDelta（实时性）
- **AND** WorldSnapshot 用于一致性校验和脏数据清理

### Requirement: NPC 快速决策循环

NPC Agent 使用规则引擎直接决策，跳过 LLM 调用，实现快速低成本的决策循环，用于开发验证阶段。

#### Scenario: NPC 使用规则引擎决策
- **WHEN** NPC Agent 的决策计时器到期
- **THEN** 直接调用规则引擎基于当前动机和 WorldState 生成动作
- **AND** 不调用 LLM
- **AND** 动作应用并推送 delta 事件

#### Scenario: NPC 决策间隔可配置
- **WHEN** 创建 NPC Agent 时指定决策间隔
- **THEN** NPC 按照指定间隔执行决策
- **AND** 默认间隔为 1s（比玩家 Agent 更快）

#### Scenario: NPC 与玩家 Agent 共存
- **WHEN** 世界中同时存在 NPC 和玩家 Agent
- **THEN** NPC 的快速决策不阻塞玩家 Agent
- **AND** 两者的 delta 事件通过同一通道推送
- **AND** Godot 端无法区分 NPC 和玩家 Agent（统一渲染）
