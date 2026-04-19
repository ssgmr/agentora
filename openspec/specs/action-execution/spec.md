# 动作执行

## Purpose

定义 World::apply_action() 中所有动作类型的执行逻辑，包括 Move、Gather、Wait、Trade、Talk、Attack、Build、Explore、AllyPropose/AllyAccept/AllyReject、InteractLegacy 的完整处理流程。

## Requirements

### Requirement: Move 动作执行

`World::apply_action()` SHALL 正确处理 `Move` 动作类型，调用 Agent 移动模块执行真实移动。

#### Scenario: 正常移动

- **WHEN** Agent 执行 Move 动作
- **THEN** 系统 SHALL 调用 `Agent::move_direction()` 执行移动
- **AND** 移动 SHALL 包含边界检查和地形通行性检查
- **AND** 移动成功后 SHALL 记录叙事事件

#### Scenario: 移动至不可通行格

- **WHEN** Agent 尝试移动至山地或水域
- **THEN** 移动 SHALL 被拒绝，Agent 位置不变

#### Scenario: 移动被 Fence 阻挡

- **WHEN** Agent 尝试 Move 到 Fence 所在格
- **AND** Agent 与 Fence 所有者为 Enemy 关系
- **THEN** 动作被 Blocked，返回错误叙事

#### Scenario: 非敌对 Agent 通过 Fence

- **WHEN** Agent 尝试 Move 到 Fence 所在格
- **AND** Agent 与 Fence 所有者非 Enemy 关系
- **THEN** 移动正常执行

### Requirement: Gather 动作执行

`World::apply_action()` SHALL 正确处理 `Gather` 动作类型，调用 Agent 采集模块。

#### Scenario: 成功采集

- **WHEN** Agent 在资源格执行 Gather 动作
- **THEN** 系统 SHALL 调用 `Agent::gather()` 执行采集
- **AND** 采集 SHALL 包含资源节点库存检查和背包容量检查
- **AND** 采集成功后 SHALL 记录资源类型和数量到叙事事件

#### Scenario: 资源不足

- **WHEN** 资源节点库存为 0
- **THEN** 采集 SHALL 失败

### Requirement: Wait 动作执行

`World::apply_action()` SHALL 正确处理 `Wait` 动作类型，专注饮食恢复。

#### Scenario: Wait 有食物和水

- **WHEN** Agent 执行 Wait 动作
- **AND** 背包 Food ≥ 1, Water ≥ 1
- **THEN** 消耗 1 Food, satiety +30; 消耗 1 Water, hydration +25; HP 不变

#### Scenario: Wait 无食物无水

- **WHEN** Agent 执行 Wait 动作
- **AND** 背包无 Food 也无 Water
- **THEN** satiety 不变, hydration 不变, HP 不变（纯休息）

#### Scenario: Wait 仅有食物

- **WHEN** Agent 执行 Wait 动作
- **AND** 背包有 Food ≥ 1 且 Water = 0
- **THEN** 消耗 1 Food, satiety +30; hydration 不变; HP 不变

#### Scenario: Wait 仅有水

- **WHEN** Agent 执行 Wait 动作
- **AND** 背包 Food = 0 且 Water ≥ 1
- **THEN** 消耗 1 Water, hydration +25; satiety 不变; HP 不变

### Requirement: TradeOffer 动作执行

`World::apply_action()` SHALL 正确处理 `TradeOffer` 动作类型。

#### Scenario: 发起交易提议

- **WHEN** Agent A 执行 `TradeOffer` 动作，目标为同格的 Agent B
- **THEN** 系统 SHALL 调用 `Agent::propose_trade()` 生成交易提议
- **AND** 提议 SHALL 记录 offer 资源和 want 资源
- **AND** 交易状态 SHALL 标记为 pending
- **AND** 叙事事件 SHALL 记录 "A 向 B 提议交易"

### Requirement: TradeAccept 动作执行

`World::apply_action()` SHALL 正确处理 `TradeAccept` 动作类型。

#### Scenario: 接受交易

- **WHEN** Agent B 执行 `TradeAccept` 动作，响应 Agent A 的交易提议
- **THEN** 系统 SHALL 调用 `Agent::accept_trade()` 验证双方资源充足性
- **AND** 双方 SHALL 原子交换 offer/want 资源
- **AND** 双方关系信任值 SHALL 增加
- **AND** 若发起方资源不足，交易 SHALL 失败并记录欺诈事件
- **AND** 叙事事件 SHALL 记录交易结果

### Requirement: TradeReject 动作执行

`World::apply_action()` SHALL 正确处理 `TradeReject` 动作类型。

#### Scenario: 拒绝交易

- **WHEN** Agent B 执行 `TradeReject` 动作
- **THEN** 交易状态 SHALL 标记为 rejected
- **AND** 双方资源 SHALL 不变
- **AND** 发起方对 B 的关系信任值 SHALL 略微下降

### Requirement: Talk 动作执行

`World::apply_action()` SHALL 正确处理 `Talk` 动作类型，调用 Agent 对话模块。

#### Scenario: 发起对话

- **WHEN** Agent A 执行 `Talk` 动作，目标为同格的 Agent B
- **THEN** 系统 SHALL 调用 `Agent::talk()` 生成对话消息
- **AND** 消息 SHALL 基于双方动机、库存、关系生成内容
- **AND** 消息 SHALL 记录到双方的对话历史
- **AND** 叙事事件 SHALL 记录对话摘要

#### Scenario: 对话轮次限制

- **WHEN** 同一对 Agent 在连续 tick 中互相 Talk
- **THEN** 对话 SHALL 最多持续 3 轮
- **AND** 超过轮次后系统 SHALL 终止对话状态

### Requirement: Attack 动作执行

`World::apply_action()` SHALL 正确处理 `Attack` 动作类型，调用 Agent 战斗模块。

#### Scenario: 成功攻击

- **WHEN** Agent A 执行 `Attack` 动作，目标为同格的 Agent B
- **THEN** 系统 SHALL 调用 `Agent::attack()` 计算伤害
- **AND** Agent B 生命值 SHALL 降低 10~30 点
- **AND** Agent A SHALL 获取 B 的 1~3 个资源
- **AND** 双方关系 SHALL 标记为敌对
- **AND** 叙事事件 SHALL 记录攻击结果

#### Scenario: 攻击导致死亡

- **WHEN** Agent B 生命值因攻击降至 0 或以下
- **THEN** Agent B SHALL 标记为死亡（`is_alive = false`）
- **AND** 系统 SHALL 触发 Legacy 流程
- **AND** Agent B 的背包资源 SHALL 散落在原位置
- **AND** 叙事事件 SHALL 记录死亡信息

#### Scenario: 攻击无效目标

- **WHEN** 攻击目标不在同格或不存在
- **THEN** 攻击 SHALL 被拒绝
- **AND** Agent 位置和资源 SHALL 不变
- **AND** 系统 SHALL 记录警告日志

### Requirement: Build 动作执行

`World::apply_action()` SHALL 正确处理 `Build` 动作类型，消耗资源并在地图上放置建筑。

#### Scenario: 成功建造

- **WHEN** Agent 执行 `Build` 动作且背包资源满足建造需求
- **THEN** 系统 SHALL 扣除对应资源
- **AND** 地图对应格子 SHALL 标记为建筑
- **AND** 建筑 SHALL 记录创建者 ID 和类型
- **AND** 叙事事件 SHALL 记录建筑信息

#### Scenario: 资源不足建造失败

- **WHEN** Agent 背包资源不满足建造需求
- **THEN** 建造 SHALL 失败
- **AND** Agent 资源 SHALL 不变
- **AND** 系统 SHALL 记录资源不足错误

### Requirement: Explore 动作执行

`World::apply_action()` SHALL 正确处理 `Explore` 动作类型。

#### Scenario: 探索行为

- **WHEN** Agent 执行 `Explore` 动作
- **THEN** Agent SHALL 向随机可通行方向移动
- **AND** 系统 SHALL 检查新位置的资源、Agent、结构
- **AND** 发现结果 SHALL 记录到 Agent 短期记忆
- **AND** 叙事事件 SHALL 记录探索结果

### Requirement: AllyPropose 动作执行

`World::apply_action()` SHALL 正确处理 `AllyPropose` 动作类型。

#### Scenario: 发起结盟

- **WHEN** Agent 执行 `AllyPropose` 动作
- **THEN** 系统 SHALL 调用 Agent 的结盟方法
- **AND** 结盟提议 SHALL 记录到双方关系
- **AND** 叙事事件 SHALL 记录结盟提议

### Requirement: AllyAccept 动作执行

`World::apply_action()` SHALL 正确处理 `AllyAccept` 动作类型。

#### Scenario: 接受结盟

- **WHEN** Agent 执行 `AllyAccept` 动作
- **THEN** 双方关系 SHALL 设为盟友
- **AND** 双方交易效率 SHALL 提升
- **AND** 叙事事件 SHALL 记录结盟成功

### Requirement: AllyReject 动作执行

`World::apply_action()` SHALL 正确处理 `AllyReject` 动作类型。

#### Scenario: 拒绝结盟

- **WHEN** Agent 执行 `AllyReject` 动作
- **THEN** 双方关系 SHALL 略微下降
- **AND** 叙事事件 SHALL 记录结盟被拒

### Requirement: InteractLegacy 动作执行

`World::apply_action()` SHALL 正确处理 `InteractLegacy` 动作类型。

#### Scenario: 交互遗产

- **WHEN** Agent 执行 `InteractLegacy` 动作
- **THEN** Agent SHALL 获取遗产中的资源或知识
- **AND** 遗产内容 SHALL 减少或标记为已吸收
- **AND** Agent 的认知动机 SHALL 获得提升
- **AND** 叙事事件 SHALL 记录遗产交互内容
