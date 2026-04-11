## 1. libp2p 版本升级

- [x] 1.1 更新 Cargo.toml 依赖版本
  - libp2p: 0.54 → 0.56
  - libp2p-gossipsub: 0.47 → 0.49
  - libp2p-kad: 0.46 → 0.48
  - libp2p-relay: 0.18 → 0.21
  - libp2p-ping: 0.45 → 0.47
  - libp2p-identify: 0.45 → 0.47
  - libp2p-tcp: 0.42 → 0.44
  - libp2p-noise: 0.45 → 0.47
  - libp2p-yamux: 0.46 → 0.48
  - libp2p-dns: 0.42 → 0.44
  - libp2p-swarm-derive: 0.35 → 0.36
- [x] 1.2 添加新依赖
  - libp2p-dcutr: ^0.14
  - libp2p-autonat: ^0.15
- [x] 1.3 修复编译错误（API 变化）
- [x] 1.4 验证基础功能正常（GossipSub、KAD、Relay）

## 2. DCUtR 集成

- [x] 2.1 添加 `dcutr::Behaviour` 到 `AgentoraBehaviour`
- [x] 2.2 实现 `AgentoraBehaviourEvent::Dcutr` 变体
- [x] 2.3 实现 `From<dcutr::Event>` trait
- [x] 2.4 实现 DCUtR 事件处理
  - `DirectConnectionUpgradeSucceeded`
  - `DirectConnectionUpgradeFailed`
- [x] 2.5 实现 `dcutr_upgrade` 方法

## 3. AutoNAT 集成

- [x] 3.1 添加 `autonat::Behaviour` 到 `AgentoraBehaviour`
- [x] 3.2 实现 `AgentoraBehaviourEvent::Autonat` 变体
- [x] 3.3 实现 `From<autonat::Event>` trait
- [x] 3.4 实现 AutoNAT 事件处理
  - `OutboundProbeConfirmed`
  - `OutboundProbeFailed`
  - `InboundProbeConfirmed`
  - `InboundProbeFailed`
- [x] 3.5 实现 NAT 类型探测和缓存
- [x] 3.6 实现 NAT 类型到策略的映射

## 4. 混合穿透策略实现

- [x] 4.1 实现连接管理器（ConnectionManager）
- [x] 4.2 实现连接质量评估（直连优先）
- [x] 4.3 实现渐进式降级逻辑
  - 直连 → DCUtR → Relay
- [x] 4.4 实现 `connect_to_peer` 智能选择
- [x] 4.5 实现 `get_peer_address` 地址发现
- [x] 4.6 实现 `find_available_relay` 中继选择

## 5. 配置与 API

- [x] 5.1 添加 DCUtR 配置选项
  - 重试次数
  - 超时时间
  - 并发打洞数量
- [x] 5.2 添加 AutoNAT 配置选项
  - 探测频率
  - 探测超时
  - 是否探测内网地址
- [x] 5.3 添加混合策略配置选项
  - 直连超时
  - DCUtR 超时
  - 降级阈值
- [x] 5.4 更新 `Libp2pTransport` 公共 API
  - `connect_to_peer` 行为变更
  - 新增 `get_nat_type` 方法
  - 新增 `get_connection_type` 方法

## 6. 测试与验证

- [x] 6.1 编写单元测试
  - DCUtR 事件处理
  - AutoNAT 事件处理
  - 混合策略逻辑
- [x] 6.2 编写集成测试
  - 单节点启动
  - 两节点直连
  - 两节点 DCUtR 穿透
  - 两节点 Relay 连接
- [x] 6.3 性能基准测试
  - 连接延迟
  - 穿透成功率
  - 中继带宽使用率
- [x] 6.4 多节点联机测试
  - 3+ 节点同时在线
  - 跨 NAT 连接验证
  - 消息广播正确性

## 7. 文档与迁移

- [x] 7.1 更新网络层文档
- [x] 7.2 编写 NAT 穿透配置指南
- [x] 7.3 编写迁移指南（0.54 → 0.56）
- [x] 7.4 更新多节点测试脚本

## 8. 回滚与降级

- [x] 8.1 实现配置开关（禁用 DCUtR/AutoNAT）
- [x] 8.2 保留 0.54 依赖分支
- [x] 8.3 编写回滚操作手册
