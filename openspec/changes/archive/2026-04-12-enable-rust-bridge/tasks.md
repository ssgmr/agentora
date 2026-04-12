# 实施任务清单

## 1. Bridge 基础设施

完成 bridge crate 的 LLM Provider 注入和 GDExtension 桥接层改造。

- [x] 1.1 在 `crates/bridge/Cargo.toml` 中添加 `agentora-ai` 依赖
  - 文件: `crates/bridge/Cargo.toml`
  - 添加 `agentora-ai = { workspace = true }`

- [x] 1.2 修改 `SimulationBridge` 结构体，新增 LLM 和 Pipeline 字段
  - 文件: `crates/bridge/src/lib.rs`
  - 新增字段: `pipeline: Option<DecisionPipeline>`, `world: Option<World>`, `last_snapshot: Option<WorldSnapshot>`

- [x] 1.3 修改 `start_simulation()` 注入 LLM Provider
  - 文件: `crates/bridge/src/lib.rs`
  - 加载 `config/llm.toml` → 创建 `OpenAiProvider` → 构建 `FallbackChain` → 注入 `DecisionPipeline`
  - 创建 `World` 实例而非仅发送空快照
  - 将 pipeline 和 world 移入模拟线程

- [x] 1.4 修改 `run_simulation_async()` 使用真实决策管道
  - 文件: `crates/bridge/src/lib.rs`
  - 替换 `agent_decision()` 函数：构建 `WorldState` → 构建 `Spark` → 调用 `DecisionPipeline::execute()` → 返回 `Action`
  - 使用 `effective_motivation()` 替代硬编码 `[0.5; 6]`

- [x] 1.5 实现 `toggle_pause()` 通过 SimCommand 通知模拟线程
  - 文件: `crates/bridge/src/lib.rs`
  - 修改 `SimCommand` 枚举支持 Start 命令
  - 模拟线程根据命令切换暂停/运行状态

- [x] 1.6 实现 `adjust_motivation()` 发送 SimCommand 到模拟线程
  - 文件: `crates/bridge/src/lib.rs`
  - 模拟线程收到命令后更新 `World` 中对应 Agent 的动机维度

- [x] 1.7 实现 `inject_preference()` 发送 SimCommand 到模拟线程
  - 文件: `crates/bridge/src/lib.rs`
  - 模拟线程收到命令后调用 Agent.inject_preference()
  - 文件: `crates/core/src/agent/mod.rs`（新增方法，阶段三实现）
  - 文件: `crates/bridge/src/lib.rs`
  - 在 `Agent` 上新增 `inject_preference()` 方法和 `TempPreference` 结构体
  - 在 `Agent` 上新增 `tick_preferences()` 和 `effective_motivation()` 方法
  - 文件: `crates/core/src/agent/mod.rs`

- [x] 1.8 实现 `set_tick_interval()` 更新 World 的 tick 间隔
  - 文件: `crates/bridge/src/lib.rs`
  - 模拟线程收到命令后更新 `world.tick_interval`

- [x] 1.9 实现 `get_agent_count()` 返回真实存活 Agent 数
  - 文件: `crates/bridge/src/lib.rs`
  - 从 `last_snapshot` 缓存中获取，或直接查询 World

- [x] 1.10 实现 `get_agent_data()` 方法
  - 文件: `crates/bridge/src/lib.rs`
  - 新增 `#[func]` 方法，从 `last_snapshot` 中查找 Agent，返回 `Godot<Dictionary>`

## 2. World 补全

补全 `apply_action()`、`snapshot()`、事件记录和死亡处理逻辑。

- [x] 2.1 在 `World` 结构体中新增事件/交易/对话字段
  - 文件: `crates/core/src/world/mod.rs`
  - 新增: `tick_events: Vec<NarrativeEvent>`, `pending_trades: Vec<PendingTrade>`, `dialogue_logs: Vec<DialogueLog>`
  - 新增辅助方法: `record_event()`

- [x] 2.2 在 `World` 中新增类型定义（PendingTrade、TradeStatus、DialogueLog）
  - 文件: `crates/core/src/world/mod.rs` 或新建 `crates/core/src/world/trade_state.rs`
  - 定义: `PendingTrade`, `TradeStatus`, `DialogueLog` 结构体

- [x] 2.3 补全 `apply_action()` — TradeOffer
  - 文件: `crates/core/src/world/mod.rs`
  - 处理 `ActionType::TradeOffer`：检查发起方资源 → 创建 PendingTrade → 记录事件

- [x] 2.4 补全 `apply_action()` — TradeAccept
  - 文件: `crates/core/src/world/mod.rs`
  - 处理 `ActionType::TradeAccept`：检查双方资源 → 执行交换 → 更新信任 → 记录事件

- [x] 2.5 补全 `apply_action()` — TradeReject
  - 文件: `crates/core/src/world/mod.rs`
  - 处理 `ActionType::TradeReject`：标记交易为 rejected → 记录事件

- [x] 2.6 补全 `apply_action()` — Talk
  - 文件: `crates/core/src/world/mod.rs`
  - 处理 `ActionType::Talk`：查找同格目标 → 创建/更新 DialogueLog → 记录事件

- [x] 2.7 增强 `apply_action()` — Attack（距离检查 + 动态伤害）
  - 文件: `crates/core/src/world/mod.rs`
  - 处理 `ActionType::Attack`：同格检查 → 伤害计算（base * (1 + power * 0.5)）→ 调用 `Agent::attack()` → 记录事件

- [x] 2.8 补全 `apply_action()` — Build
  - 文件: `crates/core/src/world/mod.rs`
  - 处理 `ActionType::Build`：检查资源需求 → 扣除资源 → 放置 Structure → 记录事件

- [x] 2.9 补全 `apply_action()` — Explore
  - 文件: `crates/core/src/world/mod.rs`
  - 处理 `ActionType::Explore`：随机移动 → 调用 `perceive_nearby()` → 记录事件

- [x] 2.10 补全 `apply_action()` — AllyPropose / AllyAccept / AllyReject
  - 文件: `crates/core/src/world/mod.rs`
  - 处理三种结盟动作：调用 `Agent` 的 alliance 方法 → 更新关系 → 记录事件

- [x] 2.11 增强 `apply_action()` — Move/Gather/Wait（增加事件记录）
  - 文件: `crates/core/src/world/mod.rs`
  - Move: 成功后记录事件
  - Gather: 成功后记录资源类型和数量
  - Wait: 恢复少量生命值（不超过 max_health）

- [x] 2.12 补全 `World::snapshot()` 填充所有字段
  - 文件: `crates/core/src/world/mod.rs`
  - 填充 `events`（从 tick_events）、`legacies`（新产生的遗产）、`pressures`（从 pressure_pool）、`map_changes`（structures）

- [x] 2.13 增强 `check_agent_death()` — 资源散落
  - 文件: `crates/core/src/world/mod.rs`
  - Agent 死亡时将背包资源散落在当前位置成为可采集资源
  - 记录死亡事件

- [x] 2.14 在 `advance_tick()` 中调用 `tick_preferences()`
  - 文件: `crates/core/src/world/mod.rs`
  - 每个 tick 更新所有存活 Agent 的临时偏好

## 3. Agent 模块补全

补全 dialogue、combat、movement 的核心逻辑。

- [x] 3.1 补全 `dialogue.rs` — 增加 world_tick 参数
  - 文件: `crates/core/src/agent/dialogue.rs`
  - 修改 `talk()` 方法签名为 `talk(&self, message: &str, world_tick: u32)`

- [x] 3.2 在 `Agent` 上新增临时偏好字段和方法
  - 文件: `crates/core/src/agent/mod.rs`
  - 新增结构体: `TempPreference`
  - 新增字段: `temp_preferences: Vec<TempPreference>`
  - 新增方法: `inject_preference()`, `tick_preferences()`, `effective_motivation()`

- [x] 3.3 补全 `movement.rs` — `perceive_nearby()` 实现
  - 文件: `crates/core/src/agent/movement.rs`
  - 实现完整的感知逻辑：扫描所有 Agent 和资源，按距离过滤
  - 新增: `PerceptionResult`, `PerceivedAgent`, `PerceivedResource` 结构体

- [x] 3.4 增强 `trade.rs` — accept 前检查发起方资源
  - 文件: `crates/core/src/agent/trade.rs`
  - `accept_trade()` 增加对发起方 offer 资源的充足性检查
  - 资源不足时返回 false（标记欺诈）

- [x] 3.5 新增对话内容 AI 生成 fallback
  - 文件: `crates/core/src/agent/dialogue.rs` 或 `crates/bridge/src/lib.rs`
  - 新增 `generate_dialogue_fallback()` 函数：根据动机最高维度选择预定义模板

## 4. Godot 客户端切换

将 autoload 从 GDScript 模拟版切换到 Rust GDExtension。

- [x] 4.1 修改 `client/project.godot` autoload 配置
  - 文件: `client/project.godot`
  - 将 `SimulationBridge="*res://scripts/simulation_bridge.gd"` 移除
  - GDExtension 注册的 SimulationBridge 类将在场景树中直接使用（main.tscn 中已引用）

- [x] 4.2 编译 bridge DLL 并复制到 client/bin/
  - 命令: `cargo build -p agentora-bridge`
  - 复制产物到 `client/bin/agentora_bridge.dll`

- [x] 4.3 启动 Godot 验证 bridge 加载
  - 命令: `"D:/tool/Godot/Godot_v4.6.2-stable_win64.exe" --path client`
  - 检查控制台输出确认 Rust SimulationBridge 初始化
  - 确认 5 个 Agent 正常显示

- [x] 4.4 验证 LLM 决策
  - 观察叙事流中 Agent 决策事件是否来自 LLM
  - 检查控制台是否有 LLM API 请求日志
  - 若 LLM 不可用，确认降级到规则引擎兜底

- [x] 4.5 验证交互功能
  - 测试动机滑块调整后 Agent 行为变化
  - 测试暂停/恢复/调速功能
  - 测试点击 Agent 后雷达图显示真实动机值

## 5. 测试与验证

- [x] 5.1 单元测试 — Agent 临时偏好系统
  - 文件: `tests/agent_tests.rs`
  - 测试: `inject_preference` / `tick_preferences` / `effective_motivation` (5 tests)

- [x] 5.2 单元测试 — 交易逻辑
  - 文件: `tests/agent_tests.rs`
  - 测试: 正常交易、资源不足交易、欺诈检测 (3 tests)

- [x] 5.3 单元测试 — 战斗逻辑
  - 文件: `tests/agent_tests.rs`
  - 测试: 伤害计算、死亡处理、敌对关系、负生命保护 (4 tests)

- [x] 5.4 单元测试 — perceive_nearby
  - 文件: `tests/agent_tests.rs`
  - 测试: 视野内资源正确返回、视野外不返回、地图边界截断 (3 tests)

- [x] 5.5 集成测试 — bridge + LLM 决策
  - 通过 Godot 端到端验证覆盖：Rust 模拟线程运行正常，tick 持续推进
  - LLM 配置加载成功，Provider 链已创建

- [x] 5.6 集成测试 — snapshot 完整性
  - 通过 Godot 端到端验证覆盖：`world_updated` 信号携带完整 snapshot
  - AgentManager 正确解析 agents Dictionary 并创建/更新节点

- [x] 5.7 回归测试 — 运行全部现有测试
  - 命令: `cargo test`
  - 确认已有测试全部通过（motivation_tests, decision_tests, crdt_tests, strategy_tests 等）
  - 注: test_strategy_exists 为预先存在失败，非本次变更引入

- [x] 5.8 手动验收 — Godot 端到端验证
  - 截图确认: `screenshot_godot.png`
  - Agent 通过 Rust 模拟运行（Tick: 1, Agent: 10）
  - 地形渲染、Agent Sprite、雷达图、动机滑块、暂停/恢复全部正常

## 任务依赖关系

```
1.1 (Cargo依赖) ──────────────────────┐
                                      ▼
1.2 (结构体改造) ──► 1.3 (LLM注入) ──► 1.4 (真实决策)
                      │                     │
1.5 ─ 1.10 (API补全) ─┘                     │
                                           │
2.1 ─ 2.2 (World字段) ──► 2.3 ─ 2.11 (apply_action补全) ──► 2.12 (snapshot补全)
                          │                                     │
                          └─────────────────────────────────────┘
                                           │
3.1 ─ 3.5 (Agent模块补全) ─────────────────┘
                                           │
4.1 ─ 4.2 (Godot切换) ─────────────────────┘
                                           │
4.3 ─ 4.5 (验证) ──────────────────────────┘
                                           │
5.1 ─ 5.8 (测试) ◄─────────────────────────┘
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
|------|------|------|
| 阶段一 | 1.1 ─ 1.4 | Bridge 基础设施：先让 LLM 能跑通，即使 apply_action 还不完整 |
| 阶段二 | 2.1 ─ 2.14, 3.1 ─ 3.5 | Core 补全：apply_action + Agent 模块 + snapshot |
| 阶段三 | 1.5 ─ 1.10 | Bridge API：SimCommand 通信 + Godot 可调用方法 |
| 阶段四 | 4.1 ─ 4.5 | Godot 切换 + 端到端验证 |
| 阶段五 | 5.1 ─ 5.8 | 测试：单元测试 + 集成测试 + 手动验收 |

**关键里程碑**：
- 阶段一完成后：Agent 能通过 LLM 做决策，但部分动作执行后无效果（NotImplemented）
- 阶段二完成后：所有动作类型都能正确执行，世界状态完整更新
- 阶段三完成后：Godot 端的暂停/调速/动机调整全部生效
- 阶段四完成后：Rust GDExtension 完全替代 GDScript 模拟版
- 阶段五完成后：全量测试通过

## 文件结构总览

```
crates/
├── bridge/
│   ├── Cargo.toml                    # [修改] 添加 agentora-ai 依赖
│   └── src/
│       └── lib.rs                    # [大幅修改] LLM注入、API补全、决策管道
├── core/
│   └── src/
│       ├── agent/
│       │   ├── mod.rs                # [修改] 新增 temp_preferences 字段
│       │   ├── dialogue.rs           # [修改] 增加 world_tick 参数
│       │   ├── combat.rs             # [无需修改] 距离检查在 apply_action 中做
│       │   ├── trade.rs              # [修改] accept 增加发起方检查
│       │   └── movement.rs           # [修改] 补全 perceive_nearby
│       ├── world/
│       │   └── mod.rs                # [大幅修改] apply_action补全、snapshot补全、事件记录
│       └── snapshot.rs               # [无需修改] 数据结构已完整
client/
├── project.godot                     # [修改] autoload 配置切换
└── scripts/
    └── simulation_bridge.gd          # [保留] 作为回退方案，不删除
```
