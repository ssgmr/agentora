# 功能规格说明

## ADDED Requirements

### Requirement: DecisionPipeline 必须注入 StrategyHub

Simulation 在创建 DecisionPipeline 时，SHALL 构造 StrategyHub 并通过 `.with_strategy_hub()` 注入到 DecisionPipeline 中。

#### Scenario: 正常注入

- **WHEN** Simulation::new() 创建 DecisionPipeline
- **THEN** 系统 SHALL 构造 StrategyHub（指向 Agent 策略目录）
- **AND** 系统 SHALL 调用 `.with_strategy_hub(hub)` 注入 DecisionPipeline
- **AND** DecisionPipeline 的 strategy_hub 字段 SHALL 不为 None

#### Scenario: Agent 独立策略目录

- **WHEN** 为每个 Agent 创建 DecisionPipeline
- **THEN** 每个 StrategyHub SHALL 指向 `~/.agentora/agents/<agent_id>/strategies/`
- **AND** 不同 Agent 的策略库 SHALL 相互独立

### Requirement: DecisionPipeline 构建 Prompt 时检索策略

当 DecisionPipeline 的 strategy_hub 不为 None 时，SHALL 在 build_prompt 中检索匹配当前情境的策略并注入 Prompt。

#### Scenario: 检索匹配策略

- **WHEN** DecisionPipeline.build_prompt() 执行
- **THEN** 系统 SHALL 调用 `infer_state_mode(world_state)` 获取当前 SparkType
- **AND** 系统 SHALL 通过 StrategyHub 检索匹配该 SparkType 的策略
- **AND** 检索到策略时，SHALL 格式化为 "策略：{spark_type} (成功率 {rate}%, 使用{count}次)\n推荐：{first_line}"

#### Scenario: 策略注入 Prompt

- **WHEN** 检索到匹配策略
- **THEN** 系统 SHALL 使用 `<strategy-context>` 标签包裹策略内容
- **AND** Prompt 中 SHALL 包含策略上下文段

#### Scenario: 无匹配策略

- **WHEN** 检索未找到匹配策略
- **THEN** Prompt 中 SHALL 不包含 `<strategy-context>` 段
- **AND** 决策 SHALL 继续正常执行（不受影响）

### Requirement: 策略创建使用实际决策情境 SparkType

策略创建时，SHALL 使用与检索时相同的 `infer_state_mode(world_state)` 推断 SparkType，而非硬编码固定值。

#### Scenario: 资源匮乏情境下创建策略

- **WHEN** Agent 的 satiety ≤ 30 或 hydration ≤ 30
- **AND** 决策成功执行并满足策略创建条件
- **THEN** 策略的 SparkType SHALL 为 `ResourcePressure`
- **AND** 策略目录 SHALL 为 `resource_pressure/`

#### Scenario: 社交情境下创建策略

- **WHEN** Agent 附近存在其他 Agent
- **AND** 决策成功执行并满足策略创建条件
- **THEN** 策略的 SparkType SHALL 为 `SocialPressure`
- **AND** 策略目录 SHALL 为 `social_pressure/`

#### Scenario: 探索情境下创建策略

- **WHEN** Agent 既不饥饿也不在社交情境中
- **AND** 决策成功执行并满足策略创建条件
- **THEN** 策略的 SparkType SHALL 为 `Explore`
- **AND** 策略目录 SHALL 为 `explore/`

## MODIFIED Requirements

### Requirement: Simulation 作为后端核心编排层

> 来源: `openspec/specs/simulation-orchestrator/spec.md`

Simulation SHALL 只负责以下职责：
- 管理 World 和 DecisionPipeline
- **构造并注入 StrategyHub 到 DecisionPipeline**（新增）
- 控制 Agent 决策循环（通过 AgentLoopController）
- 推进世界 Tick（通过 TickLoopController）
- 生成 Snapshot（通过 SnapshotLoopController）
- 提供公开 API：start/pause/resume/inject_preference/set_tick_interval

#### Scenario: Simulation 注入 StrategyHub

- **WHEN** Simulation::new() 创建
- **THEN** 系统 SHALL 为每个 Agent 构造 StrategyHub
- **AND** 系统 SHALL 通过 `.with_strategy_hub()` 注入对应 DecisionPipeline

### Requirement: 策略创建触发条件

> 来源: `openspec/specs/strategy-create-trigger/spec.md`

系统 SHALL 在成功决策后自动创建策略，当满足以下条件时。

#### Scenario: 成功决策触发

- **WHEN** Agent 执行决策后 Echo 反馈为"成功"
- **AND** 决策涉及 ≥ 3 个候选动作筛选
- **THEN** 系统 SHALL 自动创建策略
- **AND** 策略名 SHALL 使用 `infer_state_mode(world_state)` 推断的 SparkType（而非硬编码值）

## REMOVED Requirements

### Requirement: 策略创建使用硬编码 SparkType

**原因**：硬编码的 SparkType（无论是 `Explore` 还是 `CognitivePressure`）与检索时使用的 `infer_state_mode(world_state)` 不匹配，导致策略检索几乎永远找不到自己创建的条目，破坏了"创建→检索→参考"的闭环。

**迁移方案**：`apply_action()` 中策略创建调用改为传入 `infer_state_mode(world_state)` 的结果。由于 `apply_action()` 当前无法访问 WorldState，需要在调用链中传递或在 `apply_action` 签名中增加 `spark_type` 参数。
