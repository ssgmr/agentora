# 功能规格说明 - Bridge 职责收缩

## Purpose

定义 SimulationBridge 作为纯前端桥接层的职责边界，不创建 runtime、不实现模拟逻辑，只负责信号转发和命令传递。

## Requirements

### Requirement: Bridge 作为纯前端桥接层

SimulationBridge SHALL 只负责以下职责：
- 持有 Simulation 实例的通道 Sender/Receiver
- physics_process() 中接收 snapshot/delta/narrative 并发射 Godot 信号
- 提供 GDScript 可调用的 API（get_tick, get_agent_data, inject_preference）
- 命令转发（pause/start/set_tick_interval → SimCommand channel）

SimulationBridge SHALL **不负责**：
- ~~创建 tokio runtime~~ → 由外部创建或使用全局 runtime
- ~~初始化 LLM Provider~~ → 由 Simulation 内部处理
- ~~加载配置~~ → 由 Simulation 内部处理
- ~~实现模拟逻辑~~ → 完全在 Simulation 模块

#### Scenario: Bridge 不创建 runtime

- **WHEN** Bridge.start_simulation() 被调用
- **THEN** 创建 channel
- **AND** std::thread::spawn 创建模拟线程（线程内创建 tokio runtime）
- **AND** 不在 Bridge 内创建全局 runtime

#### Scenario: Bridge 只发射信号

- **WHEN** physics_process() 执行
- **THEN** 从 receiver.try_recv() 获取 snapshot/delta/narrative
- **AND** 发射 world_updated/agent_delta/narrative_event 信号
- **AND** 不执行任何模拟逻辑

### Requirement: Bridge 行数限制

SimulationBridge (bridge.rs) SHALL 保持轻量：
- 主要代码 SHALL < 200 行
- 类型转换 SHALL 在 conversion.rs
- 日志初始化 SHALL 在 logging.rs

#### Scenario: Bridge 模块文件结构

- **WHEN** 重构后
- **THEN** bridge 模块包含：
  - lib.rs (入口，<30行)
  - bridge.rs (SimulationBridge 定义，<200行)
  - conversion.rs (类型转换)
  - logging.rs (日志配置)
- **AND** 不包含模拟逻辑
