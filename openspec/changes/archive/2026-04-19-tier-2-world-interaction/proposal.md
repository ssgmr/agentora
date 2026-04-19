# Tier 2: 世界交互副作用落实

## 问题

当前 Agent 的行动对世界的改变大部分是"只记叙事，不产生副作用"：

1. **Build 假建** — `handle_special_action` 中 Build 只记录一条叙事文本，不创建 Structure 到 `world.structures`，不扣除资源
2. **Gather 无限采** — 库存 +1 但世界 `ResourceNode` 不减量，资源永不枯竭
3. **Trade 降级为 Wait** — LLM 解析时将 TradeOffer 直接映射为 Wait，`handle_special_action` 只记叙事，不执行库存交换
4. **Ally 降级为 Wait** — 同上，不修改关系状态
5. **Combat 双实现** — `combat.rs` 有完整逻辑但 `handle_special_action` 用硬编码的 -10 HP，两套实现不一致
6. **NPC 能力受限** — RuleEngine 的 `fallback_decision()` 只支持基础动作，NPC 无法执行 Build/Trade/Ally 等复杂动作

## 目标

让 Agent 的每一个行动都真实改变世界状态，形成"行动→世界变化→感知变化→新决策"的闭环：

- 建造真实创建建筑并消耗资源
- 采集真实扣除资源节点，节点会枯竭
- 交易真实转移库存
- 结盟真实修改关系
- 战斗统一到 `combat.rs` 实现
- NPC 通过 RuleEngine 也能执行全套复杂动作

## 范围

**包含：**
- `Action` struct 扩展结构化参数（`build_type`, `direction` 字段）
- `Build` 动作：检查资源消耗 → 扣除资源 → 创建 Structure 插入 `world.structures`
- `Gather` 动作：调用 `ResourceNode.gather()` 扣除实际资源量
- `TradeOffer/TradeAccept` 从 Wait 降级中解放：解析为真实动作 → 验证双方库存 → 执行交换
- `AllyPropose/AllyAccept` 从 Wait 降级中解放：调用 `alliance.rs` 真实修改关系
- `Attack` 动作：统一到 `combat.rs` 的 `attack()` 方法
- `RuleEngine` 扩展：`select_target()` 辅助方法 + `fallback_decision()` 支持全套复杂动作
- `World::apply_action()` 重构：拆分为独立 handler 方法，纯路由模式
- 错误处理：校验前置，失败返回 `ActionResult::Blocked(reason)` + 错误叙事
- Bridge Delta 扩展：`StructureCreated/Destroyed`、`ResourceChanged`、`TradeCompleted`、`AllianceFormed/Broken`
- Godot 客户端：Structure 创建/资源变化实时渲染，新增 SVG→PNG 建筑贴图

**不包含：**
- 建筑放置碰撞检测（未来增强）
- 建筑耐久/破坏系统（未来增强）
- 复杂交易协议（多物品分批确认，未来增强）
- 压力系统激活（Tier 3）

## 关键设计决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| Action 参数传递 | 结构化字段 | 类型安全，避免运行时 JSON 解析 |
| 错误处理 | 校验前置 + 错误叙事 | 不需要事务回滚，Agent 能"记住"失败 |
| NPC 能力范围 | 全套复杂动作 | NPC 和 LLM Agent 能力保持一致 |
| handler 组织 | 独立方法 | 可独立测试，`apply_action()` 纯路由 |
| Godot 贴图 | SVG→PNG placeholder | 暂无美术资源，先用程序化生成 |
| Structure 渲染 | Delta 实时推送 | 建筑创建需要即时在地图上显示 |

## 影响

- `crates/core/src/types.rs` — Action struct 增加 `build_type`、`direction` 字段
- `crates/core/src/decision.rs` — parse_action 不再将 Trade/Ally 降级为 Wait；解析新参数
- `crates/core/src/world/mod.rs` — apply_action 重构为路由，拆分/新增所有 handler
- `crates/core/src/rule_engine.rs` — 新增 select_target()，扩展 fallback_decision()
- `crates/core/src/snapshot.rs` — 新增 Delta 事件类型
- `crates/bridge/src/lib.rs` — apply_loop 增加新 Delta 推送
- `client/assets/textures/` — 新增 SVG→PNG 建筑贴图
- `client/scripts/world_renderer.gd` — 建筑/资源渲染逻辑
- `client/scripts/narrative_feed.gd` — 新叙事类型显示
