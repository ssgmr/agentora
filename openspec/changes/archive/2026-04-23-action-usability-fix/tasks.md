# 实施任务清单

## 1. Explore 动作删除

删除 Explore 动作类型及相关引用。

- [x] 1.1 删除 ActionType::Explore 枚举变体
  - 文件: `crates/core/src/types.rs`
  - 删除 `Explore { target_region: u32 }` 变体

- [x] 1.2 删除 EventType::Explore 枚举变体和叙事方法
  - 文件: `crates/core/src/narrative.rs`
  - 删除 `Explore` EventType 变体
  - 删除 `explore()` 方法
  - 删除 `action_type_display()` 中的 Explore 分支

- [x] 1.3 删除 SparkType::Explore 枚举变体
  - 文件: `crates/core/src/decision/spark.rs`
  - 删除 `Explore` SparkType 变体
  - 删除相关映射

- [x] 1.4 删除 LegacyInteraction::Explore 枚举变体
  - 文件: `crates/core/src/world/legacy.rs`
  - 删除 `Explore` LegacyInteraction 变体

- [x] 1.5 删除 Explore 解析逻辑
  - 文件: `crates/core/src/decision/parser.rs`
  - 删除 `parse_explore` 函数
  - 删除匹配分支 `"Explore" | "explore" | "探索"`

- [x] 1.6 删除 handle_explore 方法
  - 文件: `crates/core/src/world/actions/movement.rs`
  - 删除 `handle_explore` 方法

- [x] 1.7 删除动作路由中的 Explore 分支
  - 文件: `crates/core/src/world/actions/mod.rs`
  - 删除 `ActionType::Explore { .. } => self.handle_explore(agent_id)` 分支

- [x] 1.8 清理 Explore 相关引用
  - 文件: `crates/core/src/memory/chronicle_db.rs`
  - 删除 Explore SparkType 查询映射
  - 文件: `crates/core/src/strategy/retrieve.rs`
  - 删除 Explore Spark 映射
  - 文件: `crates/core/src/strategy/create.rs`
  - 删除 Explore Spark 映射
  - 文件: `crates/core/src/simulation/memory_recorder.rs`
  - 删除 Explore 记忆标签
  - 文件: `crates/core/src/world/feedback.rs`
  - 删除 Explore 反馈解析
  - 文件: `crates/core/src/world/mod.rs`
  - 删除 Explore 动作权重
  - 文件: `crates/core/src/rule_engine.rs`
  - 删除 Explore 校验分支

## 2. 感知数据结构增强

新增 WorldState 字段和 NearbyLegacyInfo 字段。

- [x] 2.1 新增 PendingTradeInfo 和 PendingAllyRequestInfo 结构体
  - 文件: `crates/core/src/decision/world_state.rs`
  - 新增 `PendingTradeInfo` 结构体（trade_id, proposer_name, proposer_id, offer, want）
  - 新增 `PendingAllyRequestInfo` 结构体（ally_id, proposer_name）
  - WorldState 新增 `pending_trades: Vec<PendingTradeInfo>` 字段
  - WorldState 新增 `pending_ally_requests: Vec<PendingAllyRequestInfo>` 字段

- [x] 2.2 NearbyLegacyInfo 新增 legacy_id 字段
  - 文件: `crates/core/src/world/vision.rs`
  - NearbyLegacyInfo 新增 `legacy_id: String` 字段
  - 修改 `scan_vision` 中填充 NearbyLegacyInfo 的逻辑，从 Legacy.id 获取

## 3. WorldStateBuilder 增强

填充 pending_trades 和 pending_ally_requests。

- [x] 3.1 填充 pending_trades
  - 文件: `crates/core/src/simulation/state_builder.rs`
  - 从 World.pending_trades 过滤 acceptor_id == agent_id 且 status == Pending
  - 构建 PendingTradeInfo 列表

- [x] 3.2 填充 pending_ally_requests
  - 文件: `crates/core/src/simulation/state_builder.rs`
  - 先检查 Agent.relations 是否有 pending_alliance_proposal 字段（如无，需在 Agent 模块新增）
  - 从 Agent.relations 过滤 pending_alliance_proposal == true
  - 构建 PendingAllyRequestInfo 列表

## 4. PerceptionBuilder 增强

输出 Agent ID、pending 信息、legacy_id。

- [x] 4.1 nearby_agents 输出 Agent ID
  - 文件: `crates/core/src/decision/perception.rs`
  - 修改 `build_nearby_agents_section`，在输出格式中增加 `[ID:xxx]`

- [x] 4.2 新增 pending_trades 输出
  - 文件: `crates/core/src/decision/perception.rs`
  - 新增 `build_pending_trades_section` 方法
  - 在 `build_perception_summary` 中调用（在 nearby_agents 之后）

- [x] 4.3 新增 pending_ally_requests 输出
  - 文件: `crates/core/src/decision/perception.rs`
  - 新增 `build_pending_ally_requests_section` 方法
  - 在 `build_perception_summary` 中调用（在 pending_trades 之后）

- [x] 4.4 nearby_legacies 输出 legacy_id
  - 文件: `crates/core/src/decision/perception.rs`
  - 修改 `build_legacies_section`，在输出格式中增加 `[ID:xxx]`

## 5. Prompt 动作说明补充

补充所有缺失动作的 params 说明。

- [x] 5.1 补充 Talk/Attack params 说明
  - 文件: `crates/core/src/prompt.rs`
  - Talk 增加 `params: {"message": "对话内容"}`
  - Attack 增加 `params: {"target_id": "Agent ID"}`

- [x] 5.2 新增 TradeOffer/Accept/Reject 动作说明
  - 文件: `crates/core/src/prompt.rs`
  - TradeOffer 说明 + params 格式
  - TradeAccept 说明 + params 格式
  - TradeReject 说明 + params 格式

- [x] 5.3 新增 AllyPropose/Accept/Reject 动作说明
  - 文件: `crates/core/src/prompt.rs`
  - AllyPropose 说明 + params 格式
  - AllyAccept 说明 + params 格式
  - AllyReject 说明 + params 格式

- [x] 5.4 新增 InteractLegacy 动作说明
  - 文件: `crates/core/src/prompt.rs`
  - InteractLegacy 说明 + params 格式

- [x] 5.5 删除 Explore 动作说明
  - 文件: `crates/core/src/prompt.rs`
  - 删除 `- Explore: 随机移动1-3步探索` 行

## 6. 编译与测试

验证变更后系统正常工作。

- [x] 6.1 cargo build 编译通过
  - 运行 `cargo build` 验证无编译错误

- [x] 6.2 cargo test 测试通过
  - 运行 `cargo test` 验证所有测试通过

- [x] 6.3 构建并复制 bridge
  - 运行 `cargo build --release -p agentora-bridge`
  - 复制 agentora_bridge.dll 到 client/bin/

## 任务依赖关系

```
1.x (Explore删除) ─────────────────────────────────────────┐
                                                            │
2.x (感知数据结构) ──▶ 3.x (WorldStateBuilder) ──▶ 4.x (PerceptionBuilder) ──┤
                                                                    │
5.x (Prompt补充) ─────────────────────────────────────────────────┤
                                                                    │
6.x (编译测试) ────────────────────────────────────────────────────┘
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
|------|------|------|
| 阶段一 | 1.x + 5.5 | 删除 Explore（可独立完成） |
| 阶段二 | 2.x | 新增数据结构（基础） |
| 阶段三 | 3.x | WorldStateBuilder 填充（依赖 2.x） |
| 阶段四 | 4.x | PerceptionBuilder 输出（依赖 2.x, 3.x） |
| 阶段五 | 5.x | Prompt 补充（可并行） |
| 阶段六 | 6.x | 编译验证（依赖所有） |

## 文件结构总览

```
crates/core/src/
├── types.rs                 [修改] 删除 ActionType::Explore
├── prompt.rs                [修改] 补充动作说明
├── decision/
│   ├── world_state.rs       [修改] 新增 pending 字段和结构体
│   ├── perception.rs        [修改] 输出 ID 和 pending 信息
│   ├── parser.rs            [修改] 删除 parse_explore
│   └── spark.rs             [修改] 删除 SparkType::Explore
├── world/
│   ├── vision.rs            [修改] NearbyLegacyInfo 新增 legacy_id
│   ├── legacy.rs            [修改] 删除 LegacyInteraction::Explore
│   ├── mod.rs               [修改] 删除 Explore 权重
│   ├── feedback.rs          [修改] 删除 Explore 反馈解析
│   └── actions/
│       ├── mod.rs           [修改] 删除 Explore 分支
│       └── movement.rs      [修改] 删除 handle_explore
├── simulation/
│   ├── state_builder.rs     [修改] 填充 pending 信息
│   └── memory_recorder.rs   [修改] 删除 Explore 标签
├── memory/
│   └── chronicle_db.rs      [修改] 删除 Explore 查询映射
├── strategy/
│   ├── retrieve.rs          [修改] 删除 Explore 映射
│   └── create.rs            [修改] 删除 Explore 映射
├── narrative.rs             [修改] 删除 EventType::Explore
└── rule_engine.rs           [修改] 删除 Explore 校验
```