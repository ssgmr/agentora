# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

Agentora 使用 DecisionPipeline 作为 Agent 决策核心，通过 PromptBuilder 组装包含感知摘要、记忆摘要、策略参考的 Prompt 给 LLM 决策。策略系统（StrategyHub）提供了完整的策略创建/检索/衰减框架，使用 Markdown + YAML frontmatter 格式存储。

当前架构中：
- `DecisionPipeline` 已有 `strategy_hub: Option<StrategyHub>` 字段和 `.with_strategy_hub()` builder 方法
- `build_prompt()` 中已有策略检索逻辑：`infer_state_mode(world_state)` → `retrieve_strategy()` → `wrap_strategy_for_prompt()`
- `agent_loop.rs` 已注入三层记忆摘要（ChronicleStore + ChronicleDB + ShortTermMemory）
- 压力系统已激活（`pressure_tick()` 在 `advance_tick()` 中调用）

### 1.2 现状分析

存在两个集成缺口：

**缺口 1：StrategyHub 未注入**
`Simulation::new()`（`simulation.rs:104-111`）创建 DecisionPipeline 时，只调用了 `.with_llm_provider()` 和 `.with_llm_params()`，未注入 StrategyHub。导致 `build_prompt()` 中 `self.strategy_hub.as_ref()` 永远返回 `None`，策略检索代码从未执行。

**缺口 2：策略创建 SparkType 硬编码**
`world/mod.rs:313` 中策略创建使用写死的 `SparkType::CognitivePressure`，而策略检索使用 `infer_state_mode(world_state)` 动态推断。两者类型体系不匹配，导致策略检索几乎找不到自己创建的条目：
```
创建: CognitivePressure (固定)
检索: ResourcePressure / SocialPressure / Explore (动态)
```

### 1.3 关键干系人

- core crate 的 simulation 模块 — 负责 Simulation 创建和 DecisionPipeline 注入
- core crate 的 world 模块 — 负责 apply_action 中的策略创建
- core crate 的 decision 模块 — 提供 `infer_state_mode()` 函数
- core crate 的 strategy 模块 — StrategyHub 实现

## 2. 设计目标

### 目标

- 打通 StrategyHub 注入链路，使策略检索在决策 Prompt 中生效
- 策略创建使用与检索相同的 SparkType 推断逻辑，形成"创建→检索→参考"闭环
- 保持向后兼容，不破坏现有功能

### 非目标

- 压力系统已激活，本次不涉及
- 长期记忆已注入，本次不涉及
- 策略的跨 Agent 传播（文化传承）
- Merkle/签名验证

## 3. 整体架构

### 3.1 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                     Simulation::new()                        │
│                                                              │
│  for each agent_id:                                          │
│    ├─ StrategyHub::new(agent_id)  ──┐                       │
│    │                                │                        │
│    │    DecisionPipeline::new()     │                       │
│    │      .with_llm_provider(...)   │                       │
│    │      .with_strategy_hub(──) ───┘                       │
│    │      .with_llm_params(...)                              │
│    │                                │                        │
│    └─ run_agent_loop(pipeline) ────────────────────────┐   │
│                                                         │   │
└─────────────────────────────────────────────────────────│───┘
                                                          │
┌─────────────────────────────────────────────────────────│───┐
│              run_agent_loop()                            │   │
│                                                          │   │
│  infer_state_mode(world_state) → spark_type              │   │
│  agent.memory.get_summary(spark_type) → mem_summary      │   │
│                                                          │   │
│  pipeline.execute(..., mem_summary, ...)                 │   │
│    │                                                     │   │
│    ├─ build_prompt():                                    │   │
│    │   ├─ infer_state_mode(world_state) → spark_type    │   │
│    │   ├─ strategy_hub.retrieve(spark_type) → hint ◀────┼──┘
│    │   └─ wrap_strategy_for_prompt(hint)                 │
│    │                                                     │
│    └─ LLM 决策 + RuleEngine 校验                          │
│                                                          │
│  world.apply_action(action, spark_type) ──────────────┐  │
│    │                                                  │  │
│    └─ should_create_strategy()                        │  │
│        └─ create_strategy(hub, spark_type, ...) ──────┘  │
└──────────────────────────────────────────────────────────┘

闭环验证：
  创建时 spark_type = infer_state_mode(world_state)  ← 与检索时相同
  检索时 spark_type = infer_state_mode(world_state)
  → 匹配！
```

### 3.2 核心组件

| 组件 | 修改内容 |
|------|----------|
| `simulation.rs` | `Simulation::new()` 中为每个 Agent 创建 StrategyHub 并注入 DecisionPipeline |
| `agent_loop.rs` | 将 `infer_state_mode(world_state)` 的结果传递给 `world.apply_action()` |
| `world/mod.rs` | `apply_action()` 接收 spark_type 参数，策略创建使用传入值而非硬编码 |
| `strategy/create.rs` | 新增 `action_type_to_spark_type()` 映射函数（辅助） |

### 3.3 数据流设计

**策略闭环数据流：**

```
WorldState ──infer_state_mode──▶ SparkType (动态)
     │                              │
     │ 决策构建                     │ 策略创建
     ▼                              ▼
retrieve_strategy              create_strategy
     │                              │
     ▼                              ▼
注入Prompt                      写入磁盘
```

## 4. 详细设计

### 4.1 Simulation::new() 注入 StrategyHub

**修改位置：** `crates/core/src/simulation/simulation.rs`

`Simulation` 结构中 `pipeline` 是 `Arc<DecisionPipeline>`，但 StrategyHub 是每个 Agent 独立的。有两个方案：

**方案 A（当前采用）：** `Simulation` 持有 `Vec<StrategyHub>`（按 agent_id 索引），在 `run_agent_loop()` 中为每个 Agent 取对应的 hub 并通过某种方式注入。

但实际上 `DecisionPipeline` 是创建时绑定 StrategyHub 的，每个 Agent 有独立的 pipeline。当前代码中 pipeline 是共享的 `Arc<DecisionPipeline>`，只有一个实例。

查看 `agent_loop.rs:58-68`，`pipeline` 作为 `Arc<DecisionPipeline>` 传入，多个 Agent 共享同一个 pipeline 实例。

**问题：** DecisionPipeline 的 `strategy_hub` 是 `Option<StrategyHub>`，如果共享同一个 pipeline，只能注入一个 Agent 的 StrategyHub，这不对。

**解决方案：** 策略检索在 `build_prompt()` 中执行，此时有 `world_state` 和 `agent_id`。与其在 pipeline 上绑定单个 StrategyHub，不如：

- 给 DecisionPipeline 增加一个 `strategy_hubs: HashMap<AgentId, StrategyHub>` 字段
- 或者，在 `execute()` 方法中接收 `strategy_hub: Option<&StrategyHub>` 参数

考虑到改动最小且最清晰的方案：

**决策：在 `execute()` 中接收 `strategy_hub` 参数**

- `Simulation::new()` 中创建 `HashMap<AgentId, StrategyHub>`，存储在 Simulation 结构体中
- `agent_loop()` 中根据 agent_id 从 HashMap 取出对应的 StrategyHub
- `DecisionPipeline.execute()` 增加 `strategy_hub: Option<&StrategyHub>` 参数
- `build_prompt()` 中使用传入的 strategy_hub 而非 self.strategy_hub

伪代码：

```rust
// simulation.rs
let mut strategy_hubs: HashMap<AgentId, StrategyHub> = HashMap::new();
for agent_id in agent_ids {
    let mut hub = StrategyHub::new(&format!("{:?}", agent_id));
    let _ = hub.load_all_strategies();
    strategy_hubs.insert(agent_id, hub);
}

// agent_loop.rs — 新增参数
let strategy_hub = strategy_hubs.get(&agent_id);
let result = pipeline.execute(
    &agent_id, &world_state, &perception_summary,
    memory_summary_opt.as_deref(), action_feedback,
    strategy_hub,  // 新增
).await;

// decision/mod.rs — execute 签名变更
pub async fn execute(
    &self,
    agent_id: &AgentId,
    world_state: &WorldState,
    perception_summary: &str,
    memory_summary: Option<&str>,
    action_feedback: Option<&str>,
    strategy_hub: Option<&StrategyHub>,  // 新增
) -> DecisionResult

// build_prompt 中：
let strategy_hint = strategy_hub.and_then(|hub| {
    let state_mode = infer_state_mode(world_state);
    retrieve_strategy(hub, state_mode).map(|strategy| {
        let summary = get_strategy_summary(&strategy);
        wrap_strategy_for_prompt(&summary)
    })
});
```

### 4.2 策略创建使用动态 SparkType

**修改位置：** `crates/core/src/world/mod.rs` → `apply_action()`

当前代码（line 305-317）：
```rust
if should_create_strategy(is_success, candidate_count) {
    let agent = self.agents.get_mut(agent_id).unwrap();
    let spark_type = SparkType::CognitivePressure;  // 硬编码
    let _ = create_strategy(&agent.strategies, spark_type, self.tick as u32, &action.reasoning);
}
```

**修改为接收 spark_type 参数：**

```rust
pub fn apply_action(
    &mut self,
    agent_id: &AgentId,
    action: &Action,
    spark_type: Option<SparkType>,  // 新增参数
) -> ActionResult
```

在策略创建处使用：
```rust
if should_create_strategy(is_success, candidate_count) {
    let agent = self.agents.get_mut(agent_id).unwrap();
    let spark_type = spark_type.unwrap_or_else(|| infer_state_mode_from_agent(agent, self));
    let _ = create_strategy(&agent.strategies, spark_type, self.tick as u32, &action.reasoning);
}
```

**但这里有个问题：** `apply_action` 在 `agent_loop.rs` 中被调用，该处已经有 `infer_state_mode(&ws)` 的结果。可以直接传递。

### 4.3 核心算法

**infer_state_mode 复用：**

当前 `infer_state_mode()` 定义在 `decision/mod.rs`，接收 `&WorldState`：

```rust
pub fn infer_state_mode(world_state: &WorldState) -> SparkType {
    if world_state.agent_satiety <= 30 || world_state.agent_hydration <= 30 {
        return SparkType::ResourcePressure;
    }
    if !world_state.nearby_agents.is_empty() {
        return SparkType::SocialPressure;
    }
    SparkType::Explore
}
```

在 `agent_loop.rs` 中已经调用了此函数（line 82），可以将结果传递给 `apply_action`。

### 4.6 异常处理

| 异常场景 | 处理策略 |
|----------|----------|
| StrategyHub 目录不存在 | `load_all_strategies()` 返回空，不影响决策 |
| 策略文件损坏 | `parse_strategy_file()` 返回 None，该策略被跳过 |
| strategy_hub 为 None | `build_prompt()` 中 strategy_hint 为 None，prompt 不包含策略段 |
| spark_type 为 None | `apply_action` 中 fallback 到 `infer_state_mode`，保证总能创建策略 |

## 5. 技术决策

### 决策 1：StrategyHub 注入方式

- **选型方案**：在 `execute()` 方法中接收 `strategy_hub: Option<&StrategyHub>` 参数
- **选择理由**：当前 DecisionPipeline 是多个 Agent 共享的 `Arc<DecisionPipeline>`，无法在结构体上绑定单个 Agent 的 StrategyHub。通过参数传入最简洁，不破坏现有结构。
- **备选方案 A**：给 DecisionPipeline 增加 `HashMap<AgentId, StrategyHub>` 字段
  - **放弃原因**：增加结构体复杂度，需要管理 HashMap 的生命周期
- **备选方案 B**：为每个 Agent 创建独立的 DecisionPipeline
  - **放弃原因**：改动过大，pipeline 的 LLM provider 等资源共享也有意义

### 决策 2：spark_type 传递方式

- **选型方案**：`apply_action()` 新增 `spark_type: Option<SparkType>` 参数
- **选择理由**：`agent_loop` 中已有 `infer_state_mode(&ws)` 的结果，直接传递即可，无需在 world 层重新计算
- **备选方案**：在 `apply_action` 内部从 Agent 状态推断
  - **放弃原因**：`infer_state_mode` 需要 `WorldState`（包含 nearby_agents 等），而 `apply_action` 只能访问 `World` 的局部数据

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
|--------|----------|----------|
| `execute()` 签名变更导致编译错误 | 低 | 仅 `agent_loop.rs` 一处调用点，同步修改即可 |
| `apply_action()` 签名变更 | 低 | 检查所有调用点，统一传递 spark_type |
| StrategyHub 目录权限问题 | 低 | `load_all_strategies()` 对目录不存在做 gracefully 处理 |
| 策略检索结果注入过多 tokens | 中 | `prompt.rs` 已有 token 预算和分级截断，策略段是最先被截断的 |

## 7. 迁移方案

### 7.1 部署步骤

1. 修改 `simulation.rs`：创建 `HashMap<AgentId, StrategyHub>` 并存储到 Simulation 结构体
2. 修改 `agent_loop.rs`：传入 `strategy_hub` 和 `spark_type` 参数
3. 修改 `decision/mod.rs`：`execute()` 和 `build_prompt()` 增加 `strategy_hub` 参数
4. 修改 `world/mod.rs`：`apply_action()` 增加 `spark_type` 参数，替换硬编码
5. 编译验证：`cargo build`
6. 运行测试：`cargo test`

### 7.3 回滚方案

所有修改为增量式代码变更，不涉及数据迁移。回滚只需 revert git commit。

## 8. 待定事项

- [ ] 确认 `apply_action()` 的所有调用点都能获取到 spark_type
- [ ] 确认 Simulation 结构体增加 `strategy_hubs` 字段后，P2P 模式下的兼容性
