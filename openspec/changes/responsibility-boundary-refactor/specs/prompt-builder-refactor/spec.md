# 功能规格说明 - PromptBuilder 职责扩展（增量）

## MODIFIED Requirements

### Requirement: PromptBuilder 组装决策 Prompt

PromptBuilder SHALL 负责组装完整的 LLM 决策 Prompt：
- System Prompt（性格、世界常识）
- Rules Manual（规则数值表）
- Perception Summary（感知环境）
- Memory Summary（三层记忆）
- Strategy Hint（策略参考）
- Action Feedback（上次动作结果）
- Output Format（输出格式指令）

PromptBuilder SHALL 从 PerceptionBuilder 接收感知摘要，不再自行构建。

#### Scenario: PromptBuilder 接收预构建感知

- **WHEN** build_decision_prompt() 被调用
- **THEN** 接收 perception_summary 参数
- **AND** 直接使用，不自行构建

### Requirement: PromptBuilder 包含路径推荐

PromptBuilder SHALL 在 Prompt 中包含路径推荐逻辑：
- 根据生存压力推荐优先资源方向
- 提供正确的 MoveToward direction 参数示例

#### Scenario: 路径推荐在 Prompt 中

- **WHEN** Agent 饱食度 <= 50
- **THEN** Prompt SHALL 包含【推荐路径】段落
- **AND** 提示最近 Food 的方向和距离

## ADDED Requirements

### Requirement: PromptBuilder 分级截断优先级

PromptBuilder SHALL 按以下优先级截断（超出 token 限制时）：
1. ~~策略提示~~（最低优先级，先截断）
2. ~~记忆摘要~~（次低优先级）
3. ~~感知摘要~~（相对高优先级）
4. System Prompt 和 OutputFormat（不截断）

#### Scenario: Token 超限截断策略

- **WHEN** Prompt 总 token > max_tokens
- **THEN** 先截断 strategy_hint
- **AND** 若仍超限，截断 memory_summary
- **AND** 保留 system_prompt 和 output_format