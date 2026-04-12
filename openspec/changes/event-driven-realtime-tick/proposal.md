# 需求说明书

## 背景概述

当前MVP采用 World-driven 的 tick 模型：模拟线程以固定间隔遍历所有 Agent，顺序等待每个 Agent 的决策（含 LLM 调用）完成后，才将完整 WorldSnapshot 推送给 Godot 客户端。这种设计存在两个问题：

1. **无法实时渲染反馈**：玩家的 Agent 决策完成后不能立即在 Godot 中看到效果，必须等其他所有 Agent 都执行完才能收到 snapshot。
2. **与目标架构不兼容**：最终产品是 P2P 分布式架构，每个玩家只管自己的 Agent，通过 P2P 异步同步世界状态。不存在"所有 Agent 执行完"这个概念，当前的 World-driven 模型需要重构为 Agent 独立心跳 + 事件驱动推送。

此外，Godot 客户端启动时出现大量纹理加载错误（`.godot/imported/` 缓存损坏导致所有 PNG 纹理加载失败，回退到纯色渲染），需要修复资源导入问题。

## 变更目标

- **Agent 独立心跳**：每个 Agent 拥有独立的决策循环，不再等待其他 Agent，决策完成后立即通过事件通道推送 Godot
- **增量渲染**：Godot 客户端从"全量 snapshot 替换"改为"增量 delta 更新"，收到 Agent 状态变化后立即更新对应 sprite
- **为 P2P 架构铺路**：Agent 作为独立决策单元的模型天然映射到未来"每个玩家管自己的 Agent + P2P 同步他人状态"的架构
- **修复纹理资源**：清理损坏的 Godot 导入缓存，通过 SVG 重新生成 PNG 确保资源正确加载
- **NPC 快速验证**：保留 NPC 用于开发验证阶段，使用规则引擎快速决策（不调 LLM）

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `agent-event-loop` | Agent 独立心跳循环，每个 Agent 异步决策，完成后通过 mpsc 通道发送 delta 事件 |
| `incremental-rendering` | Godot 端增量更新：接收 AgentDelta 事件后立即更新对应 Agent 的位置/状态/动画 |
| `npc-fast-loop` | NPC 使用规则引擎快速决策（跳过 LLM），用于开发验证阶段的世界活跃度验证 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `simulation-bridge` | 从单通道 snapshot 模式改为多通道事件流模式：增加 AgentDelta 通道，保留 snapshot 通道用于一致性检查/存档 |
| `agent-manager` | 从"收到 snapshot 后清空重建"改为"收到 delta 后增量更新单个 Agent sprite" |

## 影响范围

- **代码模块**：
  - `crates/bridge/src/lib.rs` — 模拟循环重构，增加 AgentDelta 通道
  - `crates/core/src/agent/mod.rs` — Agent 独立决策循环
  - `client/scripts/agent_manager.gd` — 增量更新逻辑
  - `client/scripts/simulation_bridge.gd` — 处理新增的 delta 事件类型
- **资源文件**：
  - `client/assets/sprites/*.svg` — SVG 源文件
  - `client/assets/sprites/*.png` / `client/assets/textures/*.png` — 重新生成
  - `client/.godot/imported/` — 清理并重新导入
- **依赖组件**：cairosvg（SVG 转 PNG 工具）

## 验收标准

- [ ] 删除 `.godot/imported/` 缓存后，Godot 启动无纹理加载错误
- [ ] 通过 `svg_to_png.py` 从 SVG 重新生成所有 PNG 资源，纹理正确显示
- [ ] 每个 Agent 独立决策循环，决策完成后立即通过通道推送 delta 事件
- [ ] Godot 端收到 AgentDelta 后，对应 Agent 的位置/状态在渲染中立即更新（不等其他 Agent）
- [ ] NPC 使用规则引擎快速决策，不阻塞玩家 Agent
- [ ] 保持向后兼容：snapshot 通道仍然工作（用于存档/一致性检查）
