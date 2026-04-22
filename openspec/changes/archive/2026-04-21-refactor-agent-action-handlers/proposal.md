# 提案：重构 Agent Action Handlers 职责边界

## 背景概述

当前 `world/actions.rs` 的动作处理器存在职责边界模糊问题：

1. **World 直接操作 Agent 内部属性**
   - `handle_attack` 直接修改 `agent.health`、`agent.relations`
   - `handle_trade_accept` 虽调用 `gather/consume`，但协调逻辑在 World
   - `handle_move_toward` 直接修改 `agent.position`

2. **Agent 模块方法未被调用**
   - `agent/combat.rs::attack()` 定义了完整的攻击逻辑，但 World handler 未使用
   - `agent/trade.rs::accept_trade()` 定义了交易协议，但 World handler 自行实现交换逻辑
   - 存在"孤岛代码"，职责混乱

3. **Rust 借用规则的误解**
   - 现有结论是"因借用规则限制，保持现状"
   - 实际可通过分离方法（`receive_attack` + `initiate_attack`）解决
   - 每个方法只修改一个 Agent，符合借用规则

## 变更目标

建立清晰的职责分离：

- **World 职责**：世界级校验、协调执行、叙事记录、世界状态更新
- **Agent 职责**：所有自身状态变更通过方法调用

**核心原则**：World 不直接修改 Agent 内部属性，只调用 Agent 方法

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `agent-movement` | 新增 `agent/movement.rs`：`move_to(target)` 方法 |
| `agent-survival` | 新增 `agent/survival.rs`：`eat_food()`, `drink_water()` 方法 |
| `agent-combat-refactor` | 重构 `agent/combat.rs`：`receive_attack()`, `initiate_attack()` |
| `agent-trade-refactor` | 重构 `agent/trade.rs`：`freeze_resources()`, `unfreeze_*()`，新增 `frozen_inventory` 字段 |
| `agent-social` | 新增 `agent/social.rs`：`talk_with()` 方法 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `world-actions-refactor` | 所有 handler 改为调用 Agent 方法，不直接操作属性 |
| `agent-mod-struct` | Agent 结构体新增 `frozen_inventory`、`pending_trade_id` 字段 |

### 删除功能

| 功能标识 | 删除内容 |
| --- | --- |
| `agent-trade-old` | 删除 `accept_trade()` 方法（签名不适合 World 协调） |
| `agent-combat-old` | 删除原有 `attack(target)` 方法（签名有借用问题） |

### 保持不变

| 功能标识 | 说明 |
| --- | --- |
| `agent-inventory` | `gather()`, `consume()` 保持现有实现 |
| `agent-alliance` | 所有 alliance 方法保持不变（已正确调用） |
| `world-validation` | World 的校验逻辑保持不变 |
| `world-narrative` | 叙事生成逻辑保持不变 |

## 影响范围

- **代码模块**：
  - `crates/core/src/agent/mod.rs` — 新增字段
  - `crates/core/src/agent/movement.rs` — **新建**
  - `crates/core/src/agent/survival.rs` — **新建**
  - `crates/core/src/agent/combat.rs` — 重构方法签名
  - `crates/core/src/agent/trade.rs` — 重构方法签名
  - `crates/core/src/agent/social.rs` — **新建**
  - `crates/core/src/world/actions.rs` — 重构所有 handler

- **API 接口**：Agent 公开方法签名变更
- **依赖组件**：无变化
- **关联系统**：DecisionPipeline、RuleEngine 调用路径不变

## 验收标准

- [ ] `world/actions.rs` 无直接操作 Agent 内部属性的代码
- [ ] 所有 Agent 状态变更通过方法调用完成
- [ ] Agent 模块方法签名符合 Rust 借用规则（每个方法只修改一个 Agent）
- [ ] 交易资源冻结机制正常工作
- [ ] 交易超时自动解冻机制正常工作（超过配置 tick 数自动取消）
- [ ] `cargo build` 编译通过
- [ ] `cargo test` 全部通过
- [ ] 客户端运行正常，Agent 决策、交易、攻击功能正确

## 风险

| 风险 | 缓解措施 |
| --- | --- |
| 重构破坏现有功能 | 每个 handler 重构后单独测试 |
| 借用检查器问题 | 方法设计遵循"只修改自己"原则 |
| 交易冻结逻辑复杂 | 参照现有 World handler 实现效果 |
| 交易超时配置不当 | 默认值保守（50 tick），可通过配置调整 |