# 需求说明书

## 背景概述

当前 P2P 网络基础设施（libp2p Swarm、GossipSub、Kademlia、DCUtR、AutoNAT）已在 network crate 中完整实现，CRDT 数据结构（LWW、G-Counter、OR-Set）也在 sync crate 中就绪。然而，P2P 功能从未被 Bridge 和 Godot 客户端真正接入——`SimulationBridge` 始终使用 `Simulation::new()`（中心化模式），Delta 广播从未调用 `publish_delta_p2p()`，CRDT 操作从未通过网络传输。两个客户端无法互相发现和同步，"去中心化数文明模拟器"的核心愿景尚未实现。

## 变更目标

- 目标1：打通 Bridge → P2P → Bridge 完整数据链路，两个 Godot 客户端启动后能互相发现并同步 Agent 状态
- 目标2：实现 Delta 按区域 topic 广播，远端 Agent 的动作实时出现在本地客户端
- 目标3：实现 Narrative 按区域/world topic 广播，远端叙事事件在本地叙事流中展示
- 目标4：接通 CRDT 操作传输层，实现 SyncState 的 apply_op 和 merge 通过 GossipSub 同步
- 目标5：Godot 端提供 P2P 连接状态展示（peer_id、连接数、NAT 状态）
- 目标6：编写多节点集成测试，验证两个客户端间的连通性和数据同步正确性

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `bridge-p2p-api` | SimulationBridge 暴露 P2P 方法（connect_to_seed、get_peer_id、get_connected_peers）和信号（peer_connected、peer_delta、p2p_status_changed） |
| `client-p2p-ui` | Godot 客户端 P2P 连接面板：显示 peer_id、NAT 状态、连接种子按钮、已连接 peers 列表 |
| `crdt-p2p-sync` | CRDT 操作通过 GossipSub 发布和消费，SyncState apply_op/merge 在网络层触发 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `p2p-gossip` | 现有 spec 已定义区域 topic 订阅、Delta/叙事按区域广播，但实现缺失——本次补齐实现 |
| `delta-system` | DeltaDispatcher 增加 P2P 广播调用，agent_loop 中 DeltaEmitter::emit_all 后触发 publish_delta_p2p |
| `simulation-orchestrator` | SimulationBridge 根据 sim.toml [p2p] section 选择 Simulation::new() 或 with_p2p() |

## 影响范围

- **代码模块**：
  - `crates/bridge/src/bridge.rs` — 新增 P2P 方法/信号、SimCommand
  - `crates/bridge/src/simulation_runner.rs` — P2P 模式判断和 Simulation 创建
  - `crates/core/src/simulation/agent_loop.rs` — Delta 广播触发
  - `crates/core/src/simulation/delta_emitter.rs` — 可选的 P2P 广播适配
  - `crates/core/src/simulation/simulation.rs` — P2P 网络初始化已有，需验证
  - `crates/core/src/sync/` — CRDT 操作的网络集成
  - `crates/network/src/` — 代码去重（libp2p_transport.rs 与 swarm.rs/behaviour.rs）
  - `client/scripts/` — 新增 P2P UI 面板和相关 GDScript
- **API接口**：
  - GDExtension 新增 `@export` func: `connect_to_seed(addr: String)`, `get_peer_id() -> String`, `get_connected_peers() -> Array`
  - GDExtension 新增 signal: `peer_connected(peer_id: String)`, `p2p_status_changed(status: Dictionary)`
- **依赖组件**：libp2p (已有), serde_json (已有)
- **关联系统**：Godot 4 客户端、config/sim.toml 配置

## 验收标准

- [ ] 两个 Godot 客户端分别加载 `sim_node_a.toml` 和 `sim_node_b.toml` 启动，A 连接 B 的种子地址后建立连接
- [ ] A 客户端的 Agent 执行动作（移动、采集等）后，B 客户端在 2 秒内渲染出对应的远端 Agent 状态变化
- [ ] A 客户端产生的叙事事件在 B 客户端的叙事流面板中展示
- [ ] `get_peer_id()` 返回非空字符串，`get_connected_peers()` 返回已连接 peers 列表
- [ ] 运行 `cargo test --package agentora-network` 通过所有单元测试
- [ ] 新增集成测试：两节点启动 → 连接 → A 发送 delta → B 收到并验证内容
- [ ] 清理 network crate 中的重复代码（libp2p_transport.rs 不再重复定义 behaviour/nat/config）
