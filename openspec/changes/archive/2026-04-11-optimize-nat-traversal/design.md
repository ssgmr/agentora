## Context

当前 NAT 穿透实现状态：
- libp2p 0.54 + Circuit Relay v2
- 穿透率 ~100%，但延迟高（150-300ms）
- 中继带宽成本高（100% 流量）
- 无 NAT 类型探测，无法优化路径选择

目标状态：
- libp2p 0.56 + DCUtR + AutoNAT + Relay（混合）
- 穿透率 ~98%，延迟 ~50ms（直连）
- 中继带宽降低至 ~15%（仅协调 + 对称 NAT）
- 自动探测 NAT 类型，智能选择策略

## Goals / Non-Goals

**Goals:**
- 升级 libp2p 到 0.56 最新版本
- 实现 DCUtR Hole Punching 直连
- 实现 AutoNAT NAT 类型探测
- 实现混合穿透策略（DCUtR 优先 → Relay 保底）
- 保持向后兼容，Relay 方案仍可用

**Non-Goals:**
- WebRTC 传输（后续优化）
- 自定义 NAT 穿透协议
- P2P 节点声誉系统

## Decisions

### Decision 1: libp2p 版本升级

**目标版本**: 0.56.0

```toml
# Workspace Cargo.toml
libp2p = { version = "0.56", features = ["tokio"] }
libp2p-gossipsub = "0.49"      # 升级
libp2p-kad = "0.48"            # 升级
libp2p-relay = "0.21"          # 升级
libp2p-dcutr = "0.14"          # 新增
libp2p-autonat = "0.15"        # 新增
libp2p-ping = "0.47"           # 升级
libp2p-identify = "0.47"       # 升级
libp2p-tcp = "0.44"            # 升级
libp2p-noise = "0.47"          # 升级
libp2p-yamux = "0.48"          # 升级
libp2p-dns = "0.44"            # 升级
libp2p-swarm-derive = "0.36"   # 升级
```

**理由**: 0.56 是最新稳定版，API 更统一，性能优化。

### Decision 2: DCUtR 集成

**架构设计**:

```
┌─────────────────────────────────────────────────────────┐
│              AgentoraBehaviour                           │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  #[derive(NetworkBehaviour)]                            │
│  pub struct AgentoraBehaviour {                         │
│      pub gossipsub: gossipsub::Behaviour,               │
│      pub kademlia: kad::Behaviour<...>,                 │
│      pub relay_client: relay::client::Behaviour,        │
│      pub dcutr: dcutr::Behaviour,           // 新增     │
│      pub autonat: autonat::Behaviour,       // 新增     │
│      pub ping: libp2p_ping::Behaviour,                  │
│      pub identify: libp2p_identify::Behaviour,          │
│  }                                                      │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**事件处理**:

```rust
match dcutr_event {
    dcutr::Event::DirectConnectionUpgradeSucceeded { 
        peer_id, connection_id 
    } => {
        tracing::info!("DCUtR 直连成功：{}", peer_id);
        // 记录直连成功，后续优先直连
    }
    dcutr::Event::DirectConnectionUpgradeFailed { 
        peer_id, error 
    } => {
        tracing::warn!("DCUtR 失败，降级 Relay: {} - {:?}", peer_id, error);
        // 自动降级到 Circuit Relay
    }
}
```

**理由**: DCUtR 利用 Relay 协调打洞，成功后直连，延迟降低 67%。

### Decision 3: AutoNAT 集成

**NAT 类型探测**:

```rust
// AutoNAT 配置
let autonat_config = autonat::Config {
    only_global_ips: false,  // 允许探测内网地址
    ..Default::default()
};

// 添加到 Behaviour
pub struct AgentoraBehaviour {
    pub autonat: autonat::Behaviour,
    // ...
}

// 事件处理
match autonat_event {
    autonat::Event::OutboundProbeConfirmed { 
        probe_id, 
        observed_addr,
        ..
    } => {
        // 确认 NAT 类型和公网地址
        tracing::info!("NAT 探测成功：{:?} -> {}", nat_type, observed_addr);
    }
    autonat::Event::OutboundProbeFailed { probe_id, error } => {
        // 探测失败，降级到 Relay
        tracing::warn!("NAT 探测失败：{:?}", error);
    }
}
```

**NAT 类型与策略映射**:

| NAT 类型 | 策略 | 预期成功率 |
|----------|------|------------|
| 全锥型 (Full Cone) | DCUtR | ~100% |
| 受限锥型 (Restricted Cone) | DCUtR | ~95% |
| 端口受限锥型 (Port Restricted) | DCUtR | ~85-90% |
| 对称型 (Symmetric) | Relay | ~100% |

**理由**: 对称 NAT 无法打洞，提前探测可避免无效尝试。

### Decision 4: 混合穿透策略

**连接流程**:

```
┌─────────────────────────────────────────────────────────┐
│          混合 NAT 穿透策略流程                            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  1. 检查 AutoNAT 缓存                                    │
│     └─► 对称 NAT？─是─► 直接使用 Relay                  │
│         └─否                                            │
│                                                         │
│  2. 尝试直连（已知公网地址）                              │
│     └─► 成功？─是─► 完成                               │
│         └─否                                            │
│                                                         │
│  3. 发起 DCUtR 打洞                                       │
│     └─► 成功？─是─► 完成                               │
│         └─否                                            │
│                                                         │
│  4. 降级到 Circuit Relay                                │
│     └─► 完成（保底）                                    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**实现伪代码**:

```rust
pub async fn connect_to_peer(&self, peer_id: &str) -> Result<ConnectionType> {
    // 1. 检查 NAT 类型
    let nat_type = self.autonat.local_status().await;
    if matches!(nat_type, NatType::Symmetric) {
        return self.connect_via_relay(peer_id).await;
    }
    
    // 2. 尝试直连
    if let Ok(addr) = self.get_peer_address(peer_id).await {
        if self.dial_direct(addr).await.is_ok() {
            return Ok(ConnectionType::Direct);
        }
    }
    
    // 3. 发起 DCUtR
    if let Some(relay) = self.find_available_relay().await {
        if self.dcutr_upgrade(peer_id, relay).await.is_ok() {
            return Ok(ConnectionType::Dcutr);
        }
    }
    
    // 4. 降级 Relay
    self.connect_via_relay(peer_id).await
}
```

**理由**: 渐进式降级，确保连接成功率的同时优化延迟。

## Risks / Trade-offs

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| libp2p 0.56 API 变化 | 代码需调整 | 参考官方迁移指南，逐步验证 |
| DCUtR 穿透率低于预期 | 直连成功率低 | Relay 保底，自动降级 |
| AutoNAT 探测耗时 | 启动延迟增加 | 异步探测，结果缓存 |
| 多协议并发复杂度 | bug 风险增加 | 充分的单元测试和集成测试 |
| 版本升级引入回归 | 现有功能受损 | 保留回滚能力，逐步灰度 |

## Migration Plan

### 阶段 1: 依赖升级（1-2 天）
1. 更新 Cargo.toml 依赖版本
2. 修复编译错误
3. 验证基本功能（GossipSub、KAD、Relay）

### 阶段 2: DCUtR 集成（2-3 天）
1. 添加 `dcutr::Behaviour` 到 `AgentoraBehaviour`
2. 实现 DCUtR 事件处理
3. 实现 `dcutr_upgrade` 方法

### 阶段 3: AutoNAT 集成（1-2 天）
1. 添加 `autonat::Behaviour` 到 `AgentoraBehaviour`
2. 实现 NAT 类型探测和缓存
3. 实现 NAT 类型到策略的映射

### 阶段 4: 混合策略实现（2-3 天）
1. 实现连接管理器（优先直连）
2. 实现渐进式降级逻辑
3. 添加连接质量评估

### 阶段 5: 测试与验证（2-3 天）
1. 单节点启动测试
2. 多节点联机测试
3. NAT 穿透成功率验证
4. 延迟和带宽基准测试

### 回滚策略
- Git tag 标记当前稳定状态
- 保留 0.54 依赖分支
- 配置开关可禁用 DCUtR/AutoNAT

## Open Questions

- [ ] DCUtR 重试次数和超时时间设置
- [ ] AutoNAT 探测频率（避免过于频繁）
- [ ] 中继节点选择策略（多中继场景）
- [ ] 连接质量评估指标设计
