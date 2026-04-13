# Tier 2: 世界交互副作用 — 任务清单

## Phase 1: Core 基础重构

- [x] 1.1 扩展 `Action` struct（`types.rs`）增加 `build_type: Option<StructureType>`, `direction: Option<Direction>` 字段
- [x] 1.2 更新 `decision.rs` 的 `parse_action_type()` 停止将 TradeOffer/AllyPropose 映射为 Wait，解析新参数
- [x] 1.3 重构 `World::apply_action()` 为路由模式，抽取现有内联逻辑到独立 handler（handle_move, handle_wait, handle_talk, handle_explore, handle_interact_legacy）
- [x] 1.4 新增 `handle_gather()` 调用 `ResourceNode.gather()` 真实扣除资源
- [x] 1.5 新增 `handle_build()` 校验资源 → 扣除 → 创建 Structure
- [x] 1.6 新增 `handle_attack()` 统一到 combat.rs 逻辑
- [x] 1.7 新增 `handle_trade_offer()` / `handle_trade_accept()` / `handle_trade_reject()` 调用 `trade.rs`
- [x] 1.8 新增 `handle_ally_propose()` / `handle_ally_accept()` / `handle_ally_reject()` 调用 `alliance.rs`
- [x] 1.9 统一错误处理：`ActionResult::Blocked(reason)` → 生成错误叙事

## Phase 2: RuleEngine NPC 扩展

- [x] 2.1 新增 `select_target()` 辅助方法（基于空间/信任/库存选目标）
- [x] 2.2 扩展 `fallback_decision()` 支持全套复杂动作（Build/Trade/Ally）
- [x] 2.3 新增 Build 类型选择逻辑（动机 → 建筑类型映射）
- [x] 2.4 为 NPC 决策添加动作前置校验（资源/目标/位置）

## Phase 3: Bridge Delta 扩展

- [x] 3.1 扩展 `snapshot.rs` 新增 WorldDelta 枚举变体（StructureCreated/Destroyed, ResourceChanged, TradeCompleted, AllianceFormed/Broken）
- [x] 3.2 更新 `bridge/src/lib.rs` 的 `run_apply_loop()` 推送新 Delta 事件
- [x] 3.3 确保 Delta 序列化与 Godot 兼容

## Phase 4: Godot 客户端

- [ ] 4.1 生成 SVG 建筑贴图 → PNG（storage/campfire/fortress/watchtower/wall）
- [ ] 4.2 扩展 `world_renderer.gd` 渲染 Structure 创建/销毁
- [ ] 4.3 扩展 `world_renderer.gd` 更新 Resource 视觉表现
- [ ] 4.4 扩展 `narrative_feed.gd` 显示交易/联盟叙事
- [ ] 4.5 注册新贴图资源到 Godot 项目

## Phase 5: 测试

- [x] 5.1 为每个新 handler 编写单元测试
- [x] 5.2 测试 NPC RuleEngine 全套动作决策
- [x] 5.3 测试错误叙事生成
- [x] 5.4 运行 `cargo test` 确保无回归（42 个新测试通过；`test_decay_convergence` 为预先存在的衰减收敛测试失败，与 Tier 2 无关）
