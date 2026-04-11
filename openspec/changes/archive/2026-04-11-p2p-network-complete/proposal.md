## Why

P2P 网络当前仅有框架实现：Libp2pTransport 缺少完整的 Swarm 实现，GossipSub 发布/订阅未集成，KAD DHT 节点发现未实现，区域 topic 订阅缺少退订逻辑。这导致多节点无法联机验证"P2P 状态同步"的设计假设。

## What Changes

- **新增** libp2p Swarm 完整实现（GossipSub + KAD DHT + Relay）
- **新增** GossipSub 发布/订阅集成
- **新增** KAD DHT 节点发现
- **新增** Circuit Relay v2 NAT 穿透
- **新增** 区域 topic 退订逻辑
- **新增** PeerId 密钥文件持久化

## Capabilities

### New Capabilities

- `libp2p-swarm-impl`: rust-libp2p Swarm 完整实现，GossipSub + KAD DHT + Circuit Relay v2
- `gossipsub-publish-subscribe`: GossipSub 发布/订阅集成，区域 topic 自动订阅/退订
- `kad-dht-discovery`: KAD DHT 节点发现，种子节点引导连接
- `circuit-relay-v2`: Circuit Relay v2 NAT 穿透，支持私有网络节点连接
- `peerid-key-persistence`: PeerId 密钥文件持久化，加载/保存 ed25519 密钥

### Modified Capabilities

- `region-topic-manager`: 增加退订逻辑，支持动态区域切换

## Impact

- **affected crates**: `network` (libp2p 集成), `sync` (PeerId 类型)
- **dependencies**: `libp2p` (核心), `libp2p-gossipsub`, `libp2p-kad`, `libp2p-relay`
- **breaking changes**: 无，当前网络层为框架实现
- **integration points**: World::apply_action 广播 CRDT 操作；多节点测试验证
