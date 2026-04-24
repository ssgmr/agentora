# 功能规格说明 - AgentLoop 拆分

## Purpose

定义 AgentLoop 拆分为多阶段流水线，每个阶段职责单一，主函数只协调各阶段调用。

## Requirements

### Requirement: AgentLoop 拆分为多阶段流水线

AgentLoop SHALL 拆分为以下阶段：
1. **WorldStateBuilder**: 从 World 构建决策所需的 WorldState
2. **DecisionPhase**: 调用 DecisionPipeline 获取动作
3. **ApplyPhase**: 应用动作到 World，生成 ActionResult
4. **MemoryRecordPhase**: 记录动作到 Agent 记忆系统
5. **DeltaEmitter**: 构建 AgentDelta 并发送到 delta_tx
6. **NarrativeEmitter**: 提取叙事事件并发送到 narrative_tx

run_agent_loop() SHALL 只协调各阶段调用，不包含具体逻辑。

#### Scenario: AgentLoop 使用阶段模块

- **WHEN** run_agent_loop() 执行一个决策周期
- **THEN** 使用：
  ```rust
  let state = WorldStateBuilder::build(&world, &agent_id);
  let decision = DecisionPhase::execute(&pipeline, &state);
  let result = ApplyPhase::apply(&world, &decision);
  MemoryRecordPhase::record(&world, &agent_id, &result);
  DeltaEmitter::emit(&delta_tx, &result);
  NarrativeEmitter::emit(&narrative_tx, &world);
  ```
- **AND** run_agent_loop() 行数 < 100

### Requirement: 各阶段职责单一

每个阶段模块 SHALL 只负责一个职责：
- WorldStateBuilder: 只构建状态，不执行 I/O
- DecisionPhase: 只调用 Pipeline，不修改 World
- ApplyPhase: 只应用动作，不发送事件
- MemoryRecordPhase: 只记录记忆，不构建 Delta
- DeltaEmitter: 只构建和发送 Delta，不处理其他
- NarrativeEmitter: 只提取和发送叙事，不修改状态

#### Scenario: ApplyPhase 不发送 Delta

- **WHEN** ApplyPhase.apply() 完成
- **THEN** 返回 ActionResult
- **AND** 不调用 delta_tx.send()

#### Scenario: DeltaEmitter 从 ActionResult 构建

- **WHEN** DeltaEmitter.emit() 被调用
- **THEN** 从 ActionResult 构建 AgentDelta
- **AND** 发送到 delta_tx