# 功能规格增量说明

## ADDED Requirements

### Requirement: nearby_agents 输出 Agent ID

系统 SHALL 在感知 nearby_agents 部分输出每个 Agent 的 ID，供 AI 在社交动作中使用。

#### Scenario: 输出 Agent ID

- **WHEN** 构建 nearby_agents 感知部分
- **THEN** 每个 Agent 的输出 SHALL 包含 ID 字段
- **AND** 格式示例：`名字 [ID:abc123] (x,y) [方向] 距离:N格 关系:xxx 信任:x.x`

#### Scenario: AI 获取 ID 用于动作

- **WHEN** AI 需要执行 Attack/Talk/TradeOffer/AllyPropose 动作
- **THEN** AI SHALL 从感知 nearby_agents 中获取目标 Agent 的 ID
- **AND** 使用该 ID 作为 target_id 参数

### Requirement: pending_trades 感知输出

系统 SHALL 在感知中输出待处理的交易提议列表，供 AI 决定是否接受/拒绝。

#### Scenario: 输出 pending_trades

- **WHEN** Agent 有待处理的交易提议
- **THEN** 感知 SHALL 包含 pending_trades 部分
- **AND** 每条交易 SHALL 包含：trade_id、提议方名字/ID、offer 资源、want 资源

#### Scenario: AI 决策交易

- **WHEN** AI 看到 pending_trades 列表
- **THEN** AI SHALL 能根据 trade_id 执行 TradeAccept 或 TradeReject

### Requirement: pending_ally_requests 感知输出

系统 SHALL 在感知中输出待处理的结盟请求列表，供 AI 决定是否接受/拒绝。

#### Scenario: 输出 pending_ally_requests

- **WHEN** Agent 有待处理的结盟请求
- **THEN** 感知 SHALL 包含 pending_ally_requests 部分
- **AND** 每条请求 SHALL 包含：ally_id、提议方名字/ID

#### Scenario: AI 决策结盟

- **WHEN** AI 看到 pending_ally_requests 列表
- **THEN** AI SHALL 能根据 ally_id 执行 AllyAccept 或 AllyReject

### Requirement: nearby_legacies 输出 legacy_id

系统 SHALL 在 nearby_legacies 感知中输出每个遗迹的 legacy_id。

#### Scenario: NearbyLegacyInfo 包含 ID

- **WHEN** 构建 NearbyLegacyInfo 结构体
- **THEN** 结构体 SHALL 包含 legacy_id 字段

#### Scenario: 输出 legacy_id

- **WHEN** 构建 nearby_legacies 感知部分
- **THEN** 每个遗迹的输出 SHALL 包含 legacy_id
- **AND** 格式示例：`(x,y): 遗迹类型 [ID:xxx] (名字的遗迹, 有物品)`

#### Scenario: AI 交互遗迹

- **WHEN** AI 需要执行 InteractLegacy 动作
- **THEN** AI SHALL 从感知 nearby_legacies 中获取 legacy_id
- **AND** 使用该 ID 作为 legacy_id 参数