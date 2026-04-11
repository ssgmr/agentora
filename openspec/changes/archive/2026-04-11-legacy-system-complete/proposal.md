## Why

遗产系统当前有框架实现但缺少关键功能：EchoLog 使用 LLM 压缩记忆未实现，遗迹交互逻辑未集成，遗产 GossipSub 广播未实现，遗迹衰减逻辑不完整。这导致 Agent 死亡后无法形成"死亡→遗迹→回响→他人交互"的闭环，无法验证"遗产沉淀成为新 Spark"的设计假设。

## What Changes

- **新增** LLM 回响压缩（死亡时压缩最后 3 条短期记忆为回响日志）
- **新增** 遗迹交互逻辑（祭拜/探索/拾取）
- **新增** 遗产 GossipSub 广播（死亡事件广播到全网）
- **新增** 遗迹衰减逻辑（物品 50 tick 后每 tick 衰减 10%）
- **新增** 遗产交互动机反馈（探索遗迹→认知/传承动机激励）

## Capabilities

### New Capabilities

- `echo-log-compression`: LLM 回响压缩，死亡时压缩最后 3 条短期记忆为回响日志
- `legacy-interaction`: 遗迹交互逻辑，祭拜/探索/拾取动作及动机反馈
- `legacy-gossip-broadcast`: 遗产 GossipSub 广播，死亡事件广播到全网成为他人 Spark
- `legacy-decay`: 遗迹衰减逻辑，物品 50 tick 后每 tick 衰减 10% 直至消失

### Modified Capabilities

- `legacy-system`: 完整遗产闭环实现

## Impact

- **affected crates**: `core` (legacy 模块), `network` (遗产广播)
- **dependencies**: `ai` (LLM 压缩), `sync` (CRDT 广播)
- **breaking changes**: 无，当前遗产系统为框架实现
- **integration points**: World::apply_action 处理死亡判定；Agent 交互逻辑增加遗迹交互
