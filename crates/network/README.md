# agentora-network

P2P 网络层 — libp2p 集成，GossipSub 广播 + KAD DHT + DCUtR 打洞 + AutoNAT 探测 + Relay 中继。

## 架构

```
crates/network/src/
├── lib.rs              # 公共 API 入口
├── transport.rs        # Transport trait + MessageHandler + TransportError
├── libp2p_transport.rs # libp2p 完整实现（Swarm + 行为组合 + 混合穿透策略）
├── codec.rs            # CRDT 操作序列化（CrdtOp / NetworkMessage）
└── gossip.rs           # 区域主题管理器（RegionTopicManager）
```

## 核心组件

### Transport Trait

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn publish(&self, topic: &str, data: &[u8]) -> Result<(), TransportError>;
    async fn subscribe(&self, topic: &str, handler: Box<dyn MessageHandler>) -> Result<(), TransportError>;
    async fn unsubscribe(&self, topic: &str) -> Result<(), TransportError>;
    fn peer_id(&self) -> &PeerId;
    async fn connect_to_seed(&self, addr: &str) -> Result<(), TransportError>;
}
```

### Libp2pTransport

主要的 libp2p 实现，包含以下行为：

| 行为 | 协议 | 用途 |
|------|------|------|
| GossipSub | 发布/订阅 | 区域广播 |
| Kademlia | DHT | 对等点发现 |
| Relay Client | 电路中继 | NAT 穿透保底 |
| DCUtR | 打洞升级 | 中继后建立直连 |
| AutoNAT | NAT 探测 | 检测公网/私有网络 |
| Ping | 心跳 | 连接保活 |
| Identify | 身份交换 | 对等点信息发现 |

### NAT 穿透策略

连接优先级：**直连 → DCUtR → Relay**

- 本地监听 `0.0.0.0:随机端口/TCP`
- 直连失败超过阈值 → 通过中继打洞
- 打洞失败 → 纯中继电路

## 配置结构

详见 [NAT 穿透配置指南](docs/nat-traversal-config.md)。

## libp2p 版本

当前使用 **libp2p 0.56**（从 0.54 升级）。详见 [迁移指南](docs/migration-054-to-056.md)。

## 测试

```bash
# 单元测试
cargo test -p agentora-network -- --test-threads=1

# 性能基准测试
cargo test -p agentora-network --test benchmark_tests -- --nocapture

# 多节点集成测试
cargo test -p agentora-network --test multi_node_tests -- --nocapture
```

## 依赖关系

```
agentora-network → agentora-sync (PeerId 类型)
                 → libp2p 生态 (网络协议栈)
                 → tokio (异步运行时)
                 → serde (序列化)
```
