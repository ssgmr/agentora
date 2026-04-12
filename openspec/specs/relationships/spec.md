# Capability: Agent 关系上下文

## Purpose
在视野感知和决策 Prompt 中注入 Agent 间的关系数据，支持社交决策。

## 需求

### 需求：关系数据暴露到决策管道
系统 SHALL 在构建 `WorldState` 时，为视野范围内的每个其他 Agent 填充关系信息：
- `relation_type`：从当前 Agent 的 `relations` 中查找，不存在则为 `Neutral`
- `trust`：信任值，不存在则为 `0.0`
- `distance`：曼哈顿距离

### 需求：关系信息注入决策 Prompt
系统 SHALL 在 `build_perception_summary` 中将关系信息格式化为可读文本，例如：
- "Agent_Alice 在东方3格处（朋友，信任0.7）"
- "Agent_Bob 在西方5格处（敌人，信任-0.3）"
- "Agent_Charlie 在南方2格处（陌生人）"

### 需求：关系交互后更新 last_interaction_tick
系统 SHALL 在 Talk、Attack、AllyAccept 等交互动作执行后，更新双方 Agent 之间的 `Relation.last_interaction_tick` 为当前 tick。
