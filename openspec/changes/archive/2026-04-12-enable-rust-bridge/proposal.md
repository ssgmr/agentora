# 需求说明书

## 背景概述

当前 `SimulationBridge` 使用纯 GDScript 模拟版（`res://scripts/simulation_bridge.gd`），Agent 行为完全是随机游走，没有接入 Rust 侧的真实核心引擎。虽然 Rust 侧的 `crates/bridge/` 已实现了 GDExtension 桥接框架、`DecisionPipeline` 架构、LLM Provider 接口等基础设施，但存在以下断点：

1. Godot autoload 加载的是 GDScript 模拟版，Rust GDExtension DLL 虽已编译但从未被加载
2. `DecisionPipeline::new()` 创建的管道 `llm_provider = None`，bridge 未注入任何 Provider
3. `World::apply_action()` 中 Trade/Talk/Attack/Build/Explore 等动作全部走 `NotImplemented` 分支
4. `dialogue.rs` 仅有骨架代码，`combat.rs` 缺少距离检查和死亡处理
5. bridge 的 `adjust_motivation()`、`inject_preference()` 等方法仅打印日志，无实际逻辑
6. `World::snapshot()` 返回的快照中 `map_changes`/`events`/`legacies`/`pressures` 均为空

这导致整个系统处于"框架完整、链路断裂"状态——LLM 可用、决策管道有架构、记忆系统已就绪，但无法端到端运转。

## 变更目标

- **目标1**：将 Godot 客户端的 SimulationBridge 从 GDScript 模拟版切换到 Rust GDExtension 实现
- **目标2**：在 bridge 中注入 LLM Provider，使 Agent 能通过 LLM 进行真实决策
- **目标3**：补全 `World::apply_action()` 中 Trade/Talk/Attack/Build/Explore 等动作的实际执行逻辑
- **目标4**：补全 `dialogue.rs` 对话系统和 `combat.rs` 战斗系统的核心逻辑
- **目标5**：补全 bridge 的 Godot 可调用 API（动机调整、偏好注入、暂停控制等）
- **目标6**：补全 `World::snapshot()` 填充事件/遗产/压力数据，使 Godot 端能正确显示

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `rust-bridge-integration` | Godot 通过 GDExtension 加载 Rust SimulationBridge，通过 mpsc 通道与模拟线程通信，接收 WorldSnapshot 快照 |
| `llm-decision-pipeline` | bridge 启动时注入 LLM Provider（从 config/llm.toml 加载），Agent 每 tick 通过 DecisionPipeline + LLM 进行真实决策 |
| `agent-interaction` | 补全 Trade（双向资源交换）、Dialogue（消息队列+AI回复）、Combat（距离检查+伤害+死亡）、Build（资源消耗+建筑放置）的实际执行逻辑 |
| `world-snapshot-feed` | World::snapshot() 填充 map_changes、events、legacies、pressures 数据，序列化后通过通道发送至 Godot 端 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `bridge-api` | bridge 的 `adjust_motivation()`、`inject_preference()`、`set_tick_interval()`、`toggle_pause()` 等方法从 print 占位改为实际操作 World 状态 |
| `action-execution` | `World::apply_action()` 中所有 ActionType 分支从 `NotImplemented` 改为调用对应的 Agent 交互模块 |

## 影响范围

- **代码模块**：
  - `crates/bridge/src/lib.rs` — GDExtension 桥接核心
  - `crates/bridge/src/snapshot.rs` — WorldSnapshot 序列化
  - `crates/core/src/world/mod.rs` — apply_action()、snapshot()
  - `crates/core/src/agent/dialogue.rs` — 对话系统补全
  - `crates/core/src/agent/combat.rs` — 战斗系统补全
  - `crates/core/src/decision/pipeline.rs` — DecisionPipeline 注入 Provider
  - `client/project.godot` — autoload 从 GDScript 切换到 GDExtension
  - `client/scripts/simulation_bridge.gd` — 简化为仅处理信号接收和 UI 更新
- **API接口**：bridge 的 `#[func]` 方法需要完整实现
- **依赖组件**：需要本地 LLM 服务运行（localhost:1234 或 Anthropic API），`config/llm.toml` 配置
- **关联系统**：memory 模块（决策时需要记忆检索）、strategy 模块（决策时需要策略匹配）

## 验收标准

- [ ] Godot 启动后通过 GDExtension 加载 Rust SimulationBridge（非 GDScript 模拟版）
- [ ] Agent 每 tick 的决策调用 LLM（可在 LLM 日志/日志中观察到实际 API 请求）
- [ ] LLM 不可用时自动降级到规则引擎兜底，Agent 不会卡死
- [ ] Agent 能执行 Move/Gather/Wait/Trade/Talk/Attack/Build 等动作并在世界中产生实际效果
- [ ] Godot 端能看到 Agent 真实移动（非随机游走）、叙事流显示有意义的决策事件
- [ ] 点击 Agent 后动机雷达图显示真实动机值（非硬编码 0.5）
- [ ] 动机滑块调整后能影响 Agent 后续决策行为
- [ ] 战斗系统能正确处理攻击、扣血、死亡、遗产生成
- [ ] 对话系统能记录消息历史并生成 AI 回复
- [ ] 交易系统能正确检查资源充足性并完成双向资源转移
- [ ] Godot 端能暂停/恢复模拟、调整 tick 速度
