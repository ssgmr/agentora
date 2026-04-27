# 功能规格说明

## ADDED Requirements

### Requirement: Swarm跟踪连接事件

Swarm 事件循环 SHALL 跟踪 `ConnectionEstablished` 和 `ConnectionClosed` 事件，并将连接状态存储到共享的 `connected_peers` 列表中。

#### Scenario: 新连接建立时添加到列表

- **WHEN** Swarm 接收到 `ConnectionEstablished { peer_id, endpoint, .. }` 事件
- **THEN** 系统创建 `ConnectedPeer` 结构体包含 peer_id、连接时间、endpoint 类型
- **AND** 将该结构体添加到 `connected_peers` 共享状态中
- **AND** 记录日志 "连接建立: peer_id, endpoint"

#### Scenario: 连接关闭时从列表移除

- **WHEN** Swarm 接收到 `ConnectionClosed { peer_id, cause, .. }` 事件
- **THEN** 系统从 `connected_peers` 共享状态中移除该 peer_id 的记录
- **AND** 记录日志 "连接关闭: peer_id, cause"

#### Scenario: Identify 协议获取 agent_version

- **WHEN** Swarm 接收到 `Identify::Received { peer_id, info }` 事件
- **THEN** 系统更新 `connected_peers` 中该 peer_id 的 `agent_version` 字段
- **AND** 根据 agent_version 是否包含 "agentora" 判断节点类型（玩家 vs 中继）

### Requirement: 连接信息结构体

系统 SHALL 定义 `ConnectedPeer` 结构体存储每个连接的详细信息。

#### Scenario: 结构体字段完整性

- **WHEN** 创建 `ConnectedPeer` 结构体
- **THEN** 结构体 MUST 包含以下字段：
  - `peer_id`: String — libp2p PeerId（完整46字符）
  - `agent_version`: String — 来自 Identify 协议的 agent 版本
  - `connection_type`: ConnectionType — 连接方式（Direct/Relay/Dcutr）
  - `connected_at`: DateTime — 连接建立时间
  - `endpoint_type`: String — 连接端点类型（listener/dialer）
  - `is_relay_server`: bool — 是否为中继服务器（根据 agent_version 判断）
  - `listen_addr`: Option<String> — 对方的监听地址（如果已知）

#### Scenario: 节点类型判断规则

- **WHEN** Identify 协议返回 agent_version
- **THEN** 如果 agent_version 包含 "relay" 或 "libp2p-relay" → `is_relay_server = true`
- **AND** 如果 agent_version 包含 "agentora" → `is_relay_server = false`（玩家节点）
- **AND** 其他情况 → `is_relay_server = false`（默认视为玩家节点）