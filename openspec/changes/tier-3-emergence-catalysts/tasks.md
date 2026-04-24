# 实施任务清单

## 1. StrategyHub 注入链路

为 Simulation 创建 StrategyHub 集合，并通过参数传入 DecisionPipeline，使策略检索在决策 Prompt 中生效。

- [x] 1.1 Simulation 结构体增加 `strategy_hubs: HashMap<AgentId, StrategyHub>` 字段
  - 文件: `crates/core/src/simulation/simulation.rs`
  - 在 `Simulation::new()` 中为每个初始 Agent 创建 StrategyHub，调用 `load_all_strategies()` 加载已有策略
  - 将 pipeline 从 `Arc<DecisionPipeline>` 改为不绑定 StrategyHub（移除 self.strategy_hub 字段的使用）

- [x] 1.2 DecisionPipeline `execute()` 和 `build_prompt()` 增加 `strategy_hub: Option<&StrategyHub>` 参数
  - 文件: `crates/core/src/decision/mod.rs`
  - `execute()` 签名增加 `strategy_hub: Option<&StrategyHub>` 参数
  - `build_prompt()` 签名增加 `strategy_hub: Option<&StrategyHub>` 参数
  - `build_prompt()` 中使用传入的 `strategy_hub` 替代 `self.strategy_hub`
  - 移除 `self.strategy_hub` 字段和 `with_strategy_hub()` builder 方法（不再需要）

- [x] 1.3 `agent_loop` 中传入 `strategy_hub` 和 `spark_type`
  - 文件: `crates/core/src/simulation/agent_loop.rs`
  - `run_agent_loop()` 新增 `strategy_hub: StrategyHub` 参数（每个 Agent 一个）
  - 调用 `pipeline.execute()` 时传入 `Some(&strategy_hub)`
  - 将 `infer_state_mode(&ws)` 的结果保存，后续传递给 `apply_action`

- [x] 1.4 `run_agent_loop` 调用方传入 strategy_hub
  - 文件: `crates/core/src/simulation/simulation.rs` — Agent 启动代码
  - 从 `self.strategy_hubs` 中取出对应 Agent 的 hub，传入 `run_agent_loop`

## 2. 策略创建 SparkType 动态化

策略创建时使用与实际决策情境匹配的 SparkType，替换硬编码。

- [x] 2.1 `apply_action()` 增加 `spark_type: Option<SparkType>` 参数
  - 文件: `crates/core/src/world/mod.rs`
  - `apply_action()` 签名增加 `spark_type: Option<SparkType>` 参数
  - 策略创建处（line ~313）改为：`let spark_type = spark_type.unwrap_or_else(|| infer_state_mode_from_agent(...));`
  - 需导入 `infer_state_mode`（或提供等效的从 World/Agent 推断的函数）

- [x] 2.2 更新 `apply_action()` 所有调用点
  - 文件: `crates/core/src/world/actions.rs`（如通过 action dispatcher 调用）
  - 文件: `crates/core/src/simulation/agent_loop.rs` — 传入 `Some(spark_type)`
  - 其他调用点传入 `None`（使用 fallback）

## 3. 测试与验证

- [x] 3.1 编译验证
  - 运行 `cargo build` 确认无编译错误
  - 运行 `cargo build -p agentora-bridge` 确认 bridge crate 编译通过

- [x] 3.2 单元测试
  - 运行 `cargo test` 确认所有测试通过
  - 重点关注 `strategy_tests`、`decision_tests`

- [x] 3.3 验收测试
  - 运行模拟（`cargo run` 或 bridge 方式），观察日志中策略检索是否生效
  - 验证 Prompt 中包含 `<strategy-context>` 段（通过 debug 日志）
  - 验证策略创建后，下次同类情境决策时能检索到该策略

## 任务依赖关系

```
1.1 (Simulation strategy_hubs)
  │
  ├──▶ 1.2 (DecisionPipeline execute 参数) ──▶ 1.3 (agent_loop 传入) ──▶ 1.4 (调用方)
  │
  └──▶ 2.1 (apply_action spark_type) ──▶ 2.2 (更新调用点)
                                                │
                                         ◀──────┘ (agent_loop 中同时传入两者)
                                                │
                                               3.x (测试)
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
|------|------|------|
| 阶段一 | 1.1, 1.2 | 基础架构：Simulation 存 hub，DecisionPipeline 收参数 |
| 阶段二 | 1.3, 1.4, 2.1, 2.2 | 接线：agent_loop 连通策略检索和动态创建 |
| 阶段三 | 3.1, 3.2, 3.3 | 验证：编译、测试、验收 |

## 文件结构总览

```
crates/core/src/
├── simulation/
│   ├── simulation.rs   ← 新增 strategy_hubs 字段，修改创建逻辑
│   └── agent_loop.rs   ← 传入 strategy_hub 和 spark_type
├── decision/
│   └── mod.rs          ← execute/build_prompt 增加 strategy_hub 参数
└── world/
    └── mod.rs          ← apply_action 增加 spark_type 参数
```
