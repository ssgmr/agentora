# Rust Bridge 集成

## Purpose

定义通过 Rust GDExtension 机制替代纯 GDScript 模拟版的完整集成规范，包括 GDExtension 加载、mpsc 通道通信和 SimCommand 命令处理。

## Requirements

### Requirement: Rust GDExtension 加载

Godot 客户端 SHALL 通过 GDExtension 机制加载 Rust 编译的 SimulationBridge 动态库，替代当前的纯 GDScript 模拟版。

#### Scenario: 启动时加载 GDExtension

- **WHEN** Godot 主场景加载完成
- **THEN** 系统 SHALL 通过 `agentora_bridge.gdextension` 配置文件加载 `bin/agentora_bridge.dll`
- **AND** SimulationBridge 节点 SHALL 作为 Rust GDExtension 类实例化
- **AND** `ready()` 方法 SHALL 触发 `start_simulation()` 启动后台模拟线程

#### Scenario: GDExtension 加载失败回退

- **WHEN** GDExtension DLL 文件不存在或版本不兼容
- **THEN** 系统 SHALL 回退至 GDScript 模拟版 `res://scripts/simulation_bridge.gd`
- **AND** 系统 SHALL 在控制台输出错误日志
- **AND** 模拟 SHALL 继续运行（功能降级）

#### Scenario: autoload 配置切换

- **WHEN** 项目配置 `project.godot` 中 autoload 指向 GDExtension 路径
- **THEN** SimulationBridge SHALL 为 Rust 实现
- **AND** 不再加载 `res://scripts/simulation_bridge.gd`

### Requirement: mpsc 通道通信

SimulationBridge SHALL 使用 Rust `std::mpsc` 通道在模拟线程和 Godot 主线程之间传递 `WorldSnapshot`。

#### Scenario: 模拟线程发送快照

- **WHEN** 模拟 tick 完成
- **THEN** 模拟线程 SHALL 通过 `Sender<WorldSnapshot>` 发送快照
- **AND** 快照 SHALL 包含当前 tick 编号、Agent 状态列表、事件列表

#### Scenario: Godot 主线程消费快照

- **WHEN** `physics_process()` 被调用
- **THEN** SimulationBridge SHALL 通过 `Receiver<WorldSnapshot>::try_recv()` 非阻塞轮询
- **AND** 收到快照 SHALL 触发 `world_updated` 信号
- **AND** 未收到快照 SHALL 不触发信号

#### Scenario: 模拟线程退出检测

- **WHEN** `tx.send()` 返回 `Err`（Godot 侧已断开）
- **THEN** 模拟线程 SHALL 退出循环
- **AND** Tokio 运行时 SHALL 优雅关闭

### Requirement: SimCommand 命令处理

SimulationBridge SHALL 通过 mpsc 通道接收 Godot 主线程发送的 `SimCommand`，控制模拟行为。

#### Scenario: 暂停/恢复命令

- **WHEN** Godot 调用 `toggle_pause()` 方法
- **THEN** SimulationBridge SHALL 发送 `SimCommand::Pause` 或 `SimCommand::Start` 至命令通道
- **AND** 模拟线程 SHALL 收到命令后切换暂停/运行状态

#### Scenario: Tick 间隔设置

- **WHEN** Godot 调用 `set_tick_interval(seconds)` 方法
- **THEN** SimulationBridge SHALL 发送 `SimCommand::SetTickInterval { seconds }` 至命令通道
- **AND** 模拟线程 SHALL 更新 tick 间的等待时间

#### Scenario: 动机调整命令

- **WHEN** Godot 调用 `adjust_motivation(agent_id, dimension, value)` 方法
- **THEN** SimulationBridge SHALL 发送 `SimCommand::AdjustMotivation` 至命令通道
- **AND** 模拟线程 SHALL 更新对应 Agent 的动机维度值

#### Scenario: 偏好注入命令

- **WHEN** Godot 调用 `inject_preference(agent_id, dimension, boost, duration_ticks)` 方法
- **THEN** SimulationBridge SHALL 发送 `SimCommand::InjectPreference` 至命令通道
- **AND** 模拟线程 SHALL 在指定 tick 数内提升对应维度权重
