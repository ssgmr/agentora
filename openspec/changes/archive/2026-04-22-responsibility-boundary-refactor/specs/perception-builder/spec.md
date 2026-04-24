# 功能规格说明 - 感知构建模块

## ADDED Requirements

### Requirement: 感知构建职责独立

系统 SHALL 创建 PerceptionBuilder 模块，负责从 WorldState 构建 LLM Prompt 所需的感知摘要。

PerceptionBuilder SHALL 包含以下功能：
- build_perception_summary(): 生存状态、背包、位置、地形、资源分布
- build_path_recommendation(): 根据生存压力推荐移动路径
- build_nearby_info(): 附近 Agent/建筑/遗产信息

#### Scenario: PerceptionBuilder 构建生存状态

- **WHEN** Agent 饱食度 <= 30
- **THEN** 感知摘要 SHALL 包含 "⚠️饥饿中！" 警告
- **AND** 提示需要进食

#### Scenario: PerceptionBuilder 构建路径推荐

- **WHEN** Agent 饱食度 <= 50 且视野内有 Food 资源
- **THEN** 路径推荐 SHALL 指出最近 Food 的方向
- **AND** 提供正确的 MoveToward direction 参数

### Requirement: DecisionPipeline 职责收缩

DecisionPipeline SHALL 不再包含感知构建逻辑：
- build_perception_summary() SHALL 移至 PerceptionBuilder
- build_path_recommendation() SHALL 移至 PerceptionBuilder

DecisionPipeline.execute() SHALL 只负责：
- 调用 PerceptionBuilder 获取感知
- 调用 PromptBuilder 组装 Prompt
- 调用 LLM Provider
- 调用 RuleEngine 校验

#### Scenario: DecisionPipeline 调用 PerceptionBuilder

- **WHEN** DecisionPipeline.execute() 开始
- **THEN** 调用 perception_builder.build(world_state)
- **AND** 将结果传递给 PromptBuilder

### Requirement: PromptBuilder 职责扩展

PromptBuilder SHALL 扩展职责：
- 接收 PerceptionBuilder 输出
- 组装 System Prompt + Rules + Perception + Memory + Strategy + OutputFormat
- 执行分级截断（策略 → 记忆 → 感知）

#### Scenario: PromptBuilder 组装完整 Prompt

- **WHEN** build_decision_prompt() 被调用
- **THEN** Prompt SHALL 包含规则说明书（RulesManual）
- **AND** Prompt SHALL 包含性格描述（PersonalitySeed）
- **AND** Prompt SHALL 包含感知摘要（来自 PerceptionBuilder）