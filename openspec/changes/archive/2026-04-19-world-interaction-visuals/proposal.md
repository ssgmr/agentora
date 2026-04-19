# 需求说明书

## 背景概述

当前 Agentora 客户端的世界交互缺少视觉反馈，玩家难以感知 Agent 的行为和世界状态变化。具体问题包括：
- Agent 采集资源时无视觉效果，玩家不知道 Agent 正在做什么
- 资源点只通过透明度暗示数量，不直观，玩家无法判断资源丰富度
- AgentDetailPanel 状态信息不完整，缺少当前动作、动作结果、等级等关键信息

这些问题降低了游戏的可观察性和玩家沉浸感。本次变更旨在通过简单视觉效果增强世界交互的可见性。

## 变更目标

- 目标1：资源点显示具体数量标签，让玩家直观了解资源储量
- 目标2：Agent 执行采集动作时产生闪烁效果，提示当前行为
- 目标3：AgentDetailPanel 显示完整状态信息（动作、结果、等级）
- 目标4：采用简单实现方式，复用现有代码模式，避免复杂动画系统

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `resource-label-display` | 资源数量标签显示：在世界地图资源点旁绘制具体数量文本 |
| `agent-action-flash` | Agent 动作闪烁效果：Agent 执行采集动作时产生短暂闪烁/脉动效果 |
| `agent-status-enhance` | Agent 状态面板增强：AgentDetailPanel 显示当前动作、动作结果、等级信息 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `godot-rendering` | 扩展 `_draw_resources()` 方法，添加数量标签绘制逻辑 |

## 影响范围

- **代码模块**：
  - `client/scripts/world_renderer.gd` — 资源绘制和数量标签
  - `client/scripts/agent_manager.gd` — Agent 闪烁效果系统
  - `client/scripts/agent_detail_panel.gd` — 状态面板扩展

- **API接口**：无新增接口，复用现有 `SimulationBridge.get_agent_data()` 和 delta 信号

- **依赖组件**：无新增外部依赖，使用 Godot 内置 `draw_string()` 和 `ThemeDB.fallback_font`

- **关联系统**：Rust Bridge 的 delta 推送机制（已有，无需修改）

## 验收标准

- [ ] 资源点显示具体数量数字标签（如 "50"、"120"）
- [ ] Agent 采集时产生绿色闪烁效果（约 0.3 秒）
- [ ] AgentDetailPanel 显示当前动作、动作结果、等级
- [ ] 截图验证视觉效果正常渲染