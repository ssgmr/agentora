# 功能规格说明

## ADDED Requirements

### Requirement: 查询已连接节点列表

Libp2pTransport SHALL 提供 `get_connected_peers()` 方法返回真正的已连接节点列表。

#### Scenario: 返回完整的连接列表

- **WHEN** 调用 `transport.get_connected_peers()`
- **THEN** 返回 `Vec<ConnectedPeer>` 包含所有当前活跃连接
- **AND** 每个 ConnectedPeer 包含 peer_id、agent_version、connection_type、connected_at、is_relay_server

#### Scenario: 没有连接时返回空列表

- **WHEN** 调用 `transport.get_connected_peers()`
- **AND** 当前没有任何活跃连接
- **THEN** 返回空的 `Vec<ConnectedPeer>`（长度为0）

#### Scenario: 区分玩家节点和中继服务器

- **WHEN** UI 请求显示已连接节点
- **THEN** 系统返回的列表中每个节点都有 `is_relay_server` 字段
- **AND** UI 可以根据该字段区分显示（玩家节点 vs 中继服务器）

### Requirement: 查询订阅的 Topic 列表

Libp2pTransport SHALL 提供 `get_subscribed_topics()` 方法返回当前订阅的 GossipSub topic 列表。

#### Scenario: 返回订阅列表

- **WHEN** 调用 `transport.get_subscribed_topics()`
- **THEN** 返回 `Vec<String>` 包含所有已订阅的 topic 名称
- **AND** 至少包含 "world_events" 和 "region_0"（如果已订阅）

#### Scenario: 未订阅时返回空列表

- **WHEN** 调用 `transport.get_subscribed_topics()`
- **AND** 当前没有任何订阅
- **THEN** 返回空的 `Vec<String>`（长度为0）

### Requirement: Bridge API 更新

SimulationBridge SHALL 更新 `get_connected_peers()` 方法的返回值，从 relay_reservations 改为真正的 connected_peers。

#### Scenario: 返回 JSON 格式

- **WHEN** 调用 `bridge.get_connected_peers()`
- **THEN** 返回 JSON 字符串格式：
  ```json
  [
    {
      "peer_id": "12D3Koo...",
      "agent_version": "agentora/1.0.0",
      "connection_type": "direct",
      "connected_at": "2024-01-15T10:23:45Z",
      "is_relay_server": false
    }
  ]
  ```
- **AND** connection_type 值为 "direct" / "relay" / "dcutr"

#### Scenario: 查询响应超时处理

- **WHEN** 调用 `bridge.get_connected_peers()`
- **AND** simulation_runner 响应超时（>1秒）
- **THEN** 返回 `"[]"` 作为默认值（空列表）
- **AND** 记录警告日志