# 需求说明书

## 背景概述

当前动作系统定义了 **16 种 ActionType**，但经过审计发现 **AI 实际可用仅 6 种**（MoveToward, Gather, Eat, Drink, Build, Wait）。其余 10 种动作存在不同程度的可用性问题：

### 问题分类

| 类别 | 动作 | 根因 |
|------|------|------|
| **语义重叠** | Explore | 本质是随机 MoveToward，Prompt 说 1-3 步实际只 1 步 |
| **Prompt 缺 params** | Talk, Attack | 有动作说明但无参数格式，AI 不知道如何传参 |
| **Prompt 完全缺失** | TradeOffer/Accept/Reject, AllyPropose/Accept/Reject, InteractLegacy | Prompt 没有描述这些动作，AI 不知道它们存在 |
| **感知缺 ID** | nearby_agents | 有 id 字段但未输出，社交动作无法获取 target_id |
| **感知缺 pending 信息** | TradeAccept/Reject, AllyAccept/Reject | 无 pending_trades/pending_ally_requests，AI 不知道有待处理的交易/结盟请求 |
| **感知缺 legacy_id** | InteractLegacy | NearbyLegacyInfo 缺 legacy_id 字段 |

### 影响

- 交易系统完全不可用（AI 不知道 TradeOffer 存在，也不知道有人对自己发起交易）
- 结盟系统完全不可用（同理）
- Talk/Attack 传参困难（AI 不知道 target_id 从哪获取）
- 遗产交互不可用（无 legacy_id）

## 变更目标

### 删除动作
- 删除 `ActionType::Explore` 枚举变体及相关实现（语义重叠）

### Prompt 补充
- 补充 Talk/Attack 的 params 参数说明
- 新增 TradeOffer/Accept/Reject 动作说明及 params
- 新增 AllyPropose/Accept/Reject 动作说明及 params
- 新增 InteractLegacy 动作说明及 params

### 感知数据增强
- nearby_agents 输出 Agent ID（格式：`名字 [ID:xxx] ...`）
- 新增 pending_trades 列表（待处理的交易提议）
- 新增 pending_ally_requests 列表（待处理的结盟请求）
- NearbyLegacyInfo 增加 legacy_id 字段并输出

### 清理工作
- 清理 Explore 相关的所有引用（解析器、叙事、Spark、记忆、策略等）
- 确保编译和测试通过

## 功能范围

### 新增功能

无新增功能。

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `prompt-rules-manual` | 补充 Talk/Attack params；新增 Trade/Ally/InteractLegacy 动作说明 |
| `perception` | 输出 Agent ID；新增 pending_trades/pending_ally_requests 列表 |
| `world-actions` | 删除 Explore 动作 |
| `npc-rule-engine` | Explore → MoveToward（随机方向） |
| `agent-personality-config` | Explore → MoveToward |

## 影响范围

### Prompt 补充
- `crates/core/src/prompt.rs` — 动作说明文本

### 感知数据增强
- `crates/core/src/decision/perception.rs` — 感知构建（输出 ID、pending 信息）
- `crates/core/src/decision/world_state.rs` — WorldState 结构（新增 pending 字段）
- `crates/core/src/simulation/state_builder.rs` — WorldStateBuilder（填充 pending 信息）
- `crates/core/src/world/vision.rs` — NearbyLegacyInfo（增加 legacy_id）

### Explore 删除
- `crates/core/src/types.rs` — ActionType 枚举
- `crates/core/src/decision/parser.rs` — JSON 解析
- `crates/core/src/world/actions/mod.rs` — 动作路由
- `crates/core/src/world/actions/movement.rs` — handle_explore
- `crates/core/src/narrative.rs` — EventType 枚举
- `crates/core/src/decision/spark.rs` — SparkType 枚举
- `crates/core/src/memory/chronicle_db.rs` — 检索查询
- `crates/core/src/strategy/retrieve.rs` — Spark 映射
- `crates/core/src/strategy/create.rs` — Spark 映射
- `crates/core/src/simulation/memory_recorder.rs` — 记忆标签
- `crates/core/src/world/feedback.rs` — 反馈解析
- `crates/core/src/world/mod.rs` — 动作权重
- `crates/core/src/world/legacy.rs` — LegacyInteraction 枚举

### API接口
- 无外部 API 变更

### 依赖组件
- 无

### 关联系统
- 无

## 验收标准

### Explore 动作删除
- [ ] `ActionType::Explore` 枚举变体已删除
- [ ] `EventType::Explore` 枚举变体已删除
- [ ] `SparkType::Explore` 枚举变体已删除
- [ ] `LegacyInteraction::Explore` 枚举变体已删除
- [ ] Prompt 中无 Explore 动作说明
- [ ] 解析器中无 Explore 解析逻辑
- [ ] `handle_explore` 方法已删除
- [ ] 所有相关引用已清理

### Prompt 动作说明补充
- [ ] Talk 有 params：`{"message": "对话内容"}`
- [ ] Attack 有 params：`{"target_id": "Agent ID"}`
- [ ] TradeOffer 有完整说明和 params
- [ ] TradeAccept/Reject 有完整说明和 params
- [ ] AllyPropose/Accept/Reject 有完整说明和 params
- [ ] InteractLegacy 有完整说明和 params

### 感知数据增强
- [ ] nearby_agents 输出包含 ID（格式：`名字 [ID:xxx]`）
- [ ] pending_trades 列表输出（包含 trade_id、提议方、资源详情）
- [ ] pending_ally_requests 列表输出（包含 ally_id、提议方）
- [ ] nearby_legacies 输出包含 legacy_id

### 编译与测试
- [ ] `cargo build` 编译通过
- [ ] `cargo test` 测试通过