# 迁移指南：libp2p 0.54 → 0.56

## 概述

本次升级将 libp2p 从 0.54 升级到 0.56，并新增 DCUtR 打洞和 AutoNAT 探测功能。

## 依赖版本变更

| Crate | 旧版本 (0.54) | 新版本 (0.56) |
|-------|---------------|---------------|
| libp2p | 0.54 | 0.56 |
| libp2p-gossipsub | 0.47 | 0.49 |
| libp2p-kad | 0.46 | 0.48 |
| libp2p-relay | 0.18 | 0.21 |
| libp2p-ping | 0.45 | 0.47 |
| libp2p-identify | 0.45 | 0.47 |
| libp2p-tcp | 0.42 | 0.44 |
| libp2p-noise | 0.45 | 0.47 |
| libp2p-yamux | 0.46 | 0.48 |
| libp2p-dns | 0.42 | 0.44 |
| libp2p-swarm-derive | 0.35 | 0.36 |

### 新增依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| libp2p-dcutr | 0.14 | 中继辅助的直连升级 |
| libp2p-autonat | 0.15 | NAT 类型探测 |

## API 变更

### 1. Relay Client 初始化

**0.54:**
```rust
let relay_client = relay::client::new(
    local_key.public().to_peer_id(),
    relay_transport.clone(),
);
```

**0.56:**
```rust
let (_relay_transport, relay_client) = relay::client::new(local_key.public().to_peer_id());
```

`client::new()` 现在返回 `(Transport, Behaviour)` 元组，不再需要传入传输层。

### 2. DCUtR 新增

需要在 `AgentoraBehaviour` 中添加 `dcutr::Behaviour`：

```rust
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "AgentoraBehaviourEvent")]
pub struct AgentoraBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,       // 新增
    pub autonat: autonat::Behaviour,   // 新增
    pub ping: libp2p_ping::Behaviour,
    pub identify: libp2p_identify::Behaviour,
}
```

### 3. 新增事件处理

```rust
// DCUtR 事件
AgentoraBehaviourEvent::Dcutr(dcutr_event) => {
    match dcutr_event.result {
        Ok(connection_id) => { /* 打洞成功 */ }
        Err(error) => { /* 打洞失败 */ }
    }
}

// AutoNAT 事件
AgentoraBehaviourEvent::Autonat(autonat_event) => {
    match autonat_event {
        autonat::Event::OutboundProbe(event) => { /* 出站探测 */ }
        autonat::Event::InboundProbe(event) => { /* 入站探测 */ }
        autonat::Event::StatusChanged { old, new } => { /* 状态变更 */ }
    }
}
```

### 4. 新增公共 API

| 方法 | 说明 |
|------|------|
| `get_nat_status()` | 获取当前 NAT 状态 |
| `get_connection_type(peer_id)` | 获取与对等点的连接类型 |
| `connect_to_peer(peer_id)` | 智能连接（自动降级） |
| `add_peer_address(peer_id, addr)` | 注册对等点地址 |
| `request_reservation(relay_peer_id, relay_addr)` | 请求中继 reservation |
| `connect_via_relay(relay_addr, target_peer_id)` | 通过中继连接 |
| `get_relay_reservations()` | 获取当前中继列表 |
| `get_peer_failures(peer_id)` | 获取对等点失败计数 |
| `get_peer_address(peer_id)` | 获取对等点地址缓存 |

## 连接策略变更

### 之前 (0.54)

```rust
transport.connect_to_seed(addr).await?;  // 仅支持直连/中继
```

### 之后 (0.56)

```rust
transport.connect_to_peer(peer_id).await?;  // 自动选择最佳策略
// 直连 → DCUtR → Relay 三级降级
```

## 注意事项

### 向后兼容

- `connect_to_seed` API 保持不变
- GossipSub 订阅/发布 API 不变
- CRDT 消息格式不变
- 密钥文件格式不变

### 破坏性变更

- `Libp2pTransport::new()` 内部启动的 Swarm 现在包含更多后台任务（AutoNAT 探测、DCUtR 监听）
- `get_peer_failures` 和 `record_peer_failure` 从私有方法变为公共方法

## 回滚

如果遇到问题，可以参考 [回滚与降级指南](rollback-guide.md)。
