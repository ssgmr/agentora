# 实施任务清单

## 1. Core Crate — 移除动机类型定义

删除动机系统的核心数据结构和常量定义。

- [x] 1.1 删除 `crates/core/src/motivation.rs` 整个文件
  - 包含 `MotivationVector`、`DIMENSION_NAMES`、`DECAY_ALPHA`、`NEUTRAL_VALUE` 等
  - 从 `crates/core/src/lib.rs` 或 `mod.rs` 中移除 `mod motivation;` 声明

- [x] 1.2 从 `crates/core/src/types.rs` 移除 `Action.motivation_delta` 字段
  - 修改 `Action` 结构体定义
  - 更新所有构造 `Action` 的代码（如 `parse_action_json` 中的反序列化）

- [x] 1.3 从 `crates/core/src/decision.rs` 移除动机相关类型
  - 移除 `Spark` 结构体及 impl
  - 移除 `Spark::from_gap()` 方法
  - 移除 `SparkType::from_dimension()` 方法（保留枚举用于策略/记忆分类）
  - 移除 `ActionCandidate.motivation_delta` 和 `ActionCandidate.source` 字段
  - 移除所有动机加权选择方法（select_with_motivation、softmax_select、dot_product 等）
  - 简化 DecisionPipeline.execute() 为 LLM→校验→执行
  - 移除 build_prompt 的 motivation/spark 参数

## 2. Core Crate — 修改 Agent 实体

从 Agent 中移除动机字段，保留状态值。

- [x] 2.1 从 `crates/core/src/agent/mod.rs` 移除 `Agent.motivation` 字段
  - 移除 `motivation: MotivationVector` 字段
  - 移除 `effective_motivation()` 方法
  - 移除 `inject_preference()` 中与动机相关的方法（保留 temp_preferences 机制）
  - 简化 `tick_preferences()` 中的动机相关部分

## 3. Core Crate — 简化决策管道

将五阶段管道简化为 LLM → 校验 → 执行。

- [x] 3.1 从 `crates/core/src/decision.rs` 移除动机加权选择方法
  - 移除 `select_with_motivation()`
  - 移除 `softmax_select()`
  - 移除 `dot_product()` 和 `compute_dot_product()`
  - 移除 `select_unique_or_motivated()`
  - 移除 `action_type_name()`
  - 移除 `is_low_value_action()`

- [x] 3.2 重构 `DecisionPipeline.execute()` 方法签名
  - 移除 `motivation` 和 `spark` 参数
  - 新签名: `execute(agent_id, world_state, memory_summary, action_feedback)`
  - 内部流程: 构建 Prompt → LLM 调用 → 规则校验 → 返回结果

- [x] 3.3 修改 `crates/core/src/prompt.rs` 中的 Prompt 构建
  - 移除 `build_decision_prompt()` 的 `motivation` 和 `spark` 参数
  - 移除 `format_motivation()` 方法
  - 移除动机表格和 Spark 信息的输出
  - 更新 System Prompt 为世界规则描述
  - 移除输出格式中的 `motivation_delta` 字段

## 4. Core Crate — 简化规则引擎

规则引擎从"决策+校验"简化为纯校验+LLM失败兜底。

- [x] 4.1 从 `crates/core/src/rule_engine.rs` 移除动机决策方法
  - 移除 `rule_decision()`
  - 移除 `fallback_action()`
  - 移除 `select_build_type()`
  - 移除 `generate_social_message()`
  - 移除 `generate_express_message()`
  - 移除 `is_low_value_action()`

- [x] 4.2 新增 `RuleEngine.survival_fallback()` 方法
  - satiety/hydration 极低且有食物/水 → Eat/Drink
  - 脚下有资源 → Gather
  - 视野有资源 → MoveToward 最近资源
  - 否则 → Wait

## 5. Core Crate — 修改 World 模块

从 World 的 tick 和 apply_action 中移除动机操作。

- [x] 5.1 从 `crates/core/src/world/mod.rs` 的 `advance_tick()` 移除动机衰减
  - 删除 `agent.motivation.decay()` 调用

- [x] 5.2 从 `crates/core/src/world/mod.rs` 的 `apply_action()` 移除动机操作
  - 删除动机 delta 应用逻辑
  - 删除策略-动机联动调用 `on_strategy_success()` / `on_strategy_failure()`
  - 删除策略创建中的 `motivation_alignment` 参数

## 6. Core Crate — 修改策略系统

移除策略的动机关联字段和模块。

- [x] 6.1 删除 `crates/core/src/strategy/motivation_link.rs` 整个文件
  - 从 `crates/core/src/strategy/mod.rs` 中移除 `mod motivation_link;` 声明

- [x] 6.2 从 `crates/core/src/strategy/mod.rs` 修改 `Strategy` 结构体
  - 移除 `motivation_delta: Option<[f32; 6]>` 字段

- [x] 6.3 修改 `StrategyFrontmatter` 同步移除 `motivation_delta` 字段
  - 更新 frontmatter 序列化/反序列化逻辑

- [x] 6.4 从 `crates/core/src/strategy/create.rs` 移除动机相关逻辑
  - 移除 `motivation_delta` 归一化和记录
  - 从 `should_create_strategy()` 移除 `motivation_alignment` 参数

- [x] 6.5 修改策略 Prompt 注入逻辑
  - 不再计算候选与策略的动机对齐度
  - 不再基于动机对齐度给予额外 boost

## 7. Bridge Crate — 移除动机序列化

从 Godot 桥接层移除动机数据传输。

- [x] 7.1 从 `crates/bridge/src/lib.rs` 的 `AgentSnapshot` 移除 `motivation` 数组
  - 修改序列化逻辑，不再包含动机数据

- [x] 7.2 从 `crates/bridge/src/lib.rs` 的 `AgentDelta` 移除动机相关字段
  - 移除动机变化的枚举变体

## 8. Godot 客户端 — 移除动机 UI

从客户端删除动机雷达图和相关展示。

- [x] 8.1 删除 `client/scripts/motivation_radar.gd` 整个文件
- [x] 8.2 从 `client/scripts/agent_detail_panel.gd` 移除动机值展示
  - 移除动机雷达图节点引用
  - 移除动机数据更新逻辑

- [x] 8.3 清理 `client/` 场景文件（`.tscn`）中的动机雷达图节点
  - 搜索并移除所有 `motivation_radar` 引用

## 9. 测试更新

更新测试以匹配新的数据结构。

- [x] 9.1 移除 `tests/motivation_tests.rs` 整个测试文件
  - 动机系统不再存在，测试无需保留

- [x] 9.2 修改 `tests/decision_tests.rs`
  - 移除涉及 `motivation_delta`、`Spark`、`softmax_select` 的测试用例
  - 更新测试以使用简化的 `ActionCandidate` 结构

- [x] 9.3 修改 `tests/strategy_tests.rs`
  - 移除涉及 `motivation_delta` 和 `motivation_link` 的测试用例

- [x] 9.4 修改其他测试文件中引用 `motivation` 的部分
  - `tests/multi_agent.rs`
  - `tests/tier2_action_tests.rs`
  - `tests/legacy_tests.rs`
  - `tests/single_agent.rs`

- [x] 9.5 为 `RuleEngine.survival_fallback()` 添加测试
  - 测试低饱食度/水分度场景
  - 测试有/无食物场景
  - 测试默认 Wait 场景

## 10. 构建与验证

编译、测试、集成验证。

- [x] 10.1 运行 `cargo build` 验证编译通过
  - 修复所有编译错误
  - 确保 `agentora-bridge` 也能编译

- [x] 10.2 运行 `cargo test` 验证单元测试全部通过
- [x] 10.3 运行 Godot 客户端验证 UI 无报错
  - 检查控制台无缺失节点/脚本错误
  - 验证 Agent 状态正常显示

- [x] 10.4 清理所有残留的 `motivation` / `Spark` 引用
  - `grep -rn "motivation\|Spark\|motivation_delta"` 搜索全项目
  - 逐一确认无遗漏

## 任务依赖关系

```
阶段一（类型移除）        1.x ─────────────┐
                                         │
阶段二（Agent实体）       2.x ─────────────┤
                                         │
阶段三（决策管道）        3.x ←────────────┘ ← 依赖 1.x, 2.x
                                         │
阶段四（规则引擎）        4.x ←────────────┤ ← 依赖 1.x, 3.x（ActionCandidate变化）
                                         │
阶段五（World模块）       5.x ←────────────┤ ← 依赖 1.x, 2.x
                                         │
阶段六（策略系统）        6.x ←────────────┘ ← 依赖 1.x
                                         │
阶段七（Bridge层）        7.x ←────────────┘ ← 依赖 1.x, 2.x
                                         │
阶段八（Godot客户端）     8.x ←────────────┘ ← 依赖 7.x
                                         │
阶段九（测试更新）        9.x ←────────────┘ ← 依赖 1.x ~ 6.x
                                         │
阶段十（构建验证）        10.x ←────────────┘ ← 依赖所有阶段
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 一 | 1.x | 核心类型移除 — 删除动机定义，修改 Action/ActionCandidate 结构 |
| 二 | 2.x | Agent 实体 — 移除 motivation 字段，简化 Agent 结构 |
| 三 | 3.x | 决策管道 — 简化为 LLM→校验→执行，移除 Spark/加权选择 |
| 四 | 4.x | 规则引擎 — 移除动机决策方法，新增 survival_fallback |
| 五 | 5.x | World 模块 — 移除 tick 中的动机衰减和 apply_action 中的动机操作 |
| 六 | 6.x | 策略系统 — 删除 motivation_link，移除 motivation_delta |
| 七 | 7.x | Bridge 层 — 移除动机的序列化和数据传输 |
| 八 | 8.x | Godot 客户端 — 删除雷达图，清理场景引用 |
| 九 | 9.x | 测试更新 — 移除/修改动机相关测试，新增 fallback 测试 |
| 十 | 10.x | 构建验证 — 编译、测试、运行 Godot 验证 |

## 文件结构总览

```
crates/core/src/
├── motivation.rs                    [删除]
├── decision.rs                      [修改] — 移除 Spark/CandidateSource/motivation 方法
├── rule_engine.rs                   [修改] — 移除动机决策，新增 survival_fallback
├── prompt.rs                        [修改] — 移除动机表格/Spark 输出
├── types.rs                         [修改] — Action 移除 motivation_delta
├── agent/
│   └── mod.rs                       [修改] — Agent 移除 motivation 字段
├── strategy/
│   ├── mod.rs                       [修改] — Strategy 移除 motivation_delta
│   ├── motivation_link.rs           [删除]
│   ├── create.rs                    [修改] — 移除 motivation_alignment
│   └── retrieve.rs                  [修改] — 移除动机对齐度计算
├── world/
│   └── mod.rs                       [修改] — 移除动机衰减和应用

crates/bridge/src/
└── lib.rs                           [修改] — AgentSnapshot/AgentDelta 移除动机

client/
├── scripts/
│   ├── motivation_radar.gd          [删除]
│   └── agent_detail_panel.gd        [修改] — 移除动机展示
└── scenes/                          [修改] — 移除雷达图节点引用

tests/
├── motivation_tests.rs              [删除]
├── decision_tests.rs                [修改]
├── strategy_tests.rs                [修改]
├── multi_agent.rs                   [修改]
└── tier2_action_tests.rs            [修改]
```
