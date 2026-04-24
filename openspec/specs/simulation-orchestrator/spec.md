# 功能规格说明 - Simulation 编排层

## Purpose

定义 Simulation 作为后端核心编排层的职责边界，与 Bridge 通过 Channel 通信，完全与前端解耦。

## Requirements

### Requirement: Simulation 作为后端核心编排层

Simulation SHALL 只负责以下职责：
- 管理 World 和 DecisionPipeline
- 控制 Agent 决策循环（通过 AgentLoopController）
- 推进世界 Tick（通过 TickLoopController）
- 生成 Snapshot（通过 SnapshotLoopController）
- 提供公开 API：start/pause/resume/inject_preference/set_tick_interval

Simulation SHALL **不负责**：
- ~~创建 tokio runtime~~ → Bridge 负责
- ~~直接发射 Godot 信号~~ → 通过 channel 传递给 Bridge
- ~~前端渲染状态管理~~ → 完全与前端解耦

#### Scenario: Simulation 提供 Snapshot Sender

- **WHEN** Simulation::new() 创建
- **THEN** 返回 snapshot_sender() 供 Bridge 克隆
- **AND** 不直接调用 Godot API

#### Scenario: Simulation 接收命令通道

- **WHEN** Bridge 创建 SimCommand 通道
- **THEN** Simulation 通过 cmd_rx 接收命令
- **AND** 不直接监听 Bridge 节点

### Requirement: Simulation 与 Bridge 通信通过 Channel

Simulation 与 Bridge 之间 SHALL 只通过 mpsc channel 通信：
- snapshot_tx / snapshot_rx：完整状态同步
- delta_tx / delta_rx：增量事件推送
- narrative_tx / narrative_rx：叙事事件流
- cmd_tx / cmd_rx：控制命令流

#### Scenario: Bridge 不直接调用 Simulation 方法

- **WHEN** Bridge 需要暂停模拟
- **THEN** 发送 SimCommand::Pause 到 cmd_tx
- **AND** Simulation 在命令循环中处理