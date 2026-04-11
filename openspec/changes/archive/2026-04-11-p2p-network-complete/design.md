## Context

当前 P2P 网络实现状态：
- `Libp2pTransport` 已实现完整的 Swarm 事件循环
- `publish/subscribe/connect_to_seed` 已实现
- `RegionTopicManager` 有订阅/退订完整逻辑
- libp2p Swarm 配置和事件循环已完成
- Circuit Relay v2 NAT 穿透已实现

MVP 验证需求：2 个节点各跑 2-3 个 Agent，30 秒内建立连接，事件正确广播，Agent 跨节点可见。

## Goals / Non-Goals

**Goals:**
- 实现完整的 libp2p Swarm 事件循环
- 实现 GossipSub 发布/订阅
- 实现 KAD DHT 节点发现
- 实现 Circuit Relay v2 NAT 穿透
- 实现区域 topic 订阅/退订
- 实现 PeerId 密钥持久化

**Non-Goals:**
- WebRTC 传输（后续优化）
- 复杂路由协议
- 节点声誉系统

## Decisions

### Decision 1: libp2p Swarm 配置

**实现方式**: 手动构建 Swarm，使用 `Swarm::new()` 直接创建

```rust
let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
    .upgrade(libp2p::core::upgrade::Version::V1Lazy)
    .authenticate(noise::Config::new(&local_key)?)
    .multiplex(yamux::Config::default())
    .boxed();

let swarm = Swarm::new(transport, behaviour, local_peer_id, libp2p::swarm::Config::with_tokio_executor());
```

**理由**: libp2p 0.54 的 SwarmBuilder API 较为复杂，手动构建更灵活。使用 TCP + Noise + Yamux 传输栈。

### Decision 2: GossipSub 配置

- 消息验证：签名验证（ed25519）
- 订阅：按区域 topic（region_0, region_1, ...）
- 广播：CRDT 操作序列化为 JSON
- 消息 ID 函数：SHA256 哈希

**理由**: 与设计文档一致，区域订阅减少无关消息

### Decision 3: KAD DHT 引导

- 使用自定义协议名称 `/agentora/kad/1.0.0`
- 通过 `add_peer_address()` 手动添加种子节点
- 触发 `get_closest_peers()` 查询建立连接

**理由**: 种子节点引导，KAD 自动发现邻近节点

### Decision 4: Circuit Relay v2 NAT 穿透

**实现方式**: 使用 libp2p-relay 0.18 客户端 API

```rust
let (_relay_transport, relay_client) = relay::client::new(local_peer_id);
```

- 请求 reservation：`request_reservation(relay_peer_id, relay_addr)`
- 通过中继连接：`connect_via_relay(relay_addr)`
- 电路地址格式：`/ip4/RELAY_IP/tcp/PORT/p2p/RELAY_PEER/p2p-circuit/p2p/TARGET_PEER`

**理由**: NAT 穿透是 P2P 联机的关键。libp2p-relay 0.18 的 client API 相对简单，通过 reservation 机制实现。

### Decision 5: NetworkBehaviour derive 宏

使用 `libp2p-swarm-derive` crate 的 `#[derive(NetworkBehaviour)]` 宏：

```rust
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "AgentoraBehaviourEvent")]
pub struct AgentoraBehaviour {
    #[behaviour(to_event = "AgentoraBehaviourEvent::Gossipsub")]
    pub gossipsub: gossipsub::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Kademlia")]
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    #[behaviour(to_event = "AgentoraBehaviourEvent::RelayClient")]
    pub relay_client: relay::client::Behaviour,
    pub ping: libp2p_ping::Behaviour,
    pub identify: libp2p_identify::Behaviour,
}
```

## Risks / Trade-offs

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| libp2p API 变化 | 版本升级可能需要调整 | 已验证 libp2p 0.54 + libp2p-relay 0.18 |
| TCP 防火墙阻断 | 部分网络无法直连 | 使用 Circuit Relay v2 作为备选 |
| 中继节点带宽限制 | 消息延迟增加 | 优先直连，中继作为备选 |

## Migration Plan

### 部署步骤

1. ✅ 添加 libp2p 依赖和各协议模块
2. ✅ 实现 Swarm 事件循环
3. ✅ 实现 GossipSub 发布/订阅
4. ✅ 实现 KAD DHT 节点发现
5. ✅ 实现 Circuit Relay v2
6. ⏳ 运行多节点测试验证联机（需手动配置）

### 回滚策略

- git tag 标记当前状态
- 若 P2P 失败，回退到单节点模式

## Open Questions

- [ ] GossipSub 参数调优（fanout 等）- 需实际测试
- [ ] 中继节点部署方式 - 需公网服务器
