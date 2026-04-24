# Tier 3: 涌现催化剂 — 策略与压力系统

## 问题

当前系统有完整的策略学习和压力事件框架，但存在两处集成缺口：

1. **策略系统悬空** — `DecisionPipeline` 内部已实现策略检索和注入逻辑（`build_prompt` 调用 `retrieve_strategy`，`prompt.rs` 有 `<strategy-context>` 模板），但 `Simulation::new()` 创建 `DecisionPipeline` 时未调用 `.with_strategy_hub()`，`strategy_hub` 字段始终为 `None`，检索结果永远为空
2. **策略创建 SparkType 硬编码** — `world/mod.rs` 中策略创建使用写死的 `SparkType::CognitivePressure`（早期是 `Explore`，后改为 `CognitivePressure`），但策略检索使用 `infer_state_mode(world_state)` 动态推断（`ResourcePressure` / `SocialPressure` / `Explore`），**创建和检索的类型体系不匹配**，导致策略几乎找不到自己创建的条目

> **已修复（本变更之前）：**
> - ~~压力系统休眠~~ — `pressure_tick()` 已在 `advance_tick()` 和 `advance_tick_local_only()` 中激活，生成 drought/abundance/plague 事件
> - ~~长期记忆未注入~~ — `agent_loop.rs` 已在决策前调用 `agent.memory.get_summary(spark_type)`，三层记忆（ChronicleStore + ChronicleDB + ShortTermMemory）已注入 Prompt

## 目标

打通策略系统的最后两段集成缺口，让 Agent 的经验学习真正生效：

- `DecisionPipeline` 能检索到已有策略并注入 Prompt
- 策略创建时使用与实际决策情境匹配的 SparkType，使检索能找到对应策略
- 形成 "创建策略 → 检索策略 → 参考策略" 的闭环

## 范围

**包含：**
- 在 Bridge 的 `Simulation::new()` 中构造 `StrategyHub` 并通过 `.with_strategy_hub()` 注入 `DecisionPipeline`
- 修复策略创建的 SparkType 硬编码：创建时传入实际决策情境的 SparkType（从 `infer_state_mode` 或 `ActionCandidate` 映射）
- 验证策略检索结果在 Prompt 中的展示效果

**不包含：**
- 复杂压力事件类型（区域封锁、自然灾害，未来增强）
- 策略的跨 Agent 传播（文化传承，未来增强）
- Merkle/签名验证（安全增强）

## 影响

- `crates/bridge/src/simulation_runner.rs` — 构造 `StrategyHub` 并传入 `Simulation::new()`
- `crates/core/src/simulation/simulation.rs` — `DecisionPipeline` builder 增加 `.with_strategy_hub()` 调用
- `crates/core/src/world/mod.rs` — `apply_action()` 中策略创建使用动态 SparkType
- `crates/core/src/strategy/create.rs` — 可能需要增加从 `ActionType` 到 `SparkType` 的映射
- `crates/core/src/decision/mod.rs` — 确认 `infer_state_mode` 可复用或导出
