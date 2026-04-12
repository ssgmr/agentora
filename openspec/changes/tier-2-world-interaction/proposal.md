# Tier 2: 世界交互副作用落实

## 问题

当前 Agent 的行动对世界的改变大部分是"只记叙事，不产生副作用"：

1. **Build 假建** — `handle_special_action` 中 Build 只记录一条叙事文本，不创建 Structure 到 `world.structures`，不扣除资源
2. **Gather 无限采** — 库存 +1 但世界 `ResourceNode` 不减量，资源永不枯竭
3. **Trade 降级为 Wait** — LLM 解析时将 TradeOffer 直接映射为 Wait，`handle_special_action` 只记叙事，不执行库存交换
4. **Ally 降级为 Wait** — 同上，不修改关系状态
5. **Combat 双实现** — `combat.rs` 有完整逻辑但 `handle_special_action` 用硬编码的 -10 HP，两套实现不一致

## 目标

让 Agent 的每一个行动都真实改变世界状态，形成"行动→世界变化→感知变化→新决策"的闭环：

- 建造真实创建建筑并消耗资源
- 采集真实扣除资源节点，节点会枯竭
- 交易真实转移库存
- 结盟真实修改关系
- 战斗统一到 `combat.rs` 实现

## 范围

**包含：**
- `Build` 动作：检查资源消耗 → 扣除资源 → 创建 Structure 插入 `world.structures`
- `Gather` 动作：调用 `ResourceNode.gather()` 扣除实际资源量
- `TradeOffer/TradeAccept` 从 Wait 降级中解放：解析为真实动作 → 验证双方库存 → 执行交换
- `AllyPropose/AllyAccept` 从 Wait 降级中解放：调用 `alliance.rs` 真实修改关系
- `Attack` 动作：统一到 `combat.rs` 的 `attack()` 方法
- Godot 客户端接收 Structure 创建/资源变化的 Delta 事件

**不包含：**
- 建筑放置碰撞检测（未来增强）
- 建筑耐久/破坏系统（未来增强）
- 复杂交易协议（多物品分批确认，未来增强）
- 压力系统激活（Tier 3）

## 影响

- `crates/core/src/world/mod.rs` — handle_special_action 重写 Build/Gather/Trade/Ally/Attack 分支
- `crates/core/src/decision.rs` — parse_action_type 不再将 Trade/Ally 降级为 Wait
- `crates/bridge/src/lib.rs` — apply_loop 增加 Structure/Resource Delta 推送
- `crates/core/src/snapshot.rs` — WorldSnapshot 增加资源变化事件
- `crates/core/src/rule_engine.rs` — Build 资源消耗校验（已有，需确认与 world 一致）
- `client/scripts/` — Godot 端渲染 Structure 和资源节点变化
