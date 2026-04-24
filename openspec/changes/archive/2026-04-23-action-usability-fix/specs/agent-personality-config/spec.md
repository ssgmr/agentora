# 功能规格增量说明

## MODIFIED Requirements

### Requirement: 性格模板决策倾向

原需求（部分）：
- "好奇的探索者" SHALL 倾向Explore和Gather动作

修改后：
- "好奇的探索者" SHALL 倾向MoveToward（探索新方向）和Gather动作

#### Scenario: openness影响探索倾向

- **WHEN** Agent openness 较高
- **THEN** Agent SHALL 更倾向选择随机方向的 MoveToward 探索新区域
- **AND** Agent SHALL 不再使用 Explore 动作类型