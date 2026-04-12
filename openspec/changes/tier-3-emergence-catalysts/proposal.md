# Tier 3: 涌现催化剂 — 策略与压力系统

## 问题

当前系统有完整的策略学习和压力事件框架，但未接入决策循环：

1. **策略系统悬空** — `StrategyHub` 有创建/检索/衰减/动机联动的完整实现，但 `DecisionPipeline` 创建时 `strategy_hub=None`，永远不检索策略
2. **策略创建硬编码** — `world/mod.rs` 中策略创建时 SparkType 硬编码为 `Explore`，不反映实际决策类型
3. **压力系统休眠** — `pressure_tick()` 是 TODO，世界永远不会生成动态事件（资源波动、气候事件、区域封锁）
4. **长期记忆未注入** — `ChronicleStore` 的 Markdown 摘要可以注入 Prompt 提供文明传承感，但当前决策 Prompt 中 memory_summary 恒为空字符串

## 目标

为 Agent 引入经验学习和世界动态变化能力，让文明从"个体反应"走向"集体涌现"：

- Agent 能从成功经验中提炼策略并在后续决策中参考
- 策略成功/失败反馈到动机向量形成强化循环
- 世界定期生成压力事件，给 Agent 提供环境 Spark
- 长期记忆摘要进入 Prompt，Agent 有文明传承感

## 范围

**包含：**
- 在 Bridge 的 `DecisionPipeline` 创建时注入 `StrategyHub`
- 修复策略创建的 SparkType 硬编码问题，使用实际决策类型
- 激活 `pressure_tick()` 生成压力事件（资源波动、气候事件）
- 在决策 Prompt 中注入 Chronicle 摘要（最近 3 条冻结快照）
- 策略检索结果注入 Prompt（"上次采集策略成功率 85%"）

**不包含：**
- 复杂压力事件类型（区域封锁、自然灾害，未来增强）
- 策略的跨 Agent 传播（文化传承，未来增强）
- Merkle/签名验证（安全增强）

## 影响

- `crates/bridge/src/lib.rs` — DecisionPipeline 注入 StrategyHub
- `crates/core/src/world/mod.rs` — 修复策略创建 SparkType、激活 pressure_tick
- `crates/core/src/strategy/retrieve.rs` — 确保检索结果可格式化注入 Prompt
- `crates/core/src/decision.rs` — build_prompt 中注入策略和长期记忆
- `crates/core/src/prompt.rs` — Prompt 模板增加策略和长期记忆段
