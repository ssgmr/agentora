# Agent 状态 — 移除动机关联

## MODIFIED Requirements

### Requirement: Agent 核心实体

系统 SHALL 为每个 Agent 维护物理状态和社交关系，不再维护动机向量。

#### Scenario: Agent 创建

- **WHEN** 创建新 Agent
- **THEN** Agent SHALL 包含以下状态字段：
  - id, name, position
  - health, max_health
  - satiety（饱食度）, hydration（水分度）
  - inventory（背包）
  - relations（社交关系）
  - personality（人格种子，仅用于角色配置）
  - age, max_age, is_alive
  - experience, level
  - last_action_type, last_action_result
  - temp_preferences（保留，用于运行时动态倾向调整）
- **AND** Agent SHALL 不再包含 motivation 字段

#### Scenario: Agent 状态快照

- **WHEN** 生成 AgentSnapshot 用于序列化
- **THEN** 快照 SHALL 包含所有状态字段
- **AND** 快照 SHALL 不再包含 motivation 数组

### Requirement: World Tick 行为

系统 SHALL 在每个 tick 中更新 Agent 状态，不再更新动机向量。

#### Scenario: 状态更新

- **WHEN** World.advance_tick() 被调用
- **THEN** 系统 SHALL 执行：
  - 生存消耗 tick（satiety/hydration 衰减）
  - 建筑效果 tick
  - Agent 临时偏好 tick 衰减
  - 环境压力 tick
  - 死亡检查
  - 策略衰减
- **AND** 系统 SHALL 不再调用 agent.motivation.decay()

#### Scenario: 动作应用

- **WHEN** World.apply_action() 被执行
- **THEN** 系统 SHALL：
  - 执行动作对应的状态修改
  - 记录 last_action_type 和 last_action_result
  - 给予经验值
  - 检查策略创建条件
- **AND** 系统 SHALL 不再应用 motivation_delta
- **AND** 系统 SHALL 不再调用策略-动机联动函数
