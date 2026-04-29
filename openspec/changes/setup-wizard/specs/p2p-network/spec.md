# 功能规格说明 - P2P 网络层（增量）

## ADDED Requirements

### Requirement: P2P 模式选择逻辑

系统 SHALL 根据用户配置选择是否初始化 P2P 网络。

#### Scenario: 单机模式跳过 P2P

- **WHEN** 配置 p2p.mode = "single"
- **THEN** Simulation SHALL 以 SimMode::Centralized 模式运行
- **AND** 不初始化 Libp2pTransport
- **AND** 所有 Agent 在本地决策

#### Scenario: 创建世界模式初始化

- **WHEN** 配置 p2p.mode = "create"
- **THEN** Simulation SHALL 以 SimMode::P2P 模式运行
- **AND** 启动 Libp2pTransport 作为种子节点
- **AND** 等待其他节点连接

#### Scenario: 加入世界模式连接

- **WHEN** 配置 p2p.mode = "join"
- **AND** 配置 p2p.seed_address 存在
- **THEN** Simulation SHALL 启动 P2P 并连接种子节点
- **AND** 通过 GossipSub 同步世界状态

### Requirement: 本地地址显示

创建世界模式 SHALL 提供本地 P2P 地址供分享。

#### Scenario: 获取本地地址

- **WHEN** 用户选择创建世界模式
- **THEN** Bridge SHALL 调用 transport.peer_id() 和监听地址
- **AND** 组合为完整 P2P 地址字符串
- **AND** 通过信号发送给 Godot 显示

#### Scenario: 地址格式

- **WHEN** 显示本地地址
- **THEN** 格式 SHALL 为：/ip4/<ip>/tcp/<port>/p2p/<peer_id>
- **AND** 自动检测公网/内网地址
- **AND** 优先显示公网地址（如果有）

### Requirement: 连接种子节点

加入世界 SHALL 支持连接指定的种子节点。

#### Scenario: 配置种子地址

- **WHEN** WorldSeed.seed_peers 包含用户输入的地址
- **THEN** Simulation SHALL 调用 transport.connect_to_seed()
- **AND** 尝试建立连接

#### Scenario: 连接成功确认

- **WHEN** 连接种子节点成功
- **THEN** Bridge SHALL 发射 peer_connected 信号
- **AND** Godot SHALL 显示"已连接到种子节点"

#### Scenario: 连接失败处理

- **WHEN** 连接种子节点失败
- **THEN** SHALL 发射 p2p_status_changed 信号
- **AND** 包含错误信息
- **AND** Godot SHALL 显示错误提示并提供重试选项