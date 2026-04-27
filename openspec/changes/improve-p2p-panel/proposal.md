# 需求说明书

## 背景概述

当前 P2P 面板显示的"已连接 peers"实际上是 Relay Reservations（中继预留列表），代表的是中继服务器的连接状态，而非真正连接的玩家节点。问题根源在于 Swarm 层的 `ConnectionEstablished` 和 `ConnectionClosed` 事件只打印日志，没有存储到共享状态供 UI 查询。

此外，面板缺少关键信息：节点类型区分（玩家 vs 中继）、连接方式（直连/中继/打洞）、agent_version、连接时间、订阅的 topic 列表等。用户难以了解真实的网络连接状态。

## 变更目标

- 目标1：显示真正的已连接节点列表，而非中继预留
- 目标2：区分节点类型（玩家节点 vs 中继服务器）
- 目标3：显示每个连接的详细信息：PeerId（完整）、agent_version、连接方式、连接时间
- 目标4：显示当前订阅的 GossipSub topic 列表
- 目标5：优化 NAT 状态显示，增加探测进度提示

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `connected-peers-tracking` | 在 Swarm 层跟踪 ConnectionEstablished/ConnectionClosed 事件，存储到共享状态 |
| `peer-info-query` | 新增 Libp2pTransport.get_connected_peers() 方法返回真正的已连接节点列表 |
| `topic-subscription-tracking` | 跟踪 GossipSub 订阅/退订事件，存储订阅的 topic 列表 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `p2p-panel-ui` | P2P 面板 UI 改进：显示完整 PeerId、节点类型、连接方式、agent_version、连接时间、订阅 topic 列表 |

## 影响范围

- **代码模块**：
  - `crates/network/src/swarm.rs` — 处理 ConnectionEstablished/Closed 事件
  - `crates/network/src/libp2p_transport.rs` — 添加 connected_peers 字段和查询方法
  - `crates/network/src/config.rs` — 新增 ConnectedPeer 结构体
  - `crates/bridge/src/simulation_runner.rs` — "peers" 查询改为调用新方法
  - `client/scripts/p2p_panel.gd` — UI 显示优化
- **API接口**：
  - `Libp2pTransport::get_connected_peers()` — 新增方法
  - `Libp2pTransport::get_subscribed_topics()` — 新增方法
  - `SimulationBridge::get_connected_peers()` — 返回值改为真正的 peers
- **依赖组件**：无新增依赖，使用现有 libp2p Identify 协议获取 agent_version
- **关联系统**：无

## 验收标准

- [ ] P2P 面板显示真正的已连接节点（通过 ConnectionEstablished 事件跟踪）
- [ ] 每个节点显示：完整 PeerId、agent_version、连接方式（Direct/Relay/Dcutr）、连接时间
- [ ] 区分节点类型：玩家节点（agent_version 包含 "agentora"）vs 中继服务器
- [ ] 显示当前订阅的 topic 列表（world_events, region_0 等）
- [ ] NAT 状态显示优化：探测中显示进度，完成后显示具体地址
- [ ] 连接断开时自动从列表移除（ConnectionClosed 事件处理）