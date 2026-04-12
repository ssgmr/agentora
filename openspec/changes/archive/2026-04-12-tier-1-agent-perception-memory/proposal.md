# Tier 1: Agent 感知与记忆接线

## 问题

当前 Agent 的决策链路存在三个关键断点，导致 Agent 处于"失明的健忘症患者在梦游"状态：

1. **视野扫描 Bug** — `crates/bridge/src/lib.rs` 中的 vision_radius=5 扫描只遍历东北1/4象限（`saturating_add` 导致 dx/dy 永远为正），Agent 实际只能看到右上方的资源和其他 Agent
2. **感知数据不完整** — `perceive_nearby()` 在 `movement.rs` 中定义了完整类型但从未被调用，Agent 对其他 Agent 的感知只有数量，没有位置/名字/关系
3. **记忆系统断线** — `memory.record()` 在整个代码库中零调用，`memory.get_summary()` 也从未被写入决策 Prompt，Agent 每次决策都是空白开局
4. **关系数据不暴露** — Agent 有 `relations` 字段且 Attack/Ally 后正确更新，但决策 Prompt 中不包含任何关系信息

## 目标

让 Agent 在决策时拥有：
- **完整的周边环境感知** — 正确的圆形视野扫描，包含资源（位置+类型+数量）、其他 Agent（位置+名字+动机摘要）、地形信息
- **短期记忆** — 每次行动后自动记录到记忆系统，下次决策时注入最近几次行动的摘要
- **关系上下文** — 决策 Prompt 中包含对周围已知 Agent 的关系状态（朋友/敌人/陌生人、信任值）

## 范围

**包含：**
- 修复 vision_radius 扫描逻辑（-radius 到 +radius）
- 将 `perceive_nearby()` 接入 WorldState 构建流程
- 在 `build_perception_summary()` 中包含地形信息和其他 Agent 详细信息
- 每次 `apply_action` 后调用 `memory.record()`
- 在决策 Prompt 中注入 memory summary
- 在决策 Prompt 中包含关系信息

**不包含：**
- 策略系统接入（Tier 3）
- Build/Gather 的真实副作用（Tier 2）
- 长期记忆 Chronicle 摘要（Tier 3）
- 视线遮挡/地形阻挡（未来增强）

## 影响

- `crates/bridge/src/lib.rs` — 修复 vision 扫描、接入 perceive_nearby
- `crates/core/src/agent/movement.rs` — 完善 perceive_nearby 的 Agent 探测逻辑
- `crates/core/src/decision.rs` — build_perception_summary 扩展、memory 接入
- `crates/core/src/world/mod.rs` — apply_action 后调用 memory.record
- `crates/core/src/prompt.rs` — Prompt 模板增加关系和记忆段
