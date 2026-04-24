# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

Agent 决策管道：World → WorldStateBuilder → WorldState → PerceptionBuilder → Prompt → LLM → Action

关键数据结构：
- `WorldState`：决策输入，包含 nearby_agents/nearby_legacies 等
- `NearbyAgentInfo`：已有 `id` 字段但未在感知输出
- `NearbyLegacyInfo`：缺少 `legacy_id` 字段
- World：有 `pending_trades` 但未传入 WorldState

### 1.2 现状分析

| 问题 | 根因 | 影响 |
|------|------|------|
| Explore 冗余 | 与 MoveToward 语义重叠 | 增加 AI 认知负担 |
| Talk/Attack 缺 params | Prompt 只写名称 | AI 不知道传参格式 |
| Trade/Ally 无 Prompt | 动作说明缺失 | AI 不知道这些动作存在 |
| nearby_agents 无 ID | perception.rs 未输出 id 字段 | 社交动作无法获取 target_id |
| 无 pending 信息 | WorldState 未包含 pending_trades/pending_ally | AI 不知道待处理的交易/结盟 |
| nearby_legacies 无 ID | NearbyLegacyInfo 缺 legacy_id 字段 | InteractLegacy 无法传参 |

## 2. 设计目标

### 目标

- 删除 Explore 动作（简化系统）
- Prompt 补全所有社交/交易/结盟/遗产动作说明（让 AI 知道动作存在）
- 感知输出 Agent ID（让 AI 能传 target_id）
- 感知输出 pending_trades/pending_ally（让 AI 能响应交易/结盟请求）
- 感知输出 legacy_id（让 AI 能交互遗迹）

### 非目标

- 不修改动作执行逻辑（Talk/Attack/Trade/Ally 实现已完整）
- 不修改规则引擎校验逻辑
- 不修改 World 核心数据结构

## 3. 整体架构

### 3.1 数据流

```
World.pending_trades ──────────────────┐
                                        │
World.pending_ally_requests (新增) ────┼──▶ WorldStateBuilder ──▶ WorldState
                                        │       (新增 pending 字段)
NearbyAgentInfo.id (已有) ─────────────┤
                                        │
NearbyLegacyInfo (新增 legacy_id) ─────┘

WorldState ──▶ PerceptionBuilder ──▶ Prompt
                    │
                    ├─ 输出 nearby_agents 包含 ID
                    ├─ 输出 pending_trades 列表
                    ├─ 输出 pending_ally_requests 列表
                    └─ 输出 nearby_legacies 包含 ID

Prompt ──▶ LLM ──▶ Action (AI 可正确使用所有社交动作)
```

## 4. 详细设计

### 4.1 Prompt 动作说明修改

**文件**：`crates/core/src/prompt.rs`

**修改点**：`output_format_instructions()` 方法中的动作说明部分

**修改内容**：

```rust
// 删除 Explore 行
// s.push_str("- Explore: 随机移动1-3步探索\n");  // 删除

// 补充 Talk params
s.push_str("- Talk: 与附近Agent对话\n");
s.push_str("    params: {\"message\": \"对话内容\"}\n");  // 新增

// 补充 Attack params
s.push_str("- Attack: 攻击相邻格Agent\n");
s.push_str("    params: {\"target_id\": \"Agent ID\"}\n");  // 新增

// 新增 Trade 系列
s.push_str("- TradeOffer: 发起交易提议\n");
s.push_str("    params: {\"target_id\": \"Agent ID\", \"offer\": {\"wood\": 5}, \"want\": {\"food\": 3}}\n");
s.push_str("- TradeAccept: 接受交易提议\n");
s.push_str("    params: {\"trade_id\": \"交易ID\"}\n");
s.push_str("- TradeReject: 拒绝交易提议\n");
s.push_str("    params: {\"trade_id\": \"交易ID\"}\n");

// 新增 Ally 系列
s.push_str("- AllyPropose: 提议结盟\n");
s.push_str("    params: {\"target_id\": \"Agent ID\"}\n");
s.push_str("- AllyAccept: 接受结盟请求\n");
s.push_str("    params: {\"ally_id\": \"Agent ID\"}\n");
s.push_str("- AllyReject: 拒绝结盟请求\n");
s.push_str("    params: {\"ally_id\": \"Agent ID\"}\n");

// 新增 InteractLegacy
s.push_str("- InteractLegacy: 与遗迹交互\n");
s.push_str("    params: {\"legacy_id\": \"遗迹ID\", \"interaction\": \"Worship/Explore/Pickup\"}\n");
```

### 4.2 感知数据结构修改

#### 4.2.1 WorldState 新增字段

**文件**：`crates/core/src/decision/world_state.rs`

```rust
pub struct WorldState {
    // ... 现有字段 ...
    
    /// 待处理的交易提议（新增）
    pub pending_trades: Vec<PendingTradeInfo>,
    
    /// 待处理的结盟请求（新增）
    pub pending_ally_requests: Vec<PendingAllyRequestInfo>,
}

/// 待处理交易信息（新增）
#[derive(Debug, Clone)]
pub struct PendingTradeInfo {
    pub trade_id: String,
    pub proposer_name: String,
    pub proposer_id: AgentId,
    pub offer: HashMap<ResourceType, u32>,
    pub want: HashMap<ResourceType, u32>,
}

/// 待处理结盟请求信息（新增）
#[derive(Debug, Clone)]
pub struct PendingAllyRequestInfo {
    pub ally_id: AgentId,
    pub proposer_name: String,
}
```

#### 4.2.2 NearbyLegacyInfo 新增字段

**文件**：`crates/core/src/world/vision.rs`

```rust
pub struct NearbyLegacyInfo {
    pub legacy_id: String,        // 新增
    pub position: Position,
    pub legacy_type: LegacyType,
    pub original_agent_name: String,
    pub has_items: bool,
    pub distance: u32,
}
```

### 4.3 WorldStateBuilder 修改

**文件**：`crates/core/src/simulation/state_builder.rs`

**新增逻辑**：从 World 提取 pending_trades 和 pending_ally_requests

```rust
impl WorldStateBuilder {
    pub fn build(&self, world: &World, agent_id: &AgentId) -> WorldState {
        // ... 现有逻辑 ...
        
        // 新增：提取待处理交易
        let pending_trades: Vec<PendingTradeInfo> = world.pending_trades.iter()
            .filter(|t| t.acceptor_id == *agent_id && t.status == TradeStatus::Pending)
            .map(|t| PendingTradeInfo {
                trade_id: t.trade_id.clone(),
                proposer_name: world.agents.get(&t.proposer_id).map(|a| a.name.clone()).unwrap_or_default(),
                proposer_id: t.proposer_id.clone(),
                offer: t.offer_resources.iter()
                    .filter_map(|(k, v)| Some((str_to_resource(k)?, *v)))
                    .collect(),
                want: t.want_resources.iter()
                    .filter_map(|(k, v)| Some((str_to_resource(k)?, *v)))
                    .collect(),
            })
            .collect();
        
        // 新增：提取待处理结盟请求（需要从 Agent.relations 中提取）
        let pending_ally_requests: Vec<PendingAllyRequestInfo> = {
            let agent = world.agents.get(agent_id).unwrap();
            agent.relations.iter()
                .filter(|(_, r)| r.pending_alliance_proposal)
                .filter_map(|(id, r)| Some(PendingAllyRequestInfo {
                    ally_id: id.clone(),
                    proposer_name: world.agents.get(id).map(|a| a.name.clone()).unwrap_or_default(),
                }))
                .collect()
        };
        
        WorldState {
            // ... 现有字段 ...
            pending_trades,
            pending_ally_requests,
        }
    }
}
```

### 4.4 PerceptionBuilder 修改

**文件**：`crates/core/src/decision/perception.rs`

#### 4.4.1 nearby_agents 输出 ID

```rust
fn build_nearby_agents_section(summary: &mut String, world_state: &WorldState) {
    if !world_state.nearby_agents.is_empty() {
        summary.push_str(&format!("附近 Agent ({} 个):\n", world_state.nearby_agents.len()));
        for agent_info in &world_state.nearby_agents {
            // 修改：输出 ID
            summary.push_str(&format!(
                "  {} [ID:{}] ({},{}) [{}] 距离:{}格 关系:{} 信任:{:.1}\n",
                agent_info.name,
                agent_info.id.as_str(),  // 新增 ID 输出
                agent_info.position.x,
                agent_info.position.y,
                dir_desc,
                agent_info.distance,
                relation_str,
                agent_info.trust,
            ));
        }
    }
}
```

#### 4.4.2 新增 pending_trades 输出

```rust
fn build_pending_trades_section(summary: &mut String, world_state: &WorldState) {
    if !world_state.pending_trades.is_empty() {
        summary.push_str(&format!("待处理交易 ({} 个):\n", world_state.pending_trades.len()));
        for trade in &world_state.pending_trades {
            let offer_str: Vec<String> = trade.offer.iter()
                .map(|(r, n)| format!("{} x{}", r.as_str(), n))
                .collect();
            let want_str: Vec<String> = trade.want.iter()
                .map(|(r, n)| format!("{} x{}", r.as_str(), n))
                .collect();
            summary.push_str(&format!(
                "  [trade_id:{}] {} 提议：用 {} 换你的 {}\n",
                trade.trade_id,
                trade.proposer_name,
                offer_str.join(" + "),
                want_str.join(" + "),
            ));
        }
    }
}
```

#### 4.4.3 新增 pending_ally_requests 输出

```rust
fn build_pending_ally_requests_section(summary: &mut String, world_state: &WorldState) {
    if !world_state.pending_ally_requests.is_empty() {
        summary.push_str(&format!("待处理结盟请求 ({} 个):\n", world_state.pending_ally_requests.len()));
        for request in &world_state.pending_ally_requests {
            summary.push_str(&format!(
                "  [ally_id:{}] {} 请求与你结盟\n",
                request.ally_id.as_str(),
                request.proposer_name,
            ));
        }
    }
}
```

#### 4.4.4 nearby_legacies 输出 ID

```rust
fn build_legacies_section(summary: &mut String, world_state: &WorldState) {
    if !world_state.nearby_legacies.is_empty() {
        summary.push_str(&format!("附近遗迹 ({} 个):\n", world_state.nearby_legacies.len()));
        for legacy in &world_state.nearby_legacies {
            // 修改：输出 legacy_id
            summary.push_str(&format!(
                "  ({}, {}): {:?} [ID:{}] ({}的遗迹, {})\n",
                legacy.position.x, legacy.position.y,
                legacy.legacy_type,
                legacy.legacy_id,  // 新增 ID 输出
                legacy.original_agent_name,
                if legacy.has_items { "有物品" } else { "空" },
            ));
        }
    }
}
```

### 4.5 Explore 删除

**涉及文件及修改**：

| 文件 | 修改内容 |
|------|----------|
| `types.rs` | 删除 `ActionType::Explore` 枚举变体 |
| `parser.rs` | 删除 `parse_explore` 函数和匹配分支 |
| `world/actions/mod.rs` | 删除 `Explore` 匹配分支 |
| `world/actions/movement.rs` | 删除 `handle_explore` 方法 |
| `narrative.rs` | 删除 `EventType::Explore` 枚举变体和相关方法 |
| `decision/spark.rs` | 删除 `SparkType::Explore` 枚举变体 |
| `memory/chronicle_db.rs` | 删除 Explore 相关查询映射 |
| `strategy/retrieve.rs` | 删除 Explore Spark 映射 |
| `strategy/create.rs` | 删除 Explore Spark 映射 |
| `simulation/memory_recorder.rs` | 删除 Explore 记忆标签 |
| `world/feedback.rs` | 删除 Explore 反馈解析 |
| `world/mod.rs` | 删除 Explore 动作权重 |
| `world/legacy.rs` | 删除 `LegacyInteraction::Explore` 枚举变体 |

## 5. 技术决策

### 决策1：pending_ally_requests 数据来源

- **选型方案**：从 Agent.relations 中提取（需新增 `pending_alliance_proposal` 字段）
- **选择理由**：结盟请求是 Agent 间关系，存储在 relations 中符合现有架构
- **备选方案**：World 新增 `pending_ally_requests` Vec（类似 pending_trades）
- **放弃原因**：会增加 World 结构复杂度，而 relations 已有类似概念

**实现方案**：在 `Agent.relations` 的 `RelationInfo` 中新增 `pending_alliance_proposal: bool` 字段，在 AllyPropose 执行时设置为 true。

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
|--------|----------|----------|
| Prompt token 超限 | 中 | 新增动作说明约 300 chars，在 token 预算内（当前 max 2500） |
| Agent.relations 无 pending_alliance_proposal 字段 | 低 | 需新增字段，在 AllyPropose 处理时设置 |
| 删除 Explore 导致存量数据问题 | 低 | Explore 只在运行时使用，无持久化 |

## 7. 迁移方案

### 7.1 部署步骤

1. 编译 bridge crate，复制到 client/bin/
2. 重启 Godot 客户端

### 7.2 回滚方案

无需回滚，变更仅影响 Prompt 和感知输出，不影响持久化数据。

## 8. 待定事项

- [ ] 需确认 Agent.relations 是否需要新增 `pending_alliance_proposal` 字段（或已有类似机制）