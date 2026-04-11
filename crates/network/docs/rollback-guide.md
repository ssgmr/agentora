# 回滚与降级指南

## 概述

本文档描述如何在 libp2p 0.56 + DCUtR/AutoNAT 版本遇到问题时，回滚到之前稳定状态。

## 8.1 配置开关（运行时禁用功能）

### 禁用 DCUtR

在不修改代码的情况下关闭 DCUtR 打洞功能：

```rust
let mut config = HybridStrategyConfig::default();
config.enable_dcutr = false;
```

效果：
- `connect_to_peer()` 将跳过 DCUtR 打洞步骤
- 连接策略变为：**直连 → Relay**（两级降级）
- DCUtR 行为仍然编译在 Swarm 中，但不会被主动调用

### 禁用 AutoNAT

```rust
let mut config = HybridStrategyConfig::default();
config.enable_autonat = false;
```

效果：
- NAT 状态将始终保持在 `Unknown`
- 不会触发 AutoNAT 探测流量
- `get_nat_status()` 返回 `NatStatus::Unknown`

### 同时禁用（极简模式）

```rust
let config = HybridStrategyConfig {
    enable_dcutr: false,
    enable_autonat: false,
    direct_timeout_secs: 3,
    dcutr_timeout_secs: 3,
    degradation_threshold: 1,
    ..HybridStrategyConfig::default()
};
```

这恢复到接近 libp2p 0.54 的行为：直连失败后直接使用中继。

## 8.2 保留 0.54 依赖分支

### Git 回滚步骤

如果需要完全回滚到 libp2p 0.54：

```bash
# 1. 确认当前提交
git log --oneline -5

# 2. 回退到 upgrade 之前的提交
# （假设优化 NAT 穿透的提交是最近的，用 git log 确认）
git log --oneline -- crates/network/Cargo.toml

# 3. 创建回滚分支
git checkout -b rollback/libp2p-054

# 4. 恢复 Cargo.toml 中的 libp2p 版本
# 编辑 crates/network/Cargo.toml 和 Cargo.toml (workspace)
# 将所有 libp2p-* 依赖改回 0.54 系列

# 5. 恢复 libp2p_transport.rs 到 0.54 兼容版本
git checkout HEAD~1 -- crates/network/src/libp2p_transport.rs

# 6. 移除 DCUtR 和 AutoNAT 依赖
# 编辑 Cargo.toml，移除 libp2p-dcutr 和 libp2p-autonat

# 7. 验证编译
cargo build -p agentora-network

# 8. 运行测试
cargo test -p agentora-network
```

### Cargo.toml 版本对照

回滚时需要修改的版本：

```toml
# workspace Cargo.toml
[workspace.dependencies]
libp2p = { version = "0.54", features = ["tokio"] }
libp2p-gossipsub = "0.47"
libp2p-kad = "0.46"
libp2p-relay = "0.18"
libp2p-ping = "0.45"
libp2p-identify = "0.45"
libp2p-tcp = { version = "0.42", features = ["tokio"] }
libp2p-noise = "0.45"
libp2p-yamux = "0.46"
libp2p-dns = { version = "0.42", features = ["tokio"] }
libp2p-swarm-derive = "0.35"
# 移除以下两行：
# libp2p-dcutr = "0.14"
# libp2p-autonat = "0.15"
```

### 代码回滚要点

回滚 `libp2p_transport.rs` 时需要：

1. **移除 `AgentoraBehaviour` 中的 dcutr 和 autonat 字段**
2. **移除对应的 `From` impl**（`From<dcutr::Event>` 和 `From<autonat::Event>`）
3. **移除 `AgentoraBehaviourEvent` 中的 `Dcutr` 和 `Autonat` 变体**
4. **移除 Swarm 事件循环中的 `dcutr` 和 `autonat` 行为创建**
5. **移除 DCUtR/AutoNAT 事件处理代码**
6. **恢复 Relay Client 初始化**（0.54 需要传入 transport）：
   ```rust
   // 0.54:
   let relay_client = relay::client::new(local_key.public().to_peer_id(), transport.clone());
   // 0.56:
   let (_relay_transport, relay_client) = relay::client::new(local_key.public().to_peer_id());
   ```
7. **移除 `enable_dcutr` / `enable_autonat` 配置字段**
8. **移除 `NatStatus` 相关代码**（如果没有 AutoNAT）

## 8.3 回滚操作手册

### 检查点

在决定是否回滚前，检查以下指标：

```bash
# 1. 编译是否通过
cargo build -p agentora-network

# 2. 测试是否通过
cargo test -p agentora-network -- --test-threads=1

# 3. 性能基准
cargo test -p agentora-network --test benchmark_tests -- --nocapture

# 4. 多节点测试
cargo test -p agentora-network --test multi_node_tests -- --nocapture
```

### 决策树

```
编译失败？
  ├─ 是 → 先尝试运行时禁用功能（enable_dcutr = false, enable_autonat = false）
  │       └─ 仍失败？ → 回滚到 0.54
  └─ 否 ↓
测试失败？
  ├─ 是 → 查看失败原因
  │       ├─ DCUtR 相关 → 禁用 DCUtR
  │       ├─ AutoNAT 相关 → 禁用 AutoNAT
  │       └─ 其他 → 回滚到 0.54
  └─ 否 ↓
性能退化？
  ├─ 是 → 检查 benchmark_tests，确认退化来源
  └─ 否 → 升级成功
```

### 回滚执行步骤

```bash
# 步骤 1: 创建回滚分支
git checkout -b rollback/nat-traversal

# 步骤 2: 恢复版本依赖
# 手动编辑 Cargo.toml 和 crates/network/Cargo.toml

# 步骤 3: 恢复核心代码
git checkout <before-upgrade-commit> -- crates/network/src/libp2p_transport.rs

# 步骤 4: 移除新增的测试文件
rm crates/network/tests/benchmark_tests.rs
rm crates/network/tests/multi_node_tests.rs

# 步骤 5: 移除新增的文档
rm -rf crates/network/docs/
rm crates/network/README.md

# 步骤 6: 验证
cargo build
cargo test -p agentora-network

# 步骤 7: 提交回滚
git commit -m "revert: rollback libp2p 0.54 → 0.55 (NAT traversal issues)"
```

### 常见问题排查

| 问题 | 原因 | 解决方案 |
|------|------|---------|
| `relay::client::new` 参数不匹配 | 版本 API 变更 | 检查 Cargo.toml 版本是否一致 |
| DCUtR 事件处理编译错误 | 缺少依赖 | 添加 `libp2p-dcutr` 或禁用功能 |
| 节点无法连接 | 中继节点未部署 | 部署中继节点或禁用 DCUtR |
| 内存占用增加 | AutoNAT 频繁探测 | 增加 `probe_interval_secs` 或禁用 |
| Swarm 启动缓慢 | 额外行为初始化 | 属正常现象，首次启动会稍慢 |

### 升级后监控

升级后建议监控以下指标：

1. **连接成功率**: `connect_to_peer` 的 `Ok` / `Err` 比例
2. **NAT 状态分布**: `Public` / `Private` / `Unknown` 的节点占比
3. **连接类型分布**: `Direct` / `Dcutr` / `Relay` 的比例
4. **失败计数**: `get_peer_failures` 的分布
