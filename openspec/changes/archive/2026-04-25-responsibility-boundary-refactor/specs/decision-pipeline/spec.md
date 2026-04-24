# 功能规格说明 - DecisionPipeline 职责收缩（增量）

## MODIFIED Requirements

### Requirement: DecisionPipeline 执行决策流程

DecisionPipeline SHALL 只负责以下职责：
- 接收 WorldState 和记忆摘要
- 调用 PromptBuilder 组装 Prompt
- 调用 LLM Provider 生成候选动作
- 调用 RuleEngine 校验候选动作
- 返回 DecisionResult

DecisionPipeline SHALL **不再负责**：
- ~~build_perception_summary()~~ → 移至 PerceptionBuilder
- ~~build_path_recommendation()~~ → 移至 PerceptionBuilder
- ~~infer_state_mode()~~ → 移至 PerceptionBuilder

#### Scenario: DecisionPipeline 调用 PerceptionBuilder

- **WHEN** DecisionPipeline.execute() 开始
- **THEN** 接收预构建的 perception_summary（来自 PerceptionBuilder）
- **AND** 不调用 build_perception_summary()

#### Scenario: DecisionPipeline 行数限制

- **WHEN** 完成重构后
- **THEN** decision.rs 行数 SHALL < 500
- **AND** 不包含感知构建函数

### Requirement: DecisionPipeline 接口简化

DecisionPipeline.execute() SHALL 接收以下参数：
- agent_id: &AgentId
- world_state: &WorldState
- perception_summary: &str（已预构建）
- memory_summary: Option<&str>
- action_feedback: Option<&str>

#### Scenario: 接口调用示例

- **WHEN** agent_loop 调用 DecisionPipeline
- **THEN** 使用：
  ```rust
  pipeline.execute(&agent_id, &world_state, &perception, memory, feedback)
  ```
- **AND** perception_summary 已由 PerceptionBuilder.build() 生成