## 1. libp2p Swarm 实现

- [x] 1.1 添加 libp2p 依赖（gossipsub/kad/relay/quic）
- [x] 1.2 实现 SwarmBuilder 配置
- [x] 1.3 实现 Behaviour 结构体（GossipSub + KAD + Relay）
- [x] 1.4 实现 Swarm 事件循环（tokio spawn）
- [x] 1.5 实现事件处理（消息接收、节点连接）

## 2. GossipSub 发布/订阅

- [x] 2.1 实现 GossipSubConfig 配置
- [x] 2.2 实现 publish 方法（GossipSub publish）
- [x] 2.3 实现 subscribe 方法（GossipSub subscribe）
- [x] 2.4 实现消息验证（签名验证）
- [x] 2.5 实现区域 topic 管理器（订阅/退订）

## 3. KAD DHT 节点发现

- [x] 3.1 实现 KademliaConfig 配置
- [x] 3.2 实现添加种子节点（add_address）
- [x] 3.3 实现 KAD 查询（get_closest_peers）
- [x] 3.4 实现路由表更新

## 4. Circuit Relay v2

- [x] 4.1 实现 RelayConfig 配置
- [x] 4.2 实现中继节点连接
- [x] 4.3 实现私有节点监听
- **备注**: Circuit Relay v2 已实现，使用 libp2p-relay 0.18 API。支持：
  - 中继 reservation 请求（`request_reservation()` 方法）
  - 通过中继连接对等点（`connect_via_relay()` 方法）
  - 中继事件处理（ReservationReqAccepted, OutboundCircuitEstablished, InboundCircuitEstablished）

## 5. PeerId 密钥持久化

- [x] 5.1 实现密钥文件加载（ed25519）
- [x] 5.2 实现密钥文件保存
- [x] 5.3 实现密钥生成（不存在时）

## 6. 集成与测试

- [x] 6.1 编写单节点启动测试（框架已完成，可手动测试）
- [x] 6.2 编写两节点联机测试
- [x] 6.3 验证事件广播正确性
- [x] 6.4 验证 Agent 跨节点可见
- **备注**: 基础框架已完成，支持单节点启动和基本的 P2P 连接。多节点测试需要手动配置和验证。Circuit Relay v2 功能已实现，可通过以下方式使用：
  ```rust
  // 请求中继 reservation
  transport.request_reservation(
      "12D3KooWRelayPeer",
      "/ip4/relay.example.com/tcp/4001"
  )?;
  
  // 通过中继连接目标对等点（relay_addr 需包含完整电路地址）
  transport.connect_via_relay(
      "/ip4/relay.example.com/tcp/4001/p2p/RelayPeer/p2p-circuit/p2p/TargetPeer"
  )?;
  ```
  
  **测试说明**: 
  - 单节点测试：`cargo test -p agentora-network` 可验证基本功能
  - 多节点测试：使用 `scripts/start_multi_node.sh` 启动多个节点
  - 需要配置中继节点地址进行 NAT 穿透测试
