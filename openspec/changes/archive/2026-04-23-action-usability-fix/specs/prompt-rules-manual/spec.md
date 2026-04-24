# 功能规格增量说明

## ADDED Requirements

### Requirement: Trade 系列动作 Prompt 说明

系统 SHALL 在 Prompt 动作说明中包含 TradeOffer/TradeAccept/TradeReject 的完整描述。

#### Scenario: TradeOffer 说明

- **WHEN** 构建 Prompt 动作说明部分
- **THEN** Prompt SHALL 包含 TradeOffer 动作说明：
  - 动作描述："发起交易提议"
  - params 格式：`{"target_id": "Agent ID", "offer": {"wood": 5}, "want": {"food": 3}}`
  - 前置条件：目标 Agent 在视野内，背包有 offer 所需资源

#### Scenario: TradeAccept 说明

- **WHEN** 构建 Prompt 动作说明部分
- **THEN** Prompt SHALL 包含 TradeAccept 动作说明：
  - 动作描述："接受交易提议"
  - params 格式：`{"trade_id": "交易ID"}`
  - 前置条件：有待处理的交易提议

#### Scenario: TradeReject 说明

- **WHEN** 构建 Prompt 动作说明部分
- **THEN** Prompt SHALL 包含 TradeReject 动作说明：
  - 动作描述："拒绝交易提议"
  - params 格式：`{"trade_id": "交易ID"}`
  - 前置条件：有待处理的交易提议

### Requirement: Ally 系列动作 Prompt 说明

系统 SHALL 在 Prompt 动作说明中包含 AllyPropose/AllyAccept/AllyReject 的完整描述。

#### Scenario: AllyPropose 说明

- **WHEN** 构建 Prompt 动作说明部分
- **THEN** Prompt SHALL 包含 AllyPropose 动作说明：
  - 动作描述："提议结盟"
  - params 格式：`{"target_id": "Agent ID"}`
  - 前置条件：目标 Agent 在视野内，信任值 > 0.5

#### Scenario: AllyAccept 说明

- **WHEN** 构建 Prompt 动作说明部分
- **THEN** Prompt SHALL 包含 AllyAccept 动作说明：
  - 动作描述："接受结盟请求"
  - params 格式：`{"ally_id": "Agent ID"}`
  - 前置条件：有待处理的结盟请求

#### Scenario: AllyReject 说明

- **WHEN** 构建 Prompt 动作说明部分
- **THEN** Prompt SHALL 包含 AllyReject 动作说明：
  - 动作描述："拒绝结盟请求"
  - params 格式：`{"ally_id": "Agent ID"}`
  - 前置条件：有待处理的结盟请求

### Requirement: InteractLegacy 动作 Prompt 说明

系统 SHALL 在 Prompt 动作说明中包含 InteractLegacy 的完整描述。

#### Scenario: InteractLegacy 说明

- **WHEN** 构建 Prompt 动作说明部分
- **THEN** Prompt SHALL 包含 InteractLegacy 动作说明：
  - 动作描述："与遗迹交互"
  - params 格式：`{"legacy_id": "遗迹ID", "interaction": "Worship/Explore/Pickup"}`
  - 前置条件：位于遗迹所在格

## MODIFIED Requirements

### Requirement: Talk 动作 Prompt 说明

原需求：Prompt 只包含动作名称描述，无 params 说明。

修改后：Prompt SHALL 包含 Talk 动作的完整 params 说明。

#### Scenario: Talk 说明完整

- **WHEN** 构建 Prompt 动作说明部分
- **THEN** Prompt SHALL 包含 Talk 动作说明：
  - 动作描述："与附近 Agent 对话"
  - params 格式：`{"message": "对话内容"}`
  - 目标：附近 3 格内的 Agent

### Requirement: Attack 动作 Prompt 说明

原需求：Prompt 只包含动作名称描述，无 params 说明。

修改后：Prompt SHALL 包含 Attack 动作的完整 params 说明。

#### Scenario: Attack 说明完整

- **WHEN** 构建 Prompt 动作说明部分
- **THEN** Prompt SHALL 包含 Attack 动作说明：
  - 动作描述："攻击相邻格 Agent"
  - params 格式：`{"target_id": "Agent ID"}`
  - 前置条件：目标 Agent 在相邻格（曼哈顿距离 ≤ 1）

## REMOVED Requirements

### Requirement: Explore 动作 Prompt 说明

**原因**：Explore 动作与 MoveToward 语义重叠，删除动作类型。

**迁移方案**：使用 MoveToward 配合随机方向实现探索行为。