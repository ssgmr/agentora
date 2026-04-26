# Simulation Orchestrator - P2P 模式接入

## Purpose

SimulationBridge 根据 sim.toml 配置选择 P2P 或中心化模式，正确创建 Simulation 实例并暴露 P2P 控制接口。

## MODIFIED Requirements

### Requirement: Simulation 实例化

SimulationBridge 根据 sim.toml `[p2p]` section 的 `mode` 字段选择构造函数。

#### Scenario: P2P 模式启动

- **WHEN** sim.toml 包含 `[p2p]` section 且 `mode = "p2p"`
- **AND** Godot 调用 `start_simulation()`
- **THEN** simulation_runner SHALL 加载 P2P 配置（port、seed_peer）
- **AND** 创建 `Simulation::with_p2p()` 实例，传入 local_peer_id
- **AND** simulation.start() SHALL 触发 `init_p2p_network()` 连接种子节点

#### Scenario: 中心化模式启动（默认）

- **WHEN** sim.toml 不包含 `[p2p]` section 或 `mode = "centralized"`
- **AND** Godot 调用 `start_simulation()`
- **THEN** simulation_runner SHALL 创建 `Simulation::new()` 实例
- **AND** 行为与当前一致，不创建 libp2p 传输层

### Requirement: SimCommand 扩展

SimCommand 枚举 SHALL 新增 P2P 控制命令。

#### Scenario: ConnectToSeed 命令

- **WHEN** Godot 调用 `connect_to_seed(addr)`
- **THEN** Bridge SHALL 发送 `SimCommand::ConnectToSeed { addr }` 到 simulation 线程
- **AND** simulation 线程 SHALL 调用 transport.connect_to_seed(addr)

#### Scenario: QueryPeerInfo 命令

- **WHEN** Godot 调用 `get_connected_peers()` 或 `get_peer_id()`
- **THEN** Bridge SHALL 通过同步查询机制获取 P2P 状态
- **AND** 返回 JSON 格式结果到 Godot
