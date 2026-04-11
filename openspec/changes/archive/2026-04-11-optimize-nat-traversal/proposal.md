## Why

当前 P2P 网络层使用 libp2p 0.54 + Circuit Relay v2 作为唯一的 NAT 穿透方案。虽然 Relay 方案穿透率 ~100%，但存在以下问题：

1. **延迟高** - 所有数据经中继转发，决策延迟从 ~50ms 增至 ~150-300ms
2. **带宽成本** - 中继节点承担 100% 流量，长期运行成本高
3. **单点瓶颈** - 中继节点故障影响所有连接
4. **版本落后** - libp2p 0.54 非最新，0.56 已发布且 API 更稳定

MVP 验证需求：多节点联机延迟 < 100ms，直连成功率 > 85%。

## What Changes

- **升级** libp2p 从 0.54 到 0.56
- **新增** DCUtR (Direct Connection Upgrade through Relay) Hole Punching 支持
- **新增** AutoNAT NAT 类型自动探测
- **保留** Circuit Relay v2 作为降级方案
- **实现** 混合穿透策略（DCUtR 优先 → Relay 保底）

## Capabilities

### New Capabilities

- `dcutr-hole-punching`: DCUtR 协议实现，通过中继协调打洞建立直连
- `autonat-detection`: AutoNAT 自动探测 NAT 类型（全锥/受限锥/端口受限/对称）
- `hybrid-nat-strategy`: 混合 NAT 穿透策略，根据 NAT 类型自动选择最优方案
- `libp2p-0.56-upgrade`: 升级到 libp2p 0.56 生态

### Modified Capabilities

- `circuit-relay-v2`: 从主要方案降级为备选方案
- `connection-manager`: 增加连接质量评估，优先直连

### Deprecated Capabilities

- 无（向后兼容，Relay 方案仍可用）

## Impact

- **affected crates**: `network` (libp2p 升级、DCUtR/AutoNAT 集成)
- **dependencies**: 
  - `libp2p`: 0.54 → 0.56
  - `libp2p-relay`: 0.18 → 0.21
  - `libp2p-dcutr`: 新增 ^0.14
  - `libp2p-autonat`: 新增 ^0.15
- **breaking changes**: 
  - libp2p 0.56 API 有小幅变化，需调整 Swarm 构建代码
  - `AgentoraBehaviour` 需添加 `dcutr` 和 `autonat` 字段
- **integration points**: 
  - `Libp2pTransport::connect_to_peer` 优先尝试直连/DCUtR
  - 连接失败自动降级到 Relay
- **performance**: 
  - 直连延迟：~150ms → ~50ms (降低 67%)
  - 中继带宽：100% → ~15% (降低 85%)
  - 穿透成功率：~100% → ~98% (对称 NAT 降级到 Relay)

## Risks

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| libp2p 0.56 API 不兼容 | 需重构代码 | 保留 0.54 代码分支，快速回滚 |
| DCUtR 穿透率低 | 连接质量下降 | Relay 保底，自动降级 |
| AutoNAT 探测延迟 | 启动时间增加 | 异步探测，缓存结果 |
| 新版本引入 bug | 功能回归 | 充分单元测试 + 多节点集成测试 |
