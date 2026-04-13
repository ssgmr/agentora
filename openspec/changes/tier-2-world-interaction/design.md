# Tier 2: 世界交互副作用 — 设计文档

## 架构概览

```
┌──────────────────────────────────────────────────────────────────┐
│                        Tier 2: 副作用落实                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Core (Rust)                                                     │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  World::apply_action()  ← 路由层                           │  │
│  │  ├── handle_move()            ← 抽取                       │  │
│  │  ├── handle_gather()          ← 新：调用 ResourceNode      │  │
│  │  ├── handle_wait()            ← 抽取                       │  │
│  │  ├── handle_build()           ← 新：扣资源+创建Structure   │  │
│  │  ├── handle_attack()          ← 新：调用 combat.rs         │  │
│  │  ├── handle_talk()            ← 抽取                       │  │
│  │  ├── handle_explore()         ← 抽取                       │  │
│  │  ├── handle_trade_offer()     ← 新：调用 trade.rs          │  │
│  │  ├── handle_trade_accept()    ← 新：调用 trade.rs          │  │
│  │  ├── handle_ally_propose()    ← 新：调用 alliance.rs       │  │
│  │  ├── handle_ally_accept()     ← 新：调用 alliance.rs       │  │
│  │  ├── handle_ally_reject()     ← 新                         │  │
│  │  └── handle_interact_legacy() ← 抽取                       │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Action struct 扩展（结构化参数）                                 │
│  struct Action {                                                 │
│      action_type: ActionType,                                    │
│      target: Option<AgentId>,        // 目标Agent                │
│      build_type: Option<StructureType>, // Build专用             │
│      direction: Option<Direction>,    // Move专用                │
│  }                                                               │
│                                                                  │
│  RuleEngine 扩展（NPC全套复杂动作）                               │
│  ├── select_target()  ← 新增：基于空间/信任/库存选目标            │
│  └── fallback_decision() ← 扩展：支持Build/Trade/Ally动作        │
│                                                                  │
│  Bridge Delta 扩展                                               │
│  ├── StructureCreated / StructureDestroyed                       │
│  ├── ResourceChanged                                             │
│  ├── TradeCompleted                                              │
│  └── AllianceFormed / AllianceBroken                             │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  Godot Client                                                    │
│  ├── world_renderer.gd  ← 新增：建筑/资源变化渲染                 │
│  ├── narrative_feed.gd  ← 扩展：交易/联盟叙事                     │
│  └── assets/textures/ ← SVG生成PNG贴图                           │
│       ├── structure_storage.png                                  │
│       ├── structure_campfire.png                                 │
│       ├── structure_fortress.png                                 │
│       ├── structure_watchtower.png                               │
│       └── structure_wall.png                                     │
└──────────────────────────────────────────────────────────────────┘
```

## apply_action 重构：路由模式

```rust
fn apply_action(&mut self, agent_id: u64, action: &Action) -> ActionResult {
    // 1. 前置校验
    if !self.agents.contains_key(&agent_id) {
        return ActionResult::InvalidAgent;
    }
    if self.agents[&agent_id].health <= 0 {
        return ActionResult::AgentDead;
    }

    // 2. 路由到具体 handler
    let result = match action.action_type {
        ActionType::Move       => self.handle_move(agent_id, action.direction),
        ActionType::Gather     => self.handle_gather(agent_id),
        ActionType::Wait       => self.handle_wait(agent_id),
        ActionType::Build      => self.handle_build(agent_id, action.build_type),
        ActionType::Attack     => self.handle_attack(agent_id, action.target),
        ActionType::Talk       => self.handle_talk(agent_id, action.target),
        ActionType::Explore    => self.handle_explore(agent_id),
        ActionType::TradeOffer => self.handle_trade_offer(agent_id, action.target),
        ActionType::TradeAccept => self.handle_trade_accept(agent_id, action.target),
        ActionType::TradeReject => self.handle_trade_reject(agent_id, action.target),
        ActionType::AllyPropose => self.handle_ally_propose(agent_id, action.target),
        ActionType::AllyAccept => self.handle_ally_accept(agent_id, action.target),
        ActionType::AllyReject => self.handle_ally_reject(agent_id, action.target),
        ActionType::InteractLegacy => self.handle_interact_legacy(agent_id, action.params),
    };

    // 3. 统一处理结果，生成叙事事件
    match &result {
        ActionResult::Success => { /* 正面叙事已在 handler 内生成 */ }
        ActionResult::Blocked(reason) => {
            self.record_error_narrative(agent_id, &action.action_type, reason);
        }
        _ => {}
    }

    result
}
```

## 错误处理：校验前置 + 错误叙事

```
┌──────────────────────────────────────────────┐
│              apply_action() 流程              │
├──────────────────────────────────────────────┤
│                                              │
│  1. 校验 Agent 存在且存活                    │
│  2. 校验动作参数合法性                       │
│  3. 执行业务逻辑（扣资源、改状态等）          │
│  4. 生成 NarrativeEvent                      │
│                                              │
│  任何步骤失败:                                │
│    → 返回 ActionResult::Blocked(reason)      │
│    → 生成错误叙事 "XXX 尝试 YYY 失败: 原因"  │
│    → 不修改世界状态                          │
│                                              │
└──────────────────────────────────────────────┘
```

每个 handler 内部也遵循同样的模式：先校验所有前置条件，全部通过后再修改状态。

```rust
fn handle_build(&mut self, agent_id: u64, build_type: Option<StructureType>) -> ActionResult {
    let build_type = match build_type {
        Some(t) => t,
        None => return ActionResult::Blocked("Build 缺少 build_type 参数".into()),
    };

    let agent = self.agents.get_mut(&agent_id).unwrap();
    let cost = build_type.resource_cost();

    // 校验前置：资源足够
    if !agent.inventory.has_resources(&cost) {
        return ActionResult::Blocked(format!("资源不足，需要{:?}，实际有{:?}", cost, agent.inventory));
    }

    // 校验前置：位置无建筑
    if self.structures.contains_key(&agent.position) {
        return ActionResult::Blocked("目标位置已有建筑".into());
    }

    // 所有校验通过，执行修改
    agent.inventory.deduct(&cost);
    let structure = Structure::new(build_type, agent.position, Some(agent_id));
    self.structures.insert(agent.position, structure);

    ActionResult::Success
}
```

## RuleEngine NPC 扩展

### select_target() 辅助方法

```
NPC 选目标策略:
  Attack  → 最近的其他 Agent / HP 最低的
  Build   → 基于动机类型（生存→Storage，社交→Campfire，权力→Fortress）
  Ally    → 同阵营 / 高信任度 / 最近的
  Trade   → 库存互补的（我多食物→换木材）
```

基于 `WorldState` 的 `nearby_agents` 和 `relations` 做简单选择，不需要 AI 级别的智能。

### fallback_decision() 扩展

```
当前:
  生存动机高 → Move / Gather / Wait
  社交动机高 → Talk
  权力动机高 → Attack

Tier 2 扩展:
  生存动机高 → Move / Gather / Wait / Build(Storage)
  社交动机高 → Talk / AllyPropose / TradeOffer
  认知动机高 → Explore
  表达动机高 → Build(Campfire)
  权力动机高 → Attack / AllyPropose / Build(Fortress)
  传承动机高 → InteractLegacy
```

## Bridge Delta 扩展

### 新增 Delta 事件

```rust
// crates/core/src/snapshot.rs

pub enum WorldDelta {
    // 已有
    AgentMoved { id: u64, position: Position },
    AgentDied { id: u64 },
    AgentSpawned { id: u64, position: Position },

    // 新增
    StructureCreated { position: Position, structure_type: String, owner_id: Option<u64> },
    StructureDestroyed { position: Position, structure_type: String },
    ResourceChanged { position: Position, resource_type: String, amount: u32 },
    TradeCompleted { from_id: u64, to_id: u64, items: String },
    AllianceFormed { id1: u64, id2: u64 },
    AllianceBroken { id1: u64, id2: u64, reason: String },
}
```

### Godot 端渲染映射

```
Delta 事件          →  Godot 渲染行为
─────────────────────────────────────────────
StructureCreated    →  在 TileMap 对应位置放置建筑 sprite
StructureDestroyed  →  移除建筑 sprite + 废墟特效
ResourceChanged     →  更新资源节点视觉（数量减少→图标变小/变暗）
TradeCompleted      →  叙事流显示交易信息
AllianceFormed      →  叙事流显示结盟 + 可能连线可视化
AllianceBroken      →  叙事流显示决裂
```

## Godot 贴图资源

暂时无美术资源，使用 SVG 生成简单 placeholder 图标，转换为 PNG 放入 `client/assets/textures/`。

```
SVG 设计思路:
  Storage     → 棕色方块 + 屋顶形状
  Campfire    → 橙色火焰 + 木柴
  Fortress    → 灰色城墙 + 垛口
  WatchTower  → 高塔形状 + 瞭望台
  Wall        → 灰色砖墙

尺寸: 32x32 或 64x64 像素
```

## 文件变更清单

| 文件 | 变更类型 | 内容 |
|------|----------|------|
| `crates/core/src/types.rs` | 修改 | Action struct 增加 `build_type`, `direction` 字段 |
| `crates/core/src/decision.rs` | 修改 | parse_action_type 停止映射 Trade/Ally 到 Wait；解析新参数 |
| `crates/core/src/world/mod.rs` | 重构 | apply_action() 改为路由；抽取/新增所有 handler |
| `crates/core/src/rule_engine.rs` | 扩展 | select_target() + fallback_decision() 支持全套动作 |
| `crates/core/src/snapshot.rs` | 扩展 | 新增 WorldDelta 枚举变体 |
| `crates/bridge/src/lib.rs` | 扩展 | run_apply_loop 推送新 Delta 事件 |
| `client/assets/textures/` | 新增 | SVG→PNG 建筑贴图 |
| `client/scripts/world_renderer.gd` | 扩展 | 建筑/资源渲染逻辑 |
| `client/scripts/narrative_feed.gd` | 扩展 | 新叙事类型显示 |
