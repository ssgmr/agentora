# 任务清单：重构 Agent Action Handlers 职责边界

## Phase 1: Agent 结构体扩展

### Task 1.1: 新增 Agent 字段
- [x] 在 `agent/mod.rs` 新增 `frozen_inventory: HashMap<String, u32>`
- [x] 在 `agent/mod.rs` 新增 `pending_trade_id: Option<String>`
- [x] 在 `Agent::new()` 中初始化新字段

**验证**：`cargo check` 通过 ✓

---

## Phase 2: 新建 Agent 子模块

### Task 2.1: 创建 agent/movement.rs
- [x] 创建文件 `crates/core/src/agent/movement.rs`
- [x] 实现 `move_to(target: Position) -> (bool, Position, Position)`
- [x] 在 `agent/mod.rs` 添加 `pub mod movement;`

### Task 2.2: 创建 agent/survival.rs
- [x] 创建文件 `crates/core/src/agent/survival.rs`
- [x] 实现 `eat_food() -> (bool, u32, u32, u32, u32)` 返回 `(成功, 变化量, 前值, 后值, 剩余)`
- [x] 实现 `drink_water() -> (bool, u32, u32, u32, u32)` 返回 `(成功, 变化量, 前值, 后值, 剩余)`
- [x] 在 `agent/mod.rs` 添加 `pub mod survival;`

### Task 2.3: 创建 agent/social.rs
- [x] 创建文件 `crates/core/src/agent/social.rs`
- [x] 实现 `talk_with(nearby_ids, message, tick)`
- [x] 实现 `receive_talk(speaker_id, speaker_name, message, tick)`
- [x] 在 `agent/mod.rs` 添加 `pub mod social;`

**验证**：`cargo check` 通过 ✓

---

## Phase 3: 重构现有 Agent 模块

### Task 3.1: 重构 agent/combat.rs
- [x] 删除原有 `attack(&mut target, damage)` 方法
- [x] 新增 `receive_attack(damage: u32, attacker_id: &AgentId)`
- [x] 新增 `initiate_attack(target_id: &AgentId)`
- [x] 删除 `AttackResult` 结构体（World自行判断target_alive）

### Task 3.2: 重构 agent/trade.rs
- [x] 删除原有 `propose_trade()` 方法（World自行创建PendingTrade）
- [x] 删除原有 `accept_trade()` 方法（签名不适合协调）
- [x] 新增 `freeze_resources(offer, trade_id) -> bool`
- [x] 新增 `complete_trade_send(offer, want)`
- [x] 新增 `cancel_trade(offer)`
- [x] 新增 `give_resources(want) -> bool`
- [x] 新增 `receive_resources(offer)`
- [x] 保留 `TradeOffer` 结构体定义（供 World 使用）

**验证**：`cargo check` 通过 ✓

---

## Phase 4: 重构 World Handlers

### Task 4.1: 重构 handle_move_toward
- [x] World 调用 `agent.move_to(target)` 替代直接改 `agent.position`
- [x] 保留边界检查、相邻校验、Fence碰撞检查（World职责）
- [x] 返回值使用 Agent 方法的返回值

### Task 4.2: 重构 handle_eat
- [x] World 调用 `agent.eat_food()` 替代直接改 satiety 和 inventory
- [x] 保留叙事记录（World职责）
- [x] 使用 Agent 方法的返回值生成反馈

### Task 4.3: 重构 handle_drink
- [x] World 调用 `agent.drink_water()` 替代直接改 hydration 和 inventory
- [x] 保留叙事记录（World职责）

### Task 4.4: 重构 handle_attack
- [x] World 计算 damage（base_damage * multiplier）
- [x] 分段借用：先调用 `target.receive_attack(damage, attacker_id)`
- [x] 再调用 `attacker.initiate_attack(target_id)`
- [x] World 维护 `total_attacks` 统计
- [x] 保留距离校验、盟友检查（World职责）

### Task 4.5: 重构 handle_talk
- [x] World 查找附近 Agent
- [x] 循环调用每个 nearby 的 `receive_talk()`
- [x] 调用发起方的 `talk_with()`
- [x] 保留叙事记录（World职责）

### Task 4.6: 重构 handle_trade_offer
- [x] World 校验发起方资源足够（调用 `freeze_resources` 前检查）
- [x] World 创建 PendingTrade（包含 trade_id）
- [x] 调用 `proposer.freeze_resources(offer, trade_id)`
- [x] 添加到 `pending_trades` 队列
- [x] 保留目标存在性校验（World职责）

### Task 4.7: 重构 handle_trade_accept
- [x] World 查找 pending_trade
- [x] World 校验双方资源足够
- [x] 分段借用：
  - `acceptor.give_resources(want)`
  - `acceptor.receive_resources(offer)`
  - `proposer.complete_trade_send(offer, want)`
- [x] 移除 pending_trade，更新统计
- [x] 保留叙事记录（World职责）

### Task 4.8: 重构 handle_trade_reject
- [x] World 查找 pending_trade
- [x] 调用 `proposer.cancel_trade(offer)`
- [x] 移除 pending_trade
- [x] 保留叙事记录（World职责）

### Task 4.9: 新增交易超时检查（tick_loop）
- [x] 在 `world/tick.rs` 或 `world/mod.rs` 添加超时检查逻辑
- [x] 每tick遍历 `pending_trades`，检查 `tick - tick_created > trade_timeout_ticks`
- [x] 调用 `proposer.cancel_trade(offer)` 解冻资源
- [x] 移除超时交易，记录超时事件
- [x] 配置参数：`config/sim.toml` 添加 `trade_timeout_ticks = 50`（World.trade_timeout_ticks 字段）

### Task 4.10: handle_explore 复用 move_to
- [x] Explore 的移动逻辑改为单步，复用 `agent.move_to()`
- [x] 保留随机方向计算（World职责）

**验证**：`cargo build` 通过 ✓

---

## Phase 5: 验证与测试

### Task 5.1: 单元测试验证
- [x] 运行 `cargo test` 全部测试通过
- [x] 检查现有测试是否需要更新（agent 方法签名变更）
- [x] 为新增 Agent 方法编写单元测试（可选）

### Task 5.2: 编译 bridge
- [x] 运行 `cargo build -p agentora-bridge`
- [x] 复制 dll 到 `client/bin/`

### Task 5.3: 客户端集成测试
- [x] 启动 Godot 客户端
- [x] 观察 Agent 决策循环正常
- [x] 验证 Talk 功能：叙事记录显示「Agent_2 与 [NPC]Explorer 交流：「你好」」
- [x] 验证 Eat 功能：叙事记录显示「进食，恢复饱食度 (+30)」
- [x] 验证 Drink 功能：叙事记录显示「饮水，恢复水分度 (+25)」
- [x] 验证 MoveToward/Gather/Explore 功能正常
- [x] Attack/Trade 验证：单元测试已覆盖，客户端运行无错误

### Task 5.4: 更新文档
- [x] 更新 `CLAUDE.md` 中 Agent 模块的描述
- [x] 在新增文件添加模块文档注释（movement.rs/survival.rs/social.rs 已有）
- [x] 更新 combat.rs/trade.rs 文档注释以反映新方法签名

---

## 总结

| Phase | 任务数 | 完成数 | 验证方式 |
|-------|-------|--------|----------|
| 1. Agent结构体扩展 | 1个 | 1个 | cargo check ✓ |
| 2. 新建子模块 | 3个 | 3个 | cargo check ✓ |
| 3. 重构现有模块 | 2个 | 2个 | cargo check ✓ |
| 4. 重构World Handlers | 10个 | 10个 | cargo build ✓ |
| 5. 验证测试 | 4个 | 4个 | cargo test ✓ + 客户端已验证 + 文档已更新 |

**总计**：5个阶段，20个任务，全部完成 ✓