# Bridge P2P API Spec

## Purpose

为 SimulationBridge GDExtension 暴露 P2P 连接管理和状态查询能力，使 Godot 客户端能连接种子节点、查看 peer 信息、接收 P2P 事件。

## ADDED Requirements

### Requirement: P2P 连接方法

SimulationBridge SHALL 提供 `connect_to_seed(addr: String)` 方法，供 Godot 端调用连接种子节点。

#### Scenario: 成功连接种子节点

- **WHEN** Godot 调用 `connect_to_seed("/ip4/127.0.0.1/tcp/4001")`
- **THEN** 系统 SHALL 通过 libp2p 拨号到种子节点
- **AND** 连接成功后 SHALL 发射 `peer_connected` 信号，参数为种子节点的 peer_id
- **AND** 系统 SHALL 执行 Kademlia 发现查询以发现更多 peers

#### Scenario: 连接种子节点失败

- **WHEN** Godot 调用 `connect_to_seed` 但目标不可达
- **THEN** 系统 SHALL 记录错误日志
- **AND** 系统 SHALL 发射 `p2p_status_changed` 信号，status 包含 `"error"` 字段

#### Scenario: 未启用 P2P 模式下调用

- **WHEN** 模拟运行在 Centralized 模式下
- **AND** Godot 调用 `connect_to_seed`
- **THEN** 系统 SHALL 返回错误（Godot 端可捕获）或静默忽略并打印警告日志

### Requirement: Peer ID 查询

SimulationBridge SHALL 提供 `get_peer_id() -> String` 方法，返回本地节点的 peer_id。

#### Scenario: P2P 模式下查询

- **WHEN** Godot 调用 `get_peer_id()`
- **AND** 模拟运行在 P2P 模式下
- **THEN** 系统 SHALL 返回本地 ed25519 密钥对应的 peer_id 字符串

#### Scenario: 中心化模式下查询

- **WHEN** Godot 调用 `get_peer_id()`
- **AND** 模拟运行在 Centralized 模式下
- **THEN** 系统 SHALL 返回空字符串 `""`

### Requirement: 已连接 Peers 查询

SimulationBridge SHALL 提供 `get_connected_peers() -> Array` 方法，返回已连接 peers 的信息列表。

#### Scenario: 查询已连接 peers

- **WHEN** Godot 调用 `get_connected_peers()`
- **THEN** 系统 SHALL 返回 Array，每个元素为 Dictionary：`{peer_id: String, connection_type: String, address: String}`
- **AND** connection_type SHALL 为 `"direct"`、`"dcutr"` 或 `"relay"` 之一

#### Scenario: 无连接时查询

- **WHEN** Godot 调用 `get_connected_peers()`
- **AND** 当前无已连接 peers
- **THEN** 系统 SHALL 返回空数组 `[]`

### Requirement: NAT 状态查询

SimulationBridge SHALL 提供 `get_nat_status() -> Dictionary` 方法，返回当前 NAT 状态。

#### Scenario: 查询 NAT 状态

- **WHEN** Godot 调用 `get_nat_status()`
- **THEN** 系统 SHALL 返回 Dictionary：`{status: String, address: String}`
- **AND** status SHALL 为 `"public"`、`"private"` 或 `"unknown"` 之一
- **AND** 当 status 为 `"public"` 时，address SHALL 包含观察到的公网地址

### Requirement: peer_connected 信号

SimulationBridge SHALL 发射 `peer_connected(peer_id: String)` 信号，当新 peer 连接建立时触发。

#### Scenario: 新 peer 连接

- **WHEN** 通过 Kademlia 发现或手动连接建立新 peer 连接
- **THEN** 系统 SHALL 发射 `peer_connected` 信号
- **AND** 参数 peer_id SHALL 为远端节点的唯一标识

### Requirement: p2p_status_changed 信号

SimulationBridge SHALL 发射 `p2p_status_changed(status: Dictionary)` 信号，当 P2P 网络状态变化时触发。

#### Scenario: NAT 状态变更

- **WHEN** AutoNAT 探测结果变化
- **THEN** 系统 SHALL 发射 `p2p_status_changed` 信号
- **AND** status Dictionary SHALL 包含：`{nat_status: String, peer_count: int, error: String}`

#### Scenario: Peer 断开

- **WHEN** 已连接 peer 断开连接
- **THEN** 系统 SHALL 发射 `p2p_status_changed` 信号
- **AND** peer_count SHALL 减少

### Requirement: SimCommand 扩展

SimCommand 枚举 SHALL 新增 P2P 相关命令变体。

#### Scenario: ConnectToSeed 命令

- **WHEN** Godot 调用 `connect_to_seed(addr)`
- **THEN** 系统 SHALL 发送 `SimCommand::ConnectToSeed { addr }` 到 simulation 线程

#### Scenario: QueryPeerInfo 命令

- **WHEN** Godot 调用 `get_connected_peers()` 或 `get_nat_status()`
- **THEN** 系统 SHALL 发送 `SimCommand::QueryPeerInfo { query_type: String, response_tx: Sender<String> }` 到 simulation 线程
- **AND** simulation 线程 SHALL 通过 response_tx 返回 JSON 格式的查询结果
