# 功能规格增量说明

## REMOVED Requirements

### Requirement: Explore 动作执行

**原因**：Explore 动作与 MoveToward 语义重叠，且实现存在信息不一致问题（Prompt 说 1-3 步实际只 1 步），简化动作系统。

**迁移方案**：使用 `MoveToward` 动作配合随机方向实现探索行为。Agent 可通过决策自行选择探索方向，无需专门的 Explore 动作。