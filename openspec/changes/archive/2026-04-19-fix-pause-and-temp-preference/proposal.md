# 需求说明书

## 背景概述

在探索模式下发现三个核心问题：

1. **暂停功能完全不生效**：`run_agent_loop` 是独立 tokio task，不检查 `is_paused` 状态。命令处理循环中的 `is_paused` 只影响自身的 sleep 循环，无法阻止 Agent 决策循环继续运行。

2. **world.tick() 从未被调用**：世界时间不推进，导致：
   - `agent.tick_preferences()` 不执行 → 临时偏好永不衰减
   - `pressure_tick()` 不执行 → 环境压力事件不触发
   - 资源刷新逻辑不执行
   - `world.tick` 计数不推进

3. **临时偏好注入链路完整但可能存在匹配问题**：代码从 GDScript → SimCommand → agent.inject_preference() → WorldState → Prompt 构建路径完整，但需要验证 agent_id 是否正确匹配，以及 prompt 是否正确生成。

这些问题导致玩家无法暂停模拟观察状态，且临时引导注入可能无法正确生效。

## 变更目标

- **目标1**：实现真正的暂停功能，暂停时所有 Agent 决策循环停止运行
- **目标2**：添加世界时间推进机制，确保 `world.tick()` 定期调用
- **目标3**：确保临时偏好注入正确生效，包括 agent_id 匹配和 prompt 生成
- **目标4**：保持暂停状态在恢复后能正确继续，不丢失任何状态

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `pause-control` | 暂停控制机制，让 Agent 决策循环能感知并响应暂停状态 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `simulation-loop` | 添加世界时间推进，定期调用 `world.tick()` |
| `temp-preference` | 确保临时偏好注入链路完整并验证生效 |

## 影响范围

- **代码模块**：
  - `crates/bridge/src/lib.rs`：`run_agent_loop`、`run_simulation`、命令处理循环
  - `crates/core/src/world/mod.rs`：`tick()` 方法

- **API接口**：
  - `SimulationBridge.toggle_pause()`：GDScript 调用
  - `SimulationBridge.inject_preference()`：GDScript 调用

- **依赖组件**：tokio runtime（async task 协调）

- **关联系统**：Godot 客户端（pause button UI）

## 验收标准

- [ ] 点击暂停按钮后，Agent 决策日志停止输出
- [ ] 暂停期间 `world.tick` 计数不推进
- [ ] 恢复后 Agent 决策正常继续
- [ ] 临时偏好注入后，在 LLM Prompt 日志中能看到 `<guidance>` 标签
- [ ] 临时偏好能正确衰减（`remaining_ticks` 递减）
- [ ] 环境压力事件能正确触发和更新