# 功能规格说明

## ADDED Requirements

### Requirement: 暂停控制机制

系统 SHALL 提供暂停控制机制，让所有 Agent 决策循环能感知并响应暂停状态。暂停时所有 Agent 决策 SHALL 停止执行。

#### Scenario: 点击暂停按钮

- **WHEN** 玩家点击暂停按钮
- **THEN** 系统 SHALL 设置全局暂停状态为 true
- **AND** 所有 Agent 决策循环 SHALL 停止执行决策
- **AND** 世界时间 SHALL 停止推进

#### Scenario: 暂停期间决策日志停止

- **WHEN** 系统处于暂停状态
- **THEN** Agent 决策日志 SHALL 不再输出新内容
- **AND** LLM 调用 SHALL 不执行

#### Scenario: 恢复运行

- **WHEN** 玩家再次点击暂停按钮（解除暂停）
- **THEN** 系统 SHALL 设置全局暂停状态为 false
- **AND** 所有 Agent 决策循环 SHALL 立即恢复运行
- **AND** 世界时间 SHALL 恢复推进

#### Scenario: 暂停期间可注入偏好

- **WHEN** 系统处于暂停状态
- **AND** 玩家注入临时偏好
- **THEN** 系统 SHALL 成功注入偏好到 Agent
- **AND** 偏好 SHALL 在恢复运行后正确传递给 LLM Prompt

### Requirement: 世界时间推进

系统 SHALL 定期调用 world.tick() 推进世界时间，确保临时偏好衰减、压力事件触发和资源刷新正常工作。

#### Scenario: 世界时间定期推进

- **WHEN** 系统正常运行（非暂停）
- **THEN** 系统 SHALL 每隔固定间隔调用 world.tick()
- **AND** world.tick 计数 SHALL 递增

#### Scenario: 暂停时世界时间停止

- **WHEN** 系统处于暂停状态
- **THEN** world.tick() SHALL 不被调用
- **AND** world.tick 计数 SHALL 保持不变

#### Scenario: 临时偏好衰减

- **WHEN** world.tick() 被调用
- **THEN** 所有 Agent 的临时偏好 remaining_ticks SHALL 递减
- **AND** remaining_ticks 为 0 的偏好 SHALL 被移除

## MODIFIED Requirements

### Requirement: guide-enhancement 临时偏好注入验证

（来自 openspec/specs/guide-enhancement/spec.md）

系统 SHALL 确保临时偏好注入后能正确传递给 LLM Prompt。注入的偏好 SHALL 在 Prompt 日志中以 `<guidance>` 标签形式显示。

#### Scenario: 注入偏好后 Prompt 包含 guidance

- **WHEN** 玩家注入临时偏好（如点击"生存"按钮）
- **AND** Agent 执行下一次决策
- **THEN** LLM Prompt SHALL 包含 `<guidance>` 标签
- **AND** 标签内容 SHALL 显示注入的偏好类型、强度和剩余回合

#### Scenario: agent_id 匹配验证

- **WHEN** 玩家注入临时偏好指定 agent_id
- **THEN** 系统 SHALL 验证 agent_id 与实际运行 Agent ID 匹配
- **AND** 若不匹配 SHALL 输出警告日志