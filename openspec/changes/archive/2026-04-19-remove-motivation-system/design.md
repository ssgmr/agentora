# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

Agentora 是一个端侧 AI 智能体驱动的文明模拟器。当前架构中，六维动机系统（`MotivationVector`）与物理状态系统（health/satiety/hydration/inventory）并存，两套机制共同影响 Agent 决策。

技术栈：Rust + Godot 4 (GDExtension) + LLM（OpenAI 兼容端点）。

### 1.2 现状分析

当前决策管道为五阶段：硬约束过滤 → 上下文构建 → LLM 生成 → 规则校验 → 动机加权选择。存在以下问题：

1. **动机值与状态值双轨并行**：状态值通过硬编码阈值（`satiety≤50 → survival+0.3`）映射到动机，LLM 同时感知动机表格和状态文字，两套系统混乱
2. **动机值不可量化**：`MotivationVector` 的 6 个维度既像性格又像需求，没有可靠的实时更新机制
3. **Spark 系统冗余**：LLM 看到 `satiety=30` 就知道该找食物，不需要 Spark 从动机缺口"翻译"
4. **策略-动机联动无意义**：策略成功修改动机向量，但动机值本身没有明确语义，联动缺乏依据
5. **决策过度工程化**：Softmax 加权、点积计算、重复性惩罚——LLM 一个输出就够了

### 1.3 关键干系人

- 核心引擎：`crates/core` — 动机/决策/规则/世界/Agent/策略模块
- Bridge 层：`crates/bridge` — Godot GDExtension，负责发送状态到客户端
- Godot 客户端：`client/` — 渲染 UI，当前包含动机雷达图面板
- 测试：多个测试文件覆盖动机、决策、策略、规则引擎

## 2. 设计目标

### 目标

- 移除运行时动机系统，简化决策管道为：上下文构建 → LLM 生成 → 规则校验 → 执行
- 状态值（health/satiety/hydration/inventory）作为 Agent 唯一真实来源
- LLM 直接读取状态值做出决策，不再需要动机向量、Spark、缺口计算
- 规则引擎简化为纯校验，不再基于动机生成兜底动作
- Godot 客户端移除动机雷达图

### 非目标

- 不新增新的 Agent 状态维度（状态值扩展留给后续变更）
- 不修改 LLM 调用接口和 JSON 解析逻辑（仅移除 motivation_delta 字段）
- 不修改 ChronicleDB/短期记忆/策略检索的核心机制

## 3. 整体架构

### 3.1 架构概览

```
重构前：                          重构后：
┌──────────┐                      ┌──────────┐
│  World   │                      │  World   │
│  tick    │                      │  tick    │
└────┬─────┘                      └────┬─────┘
     │                                 │
     ▼                                 ▼
┌──────────────────┐          ┌──────────────────┐
│  Agent           │          │  Agent           │
│  motivation ──── │          │  health          │
│  ├─ decay()     │          │  satiety         │
│  ├─ apply_delta │          │  hydration       │  ← 唯一真实来源
│  └─ compute_gap │          │  inventory       │
└──────┬───────────┘          └──────┬───────────┘
       │                             │
       ▼                             ▼
┌──────────────────┐          ┌──────────────────┐
│  Spark           │          │  直接构建 Prompt  │
│  from_gap()      │          │  ┌──────────────┐ │
│  → 找缺口最大维度│          │  │ System Prompt│ │
└──────┬───────────┘          │  │ 状态值直读    │ │
       │                      │  │ 感知摘要     │ │
       ▼                      │  │ 记忆/策略    │ │
┌──────────────────┐          │  └──────────────┘ │
│  Decision Pipeline│          └──────┬───────────┘
│  LLM → 校验      │                 │
│  → 动机加权选择  │                 ▼
└──────┬───────────┘          ┌──────────────────┐
       │                      │  Decision Pipeline│
       ▼                      │  LLM → 校验 → 执行│
┌──────────────────┐          └──────┬───────────┘
│  Strategy Hub    │                 │
│  success/fail →  │                 ▼
│  modify motive   │          ┌──────────────────┐
└──────────────────┘          │  apply_action    │
                              │  只修改状态值     │
                              └──────────────────┘
```

### 3.2 核心组件

| 组件 | 变更前职责 | 变更后职责 |
|------|-----------|-----------|
| `MotivationVector` | 6维动机向量，衰减/缺口/delta | **移除** |
| `Spark` / `SparkType` | 从动机缺口生成决策触发器 | **移除** |
| `DecisionPipeline` | 五阶段管道含动机加权选择 | 简化为 LLM→校验→执行 |
| `RuleEngine` | 硬约束过滤+校验+基于动机的兜底决策 | 仅校验 |
| `PromptBuilder` | 格式化动机表格+Spark | 只展示状态值 |
| `Agent` | 含 motivation 字段 | 移除 motivation |
| `Strategy` | 含 motivation_delta，联动动机 | 移除 motivation_delta |
| `World` | tick 中衰减动机 | 不再衰减动机 |

### 3.3 数据流设计

```
重构后的决策循环：

Tick 开始
  │
  ▼
World.survival_consumption_tick()
  │  satiety -= 1, hydration -= 1
  │  归零时 health -= 1
  ▼
World.structure_effects_tick()
  │  Camp 回血
  ▼
World.pressure_tick()
  │  环境压力事件
  ▼
World.check_agent_death()
  │  health==0 或 age>=max_age → 死亡 → Legacy
  ▼
Decision: 构建 Prompt
  │  System Prompt: "你是 [角色名]，[性格描述]"
  │  当前状态: health/satiety/hydration/inventory
  │  感知: 附近资源/Agent/建筑/地形
  │  记忆: 短期记忆 + 编年史
  │  策略: 成功策略参考
  ▼
LLM 调用 → JSON 解析 → {action_type, params, reasoning}
  │
  ▼
RuleEngine.validate_action()
  │  边界/资源/地形/目标存在性
  ├─ 通过 → apply_action()
  │         │
  │         ▼
  │       修改状态值（Eat → satiety+30）
  │       经验值/升级/里程碑
  │       策略创建检查
  │
  └─ 失败 → 记录错误到 last_action_result
            下次 Prompt 包含错误反馈

Tick 结束
```

## 4. 详细设计

### 4.1 移除的核心类型

#### 从 `motivation.rs` 移除（整个文件）

- `MotivationVector` 结构体及所有 impl
- `DIMENSION_NAMES` / `DECAY_ALPHA` / `NEUTRAL_VALUE` / `DIM_*` 常量
- 整个文件将被删除，不再需要 `mod motivation;` 声明

#### 从 `decision.rs` 移除

- `SparkType` 枚举及 impl
- `Spark` 结构体及 impl
- `CandidateSource` 枚举
- `ActionCandidate.motivation_delta` 字段
- `ActionCandidate.source` 字段
- `DecisionPipeline.select_with_motivation()` 方法
- `DecisionPipeline.softmax_select()` 方法
- `DecisionPipeline.dot_product()` 方法
- `DecisionPipeline.compute_dot_product()` 方法
- `DecisionPipeline.select_unique_or_motivated()` 方法
- `DecisionPipeline.action_type_name()` 方法（仅用于重复性检测，保留或移除取决于是否保留该特性）

#### 从 `rule_engine.rs` 移除

- `RuleEngine.rule_decision()` — 基于动机的完整决策方法
- `RuleEngine.fallback_action()` — 调用 rule_decision 的兜底方法
- `RuleEngine.select_build_type()` — 基于动机维度选择建筑
- `RuleEngine.generate_social_message()` — 社交消息生成
- `RuleEngine.generate_express_message()` — 表达消息生成
- `RuleEngine.is_low_value_action()` — 低价值动作判断

#### 从 `types.rs` 修改

- `Action` 结构体：移除 `motivation_delta: [f32; 6]` 字段

#### 从 `agent/mod.rs` 修改

- `Agent` 结构体：移除 `motivation: MotivationVector` 字段
- 移除 `effective_motivation()` 方法
- 移除 `inject_preference()` 方法（或保留并改造为注入临时倾向）
- 移除 `tick_preferences()` 中的动机相关部分

### 4.2 简化的决策管道

#### `DecisionPipeline.execute()` 新方法签名

```rust
pub async fn execute(
    &self,
    agent_id: &AgentId,
    world_state: &WorldState,
    memory_summary: Option<&str>,
    last_action_result: Option<&str>,
    agent_profile: Option<&AgentProfile>, // 可选，角色配置
) -> Option<ActionCandidate> {
    // 1. 构建 Prompt（不再需要 motivation/spark 参数）
    let prompt = self.build_prompt(
        agent_id, world_state, memory_summary,
        last_action_result, agent_profile,
    );

    // 2. 调用 LLM
    match self.call_llm(&prompt, world_state.agent_position).await {
        Ok(action) => {
            // 3. 规则校验
            if self.rule_engine.validate_action(&action, world_state) {
                Some(action)
            } else {
                // 校验失败，不执行动作，反馈错误
                tracing::warn!("Agent {} 动作校验失败", agent_id.as_str());
                None
            }
        }
        Err(e) => {
            // LLM 不可用时的兜底
            if is_provider_unavailable(&e) {
                self.rule_engine.survival_fallback(world_state)
            } else {
                // LLM 返回无效响应
                tracing::warn!("Agent {} LLM 返回无效决策: {}", agent_id.as_str(), e);
                None
            }
        }
    }
}
```

#### 新增：`RuleEngine.survival_fallback()`

```rust
/// LLM 不可用时的生存兜底
pub fn survival_fallback(&self, world_state: &WorldState) -> Option<ActionCandidate> {
    // 1. 如果 satiety/hydration 极低且背包有食物/水 → Eat/Drink
    // 2. 如果脚下有资源 → Gather
    // 3. 如果视野有资源 → MoveToward 最近资源
    // 4. 否则 → Wait
}
```

#### `ActionCandidate` 简化后结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCandidate {
    pub reasoning: String,
    pub action_type: ActionType,
    pub target: Option<String>,
    pub params: HashMap<String, serde_json::Value>,
}
```

### 4.3 Prompt 构建变化

#### 移除的内容

- 动机表格（`生存与资源: 0.75` 等 6 行）
- Spark 信息（`当前生存需求较强`）
- 动机值说明（`0.5 为中性基线` 等）
- `build_decision_prompt()` 方法签名中的 `motivation` 和 `spark` 参数

#### 保留的内容

- System Prompt（需改造为角色配置）
- 感知摘要（资源/Agent/建筑/地形）
- 记忆摘要
- 策略参考
- 输出格式指令

#### 新的 System Prompt 示例

```
你是一个自主决策的 AI Agent，在一个共享世界中生存。

世界规则：
- 饱食度和水分度会随时间自然下降，归零时 HP 会持续扣减
- 当饱食度或水分度偏低时，你需要主动进食或饮水
- 世界中有各种资源（木材、石材、铁矿、食物、水源），可以采集
- 采集到的食物和水源可以用于进食和饮水动作
- 你可以用资源建造建筑、与其他 Agent 交易或战斗
- 你应该根据当前状态和环境，自主决定做什么
```

### 4.4 策略系统修改

#### `Strategy` 结构体

```rust
// 修改前
pub struct Strategy {
    pub spark_type: String,
    pub success_rate: f32,
    pub use_count: u32,
    pub last_used_tick: u32,
    pub created_tick: u32,
    pub deprecated: bool,
    pub motivation_delta: Option<[f32; 6]>,  // ← 移除
    pub content: String,
}

// 修改后
pub struct Strategy {
    pub spark_type: String,
    pub success_rate: f32,
    pub use_count: u32,
    pub last_used_tick: u32,
    pub created_tick: u32,
    pub deprecated: bool,
    pub content: String,
}
```

#### `StrategyFrontmatter` 同步修改

移除 `motivation_delta: Option<[f32; 6]>` 字段。

#### 移除的文件

- `crates/core/src/strategy/motivation_link.rs` — 整个文件删除
- `mod.rs` 中移除 `pub mod motivation_link;` 声明

### 4.5 World 模块修改

#### `advance_tick()` 中移除

```rust
// 删除这两行
for (_, agent) in self.agents.iter_mut() {
    agent.motivation.decay();
}
```

#### `apply_action()` 中移除

```rust
// 删除：动机 delta 应用
let agent = self.agents.get_mut(agent_id).unwrap();
for (i, delta) in action.motivation_delta.iter().enumerate() {
    if i < 6 {
        let new_val = agent.motivation[i] + delta;
        agent.motivation[i] = new_val.clamp(0.0, 1.0);
    }
}

// 删除：策略-动机联动
on_strategy_success(&mut agent.motivation, &strategy);
on_strategy_failure(&mut agent.motivation, &strategy);

// 删除：策略创建中的动机对齐参数
if should_create_strategy(is_success, candidate_count, motivation_alignment) {
    // motivation_alignment 不再有意义
}
```

### 4.6 Bridge 层修改

`crates/bridge/src/lib.rs`：

- `AgentSnapshot` 序列化：移除 `motivation` 数组
- `AgentDelta` 枚举：移除动机变化相关字段
- Godot 客户端不再接收动机数据

### 4.7 Godot 客户端修改

- `motivation_radar.gd` — 整个文件删除
- `main.gd` / 场景文件 — 移除动机雷达图节点引用
- `agent_detail_panel.gd` — 移除动机值展示

### 4.6 异常处理

| 异常场景 | 处理策略 |
|----------|---------|
| LLM 超时/失败 | `RuleEngine.survival_fallback()` 提供基于状态的兜底动作 |
| LLM 返回无效 JSON | 记录错误到 `last_action_result`，下一 tick 包含错误反馈让 LLM 自我修正 |
| 规则校验失败 | 不执行动作，记录原因到 `last_action_result` |
| 策略创建时的 `spark_type` 分类 | 保留 spark_type 字段（策略按状态模式分类），但不再关联动机值 |

## 5. 技术决策

### 决策 1：规则引擎兜底策略

- **选型方案**：保留基于状态的生存兜底（satiety/hydration 优先），移除基于动机的通用兜底
- **选择理由**：LLM 不可用是罕见场景，兜底只需保证 Agent 不死。基于状态的逻辑简单明确：饿了吃/找食物，渴了喝/找水，其他时候 Wait
- **备选方案**：移除所有兜底，LLM 不可用时直接 Wait
- **放弃原因**：纯 Wait 兜底太保守，可能导致 Agent 饿死

### 决策 2：`ActionCandidate.source` 字段是否保留

- **选型方案**：移除 `source` 字段
- **选择理由**：`source` 用于区分 LLM vs RuleEngine 来源，但重构后 LLM 是唯一生成源，规则引擎只做校验
- **备选方案**：保留 `source` 用于统计调试
- **放弃原因**：调试可以通过日志实现，不需要在数据结构中保留

### 决策 3：`temp_preferences` 是否保留

- **选型方案**：保留 `Agent.temp_preferences`
- **选择理由**：这是运行时动态注入倾向的机制（如外部配置调整 Agent 行为），与动机系统无关
- **备选方案**：一并移除，角色配置全部走 AgentProfile
- **放弃原因**：temp_preferences 是灵活的运行时调整机制，AgentProfile 是静态配置，两者互补

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
|--------|---------|---------|
| 移除动机系统后，测试覆盖不足导致回归 | 中 | 保留现有决策/策略测试，仅移除动机相关用例 |
| LLM 不可用时兜底逻辑过于简单 | 低 | 当前只需保证不死，后续可根据需要增强 |
| Godot 客户端 UI 遗漏删除 | 低 | 搜索所有 `motivation` 引用，逐一清理 |
| 策略系统 spark_type 分类失去语义 | 中 | 保留 spark_type 作为状态模式分类名（如 resource_pressure 改为 low_satiety 等）|

## 7. 迁移方案

### 7.1 部署步骤

1. 修改 core crate：移除 motivation.rs、简化 decision.rs/rule_engine.rs/agent.rs/world.rs/strategy.rs
2. 修改 bridge crate：移除动机序列化
3. 修改测试文件：移除 motivation_tests，修改 decision_tests/strategy_tests
4. 修改 Godot 客户端：删除 motivation_radar.gd，清理场景引用
5. `cargo build` 验证编译通过
6. `cargo test` 验证测试通过
7. 运行集成测试或手动验证

### 7.2 回滚方案

由于是代码重构而非数据迁移，回滚即 `git revert`。无需数据迁移。

## 8. 待定事项

- [ ] `spark_type` 在策略系统中的命名是否改为更语义化的名称（如 `state_pattern`）？
- [ ] `PersonalitySeed` 是否扩展为更丰富的 `AgentProfile`（含 MBTI、行为倾向等）？还是先移除动机，后续再做角色配置？
- [ ] Godot 客户端动机雷达图删除后，是否需要替代的角色状态展示？
