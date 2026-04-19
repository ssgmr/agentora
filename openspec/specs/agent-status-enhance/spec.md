# 功能规格说明

## Requirements

### Requirement: Agent 状态面板显示当前动作

系统 SHALL 在 AgentDetailPanel 中显示选中 Agent 的当前动作信息。

#### Scenario: 显示当前动作

- **WHEN** 用户选中一个 Agent
- **AND** 该 Agent 有当前动作信息
- **THEN** AgentDetailPanel 显示 current_action 文本
- **AND** 文本字号 10，颜色白色

#### Scenario: 无动作时显示默认值

- **WHEN** Agent 无当前动作信息
- **THEN** AgentDetailPanel 显示 "等待" 作为默认值

### Requirement: Agent 状态面板显示动作结果

系统 SHALL 在 AgentDetailPanel 中显示选中 Agent 上次动作的执行结果。

#### Scenario: 显示动作结果

- **WHEN** 用户选中一个 Agent
- **AND** 该 Agent 有上次动作结果
- **THEN** AgentDetailPanel 显示 action_result 文本
- **AND** 文本字号 9，颜色根据结果类型变化

#### Scenario: 成功结果显示绿色

- **WHEN** action_result 不包含 "失败" 或 "被拒绝"
- **THEN** 结果文本颜色为绿色 Color(0.3, 0.8, 0.3)

#### Scenario: 失败结果显示红色

- **WHEN** action_result 包含 "失败" 或 "被拒绝"
- **THEN** 结果文本颜色为红色 Color(0.9, 0.3, 0.3)

#### Scenario: 无结果时显示默认值

- **WHEN** Agent 无上次动作结果
- **THEN** AgentDetailPanel 显示 "无" 作为默认值

### Requirement: Agent 状态面板显示等级

系统 SHALL 在 AgentDetailPanel 中显示选中 Agent 的等级信息。

#### Scenario: 显示等级徽章

- **WHEN** 用户选中一个 Agent
- **THEN** AgentDetailPanel 显示 "Lv.X" 标签
- **AND** 等级数值从 agent.level 获取
- **AND** 文本字号 11，颜色金色 Color(1, 0.8, 0.2)

#### Scenario: 等级动态更新

- **WHEN** Agent 等级变化（升级）
- **AND** AgentDetailPanel 仍在显示该 Agent
- **THEN** 等级标签数值更新为新等级