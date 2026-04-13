# World Actions Spec

## 能力描述

`World::apply_action()` 作为所有行动的统一入口，将每个 ActionType 路由到独立的 handler 方法，确保行动产生真实的世界状态变更。

## 需求

### 需求: Action 路由

**WHEN** `apply_action()` 被调用
**THEN** 执行以下流程：
1. 校验 Agent 存在且存活
2. 根据 `action_type` 路由到对应 handler
3. 统一处理返回结果，失败时生成错误叙事

### 需求: handle_gather

**WHEN** Agent 执行 Gather
**THEN**:
- 调用 `ResourceNode.gather()` 扣除资源节点储量
- Agent 库存 + 实际采集量
- 如果资源节点枯竭，标记 `is_depleted = true`
- 如果资源点不存在于 Agent 位置，返回 `Blocked("当前位置无资源")`

### 需求: handle_build

**WHEN** Agent 执行 Build
**THEN**:
- 校验 `build_type` 参数存在
- 校验 Agent 库存资源足够
- 校验目标位置无已有建筑
- 扣除资源 → 创建 `Structure` → 插入 `world.structures`
- 如果校验失败，返回 `Blocked(reason)` 并生成错误叙事

### 需求: handle_attack

**WHEN** Agent 执行 Attack
**THEN**:
- 调用 `combat.rs` 的 `Agent::attack()` 方法
- 更新目标 HP 和信任关系
- 如果目标死亡，触发 Legacy 流程
- 如果目标不存在或超出范围，返回 `Blocked(reason)`

### 需求: handle_trade_offer / handle_trade_accept

**WHEN** Agent 执行 TradeOffer
**THEN**:
- 调用 `trade.rs` 的 `propose_trade()` 创建待处理交易
- 加入 `world.pending_trades`

**WHEN** Agent 执行 TradeAccept
**THEN**:
- 调用 `trade.rs` 的 `accept_trade()` 执行双向库存交换
- 从 `world.pending_trades` 移除
- 如果交易不匹配或对方未发起，返回 `Blocked(reason)`

### 需求: handle_ally_propose / handle_ally_accept

**WHEN** Agent 执行 AllyPropose
**THEN**:
- 调用 `alliance.rs` 的 `propose_alliance()` 发起结盟请求

**WHEN** Agent 执行 AllyAccept
**THEN**:
- 调用 `alliance.rs` 的 `accept_alliance()` 建立联盟关系
- 修改双方关系状态

### 需求: 错误叙事

**WHEN** 任何 handler 返回 `ActionResult::Blocked(reason)`
**THEN**:
- 生成叙事事件 "Agent{agent_id} 尝试 {action_type} 失败: {reason}"
- 推送到 Godot 叙事流
- 不修改任何世界状态
