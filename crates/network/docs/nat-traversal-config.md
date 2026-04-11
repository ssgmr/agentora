# NAT 穿透配置指南

## 概述

Agentora 使用 libp2p 的三层 NAT 穿透策略，确保在不同网络环境下节点间能够建立连接。

## 连接优先级

```
直连 (Direct) → DCUtR 打洞 → Relay 中继
```

1. **直连**: 如果节点有公网 IP 或处于同一局域网，直接建立 TCP 连接
2. **DCUtR**: 通过中继节点建立电路，然后尝试升级为直接连接（UDP 打洞）
3. **Relay**: 纯中继电路，所有流量通过中继节点转发

## 配置结构

### HybridStrategyConfig

混合穿透策略主配置：

```rust
pub struct HybridStrategyConfig {
    /// 直连超时（秒），默认 5
    pub direct_timeout_secs: u64,
    /// DCUtR 超时（秒），默认 15
    pub dcutr_timeout_secs: u64,
    /// 降级阈值：直连失败多少次后降级到 DCUtR，默认 2
    pub degradation_threshold: u32,
    /// DCUtR 子配置
    pub dcutr: DcutrConfig,
    /// AutoNAT 子配置
    pub autonat: AutonatConfig,
}
```

### DcutrConfig

DCUtR 打洞详细配置：

```rust
pub struct DcutrConfig {
    /// 最大重试次数，默认 3
    pub max_retries: u32,
    /// 单次尝试超时（秒），默认 10
    pub timeout_secs: u64,
    /// 并发打洞数量，默认 2
    pub concurrent_attempts: u32,
}
```

### AutonatConfig

AutoNAT NAT 探测配置：

```rust
pub struct AutonatConfig {
    /// 是否仅探测全球 IP，默认 false（允许探测内网地址）
    pub only_global_ips: bool,
    /// 探测间隔（秒），默认 30
    pub probe_interval_secs: u64,
    /// 探测超时（秒），默认 15
    pub probe_timeout_secs: u64,
}
```

## 场景配置

### 场景 1: 公网服务器

所有节点有公网 IP，直连即可：

```rust
let config = HybridStrategyConfig {
    direct_timeout_secs: 3,       // 快速直连
    dcutr_timeout_secs: 5,        // DCUtR 作为备选
    degradation_threshold: 1,     // 一次失败就降级
    dcutr: DcutrConfig {
        max_retries: 1,           // 公网环境重试少
        timeout_secs: 3,
        concurrent_attempts: 1,
    },
    autonat: AutonatConfig {
        only_global_ips: true,    // 仅探测公网 IP
        probe_interval_secs: 60,  // 降低探测频率
        probe_timeout_secs: 10,
    },
};
```

### 场景 2: 家庭网络（常见）

大多数节点在 NAT 后面：

```rust
let config = HybridStrategyConfig::default(); // 使用默认值即可
```

默认配置已经针对家庭 NAT 环境优化。

### 场景 3: 对称 NAT / 企业网络

严格的企业防火墙可能需要纯中继：

```rust
let config = HybridStrategyConfig {
    direct_timeout_secs: 2,       // 直连快速失败
    dcutr_timeout_secs: 3,        // 打洞也快速失败
    degradation_threshold: 1,     // 立即降级到中继
    dcutr: DcutrConfig {
        max_retries: 1,
        timeout_secs: 2,
        concurrent_attempts: 1,
    },
    autonat: AutonatConfig {
        only_global_ips: false,
        probe_interval_secs: 15,  // 频繁探测
        probe_timeout_secs: 10,
    },
};
```

## 监控 NAT 状态

```rust
let transport = Libp2pTransport::new()?;

// 查询当前 NAT 状态
let status = transport.get_nat_status().await;
match status {
    NatStatus::Public(addr) => println!("公网可达: {}", addr),
    NatStatus::Private => println!("私有网络，需要中继"),
    NatStatus::Unknown => println!("正在探测"),
}

// 查询与特定对等点的连接类型
let conn_type = transport.get_connection_type("peer_id").await;
match conn_type {
    Some(ConnectionType::Direct) => println!("直连"),
    Some(ConnectionType::Dcutr) => println!("DCUtR 打洞"),
    Some(ConnectionType::Relay) => println!("中继"),
    None => println!("未连接"),
}
```

## 中继节点配置

中继节点需要运行独立的 libp2p-relay 服务。Agentora 客户端通过以下 API 请求中继 reservation：

```rust
let transport = Libp2pTransport::new()?;

// 请求中继 reservation
transport.request_reservation(
    "12D3KooWRelayPeerId...",    // 中继节点 PeerId
    "/ip4/1.2.3.4/tcp/4001",     // 中继节点地址
)?;

// 通过中继连接目标
transport.connect_via_relay(
    "/ip4/1.2.3.4/tcp/4001/p2p/RELAY_PEER",  // 中继地址
    "12D3KooWTargetPeerId...",                // 目标 PeerId
)?;
```

## 性能指标

详见 `benchmark_tests`。关键指标：

| 指标 | 目标 | 实际 |
|------|------|------|
| Transport 创建 | < 500ms | ~278μs |
| 消息发布 | < 1ms | ~145μs |
| NAT 状态查询 | < 50ms | < 1μs |
| 连接类型查询 | < 50ms | < 1μs |
| 消息序列化 | < 100μs | ~6μs |
