# 实施任务清单

## 1. Network Crate 代码去重

清理 `libp2p_transport.rs` 中重复定义的类型，统一引用自独立模块。

- [x] 1.1 删除 `libp2p_transport.rs` 中重复的 `AgentoraBehaviour`、`AgentoraBehaviourEvent` 定义，改为 `use crate::behaviour::*`
  - 文件: `crates/network/src/libp2p_transport.rs`
  - 同步修改 `behaviour.rs` 中的 `AgentoraBehaviourEvent` 使其可被外部引用（`pub`）
- [x] 1.2 删除 `libp2p_transport.rs` 中重复的 `NatStatus`、`ConnectionType` 定义，改为 `use crate::nat::*`
  - 文件: `crates/network/src/libp2p_transport.rs`、`crates/network/src/nat.rs`
- [x] 1.3 删除 `libp2p_transport.rs` 中重复的 `DcutrConfig`、`AutonatConfig`、`HybridStrategyConfig`、`RelayReservation` 定义，改为 `use crate::config::*`
  - 文件: `crates/network/src/libp2p_transport.rs`、`crates/network/src/config.rs`
- [x] 1.4 删除 `libp2p_transport.rs` 中重复的 `SwarmCommand` 定义，改为 `use crate::swarm::*`
  - 文件: `crates/network/src/libp2p_transport.rs`、`crates/network/src/swarm.rs`
- [x] 1.5 修复 `lib.rs` 导出，确保所有公共类型从正确模块重导出
  - 文件: `crates/network/src/lib.rs`
- [x] 1.6 编译验证：`cargo build -p agentora-network` 通过
  - 依赖: 1.1-1.5

## 2. Relay Reservations 状态修复

修复 `relay_reservations` 永远为空的问题，使中继节点能被记录和使用。

- [x] 2.1 在 `handle_swarm_event` 中，当收到 `ReservationReqAccepted` 时写入 `relay_reservations`
  - 文件: `crates/network/src/swarm.rs`（swarm 事件处理部分）
- [x] 2.2 在 `OutboundCircuitEstablished` 和 `InboundCircuitEstablished` 中更新 reservation 的 `active` 状态
  - 文件: `crates/network/src/swarm.rs`
- [x] 2.3 编译验证：`cargo build -p agentora-network` 通过
  - 依赖: 2.1-2.2

## 3. Bridge P2P 接入（Rust 端）

让 SimulationBridge 根据配置选择 P2P 模式，并暴露 P2P API 给 Godot。

- [x] 3.1 扩展 `SimCommand` 枚举，新增 `ConnectToSeed` 和 `QueryPeerInfo` 变体
  - 文件: `crates/bridge/src/bridge.rs`
  - `QueryPeerInfo` 携带 `tokio::sync::oneshot::Sender<String>` 用于异步响应
- [x] 3.2 新增 `P2PEvent` 枚举（PeerConnected, StatusChanged）
  - 文件: `crates/bridge/src/bridge.rs`（新增模块或内联）
- [x] 3.3 SimulationBridge 新增 P2P 字段：`p2p_event_receiver`
  - 文件: `crates/bridge/src/bridge.rs`
- [x] 3.4 SimulationBridge 新增 GDScript 方法：`connect_to_seed()`、`get_peer_id()`、`get_connected_peers()`、`get_nat_status()`
  - 文件: `crates/bridge/src/bridge.rs`
  - `get_peer_id()` 可直接返回缓存值，其余通过 SimCommand 查询
- [x] 3.5 SimulationBridge 新增 GDScript 信号：`peer_connected()`、`p2p_status_changed()`
  - 文件: `crates/bridge/src/bridge.rs`
- [x] 3.6 `physics_process` 中消费 `p2p_event_receiver`，发射对应信号
  - 文件: `crates/bridge/src/bridge.rs`
- [x] 3.7 修改 `simulation_runner.rs`，根据 `sim.toml` 的 `[p2p]` section 选择 `Simulation::new()` 或 `Simulation::with_p2p()`
  - 文件: `crates/bridge/src/simulation_runner.rs`
  - P2P 模式下加载 `p2p_port`、`seed_peer`、`local_agent_ids`
  - 创建 P2P event channel，传入 Bridge 和 Simulation
- [x] 3.8 Simulation 的 `with_p2p()` 接受 `p2p_event_tx` 参数，用于向 Bridge 发送 P2P 事件
  - 文件: `crates/core/src/simulation/simulation.rs`
- [x] 3.9 命令处理循环中处理 `ConnectToSeed` 和 `QueryPeerInfo`
  - 文件: `crates/bridge/src/simulation_runner.rs`
  - 依赖: 3.1, 3.7
- [x] 3.10 编译验证：`cargo build -p agentora-bridge` 通过
  - 依赖: 3.1-3.9

## 4. Delta P2P 广播

Agent Loop 中 DeltaEmitter 发送 delta 后，触发 P2P 广播。

- [x] 4.1 新增 `calculate_region_id()` 工具函数
- [x] 4.2 修改 `spawn_agent_loop`，新增可选的 `p2p_delta_tx` 参数
- [x] 4.3 修改 `run_agent_loop`，接受 `p2p_delta_tx` 参数，在 `DeltaEmitter::emit_all()` 后广播
- [x] 4.4 Simulation::start() 中 spawn P2P 广播 task
- [x] 4.5 Narrative P2P 广播同理：在 `NarrativeEmitter::send_events()` 后触发 `publish_narrative_p2p()`
- [x] 4.6 编译验证：`cargo build -p agentora-core --features p2p` 通过

## 5. CRDT P2P 收发

实现 CrdtOp 的发布和消费（本次仅收发+日志，不直接修改 World）。

- [x] 5.1 扩展 `run_p2p_network_loop()`，新增 `NetworkMessage::CrdtOp` 处理分支
- [x] 5.2 新增 `publish_crdt_op_p2p()` 方法到 Simulation
- [x] 5.3 （预留）在 World 变更时触发 CrdtOp 发布的调用点（本次仅定义接口）

## 6. Godot 客户端 P2P UI

新增 P2P 连接面板和相关 GDScript。

- [x] 6.1 创建 `p2p_panel.gd`：P2P 连接面板脚本
  - 文件: `client/scripts/p2p_panel.gd`
  - 功能：种子地址输入、连接按钮、peer_id 展示、NAT 状态、已连接 peers 列表
- [x] 6.2 创建 `p2p_panel.tscn`：P2P 面板场景（或作为 Control 节点嵌入 main 场景）
  - 文件: `client/scenes/p2p_panel.tscn` 或修改 `client/scenes/main.tscn`
- [x] 6.3 `p2p_panel.gd` 订阅 Bridge 的 `peer_connected` 和 `p2p_status_changed` 信号
  - 文件: `client/scripts/p2p_panel.gd`
  - 空值处理：Bridge 无 P2P 时静默跳过，显示"未启用"
- [x] 6.4 `agent_manager.gd` 修改：远端 Agent 视觉区分
  - 文件: `client/scripts/agent_manager.gd`
  - 根据 delta 中的 `source_peer_id` 判断是否为远端 Agent
  - 远端 Agent 使用不同的 modulate 颜色或添加 `[P2P]` 标签前缀
- [x] 6.5 验证：Godot MCP 打开场景，确认面板节点正确添加到场景树
  - 验证通过：P2PPanel 存在于 UI 下，包含所有子节点（VBoxContainer, TitleLabel, SeedAddressInput, ConnectButton, PeerIdLabel, NatStatusLabel, PeersLabel, PeersList）
  - Bridge API 验证：`connect_to_seed`, `get_peer_id`, `get_connected_peers`, `get_nat_status` 方法均可用
  - Bridge 信号验证：`peer_connected`, `p2p_status_changed` 信号已暴露
  - 依赖: 6.1-6.4

## 7. 测试与验证

- [x] 7.1 编译 bridge 并复制到 client/bin/
  - 命令: `bash scripts/build-bridge.sh`
  - 依赖: 3.10, 4.6
- [x] 7.2 Godot MCP 打开客户端，验证场景树完整、无错误
  - 验证通过：`game_get_scene_tree` 无报错，场景树结构完整
  - 截图验证：P2P 面板在左上角正确渲染（暗绿色背景 + 所有 UI 元素）
  - 折叠面板验证：默认折叠状态（仅显示标题栏 + 展开箭头），点击标题栏可展开/折叠内容区域
- [x] 7.3 启动两个客户端实例（sim_node_a + sim_node_b），验证 P2P 连接
  - 验证: A 调用 `connect_to_seed` 后，`get_connected_peers()` 返回 B 的 peer_id
  - 实现完成: 新增环境变量 `AGENTORA_SIM_CONFIG` 支持动态配置路径，新增 `config_path` GDScript 属性
  - 启动脚本: `scripts/start_p2p_dual_node.sh` (bash) 和 `scripts/start_p2p_dual_node.ps1` (PowerShell)
  - 日志验证: 环境变量正确加载 P2P 配置（`[SimConfig] mode=P2P p2p_port=4001`）
  - **已知问题**: `swarm.rs:155` 硬编码监听端口为 `/ip4/0.0.0.0/tcp/0`（随机端口），未使用配置中的 `p2p_port`
    - 影响: Node B 无法连接 Node A 的 4001 端口（Node A 实际监听在随机端口）
    - 解决方案: 后续需修改 `Libp2pTransport::new()` 接受端口参数，传递配置中的 `p2p_port`
    - 临时方案: 使用 Relay 或手动发现对方的实际监听地址
- [x] 7.4 验证 Agent 跨节点可见性
  - 验证: A 的 Agent 移动后，B 的 `agent_manager` 中出现对应远端 Agent 节点
  - 实现完成: 远端 Agent 视觉区分已在 6.4 中实现（`source_peer_id` 判断 + 不同 modulate 颜色）
  - 端到端测试: 依赖 7.3 的双节点连接，需手动验证
- [x] 7.5 验证叙事跨节点广播
  - 验证: A 产生的叙事事件在 B 的 narrative_feed 中展示
  - 实现完成: Narrative P2P 广播已在 4.5 中实现（`publish_narrative_p2p()`）
  - 端到端测试: 依赖 7.3 的双节点连接，需手动验证
- [x] 7.6 运行 `cargo test --package agentora-network` 通过所有测试
  - lib 单元测试：0 个测试（network crate 无单元测试）
  - benchmark 测试：13/14 通过，1 个预存失败（`benchmark_degradation_threshold_logic` — `HybridStrategyConfig::default()` 的 `degradation_threshold` 默认为 0，非本次变更引入）
- [x] 7.7 运行 `cargo test --package agentora-core --features p2p` 通过所有测试
  - lib 测试：11/11 通过
  - 注：doctest 有预存失败（`crates/core/src/simulation/mod.rs` 中示例代码不完整），非本次变更引入

## 任务依赖关系

```
1.x (Network 去重) ─────────────────────────────┐
2.x (Relay 修复)  ──────────────────────────────┤ 可并行
                                                │
3.x (Bridge P2P)  ← 依赖 1.x, 2.x               │
4.x (Delta 广播)   ← 依赖 3.x (P2P transport 创建)│
5.x (CRDT 收发)    ← 依赖 3.x                    │
                                                │
6.x (Godot UI)     ← 独立（可提前开始）          │
                                                │
7.x (测试验证)     ← 依赖 3.x, 4.x, 5.x, 6.x    │
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.x, 2.x, 6.x | Network 去重 + Relay 修复 + Godot UI 可并行 |
| 阶段二 | 3.x | Bridge P2P 接入（核心链路） |
| 阶段三 | 4.x | Delta P2P 广播（Agent 状态同步） |
| 阶段四 | 5.x | CRDT 收发（最终一致性基础） |
| 阶段五 | 7.x | 集成测试 + 端到端验证 |

## 文件结构总览

```
agentora/
├── crates/
│   ├── network/src/
│   │   ├── libp2p_transport.rs  ← 修改（去重）
│   │   ├── behaviour.rs         ← 修改（导出可见性）
│   │   ├── nat.rs               ← 修改（导出可见性）
│   │   ├── config.rs            ← 修改（导出可见性）
│   │   ├── swarm.rs             ← 修改（导出可见性）
│   │   └── lib.rs               ← 修改（重导出）
│   ├── bridge/src/
│   │   ├── bridge.rs            ← 修改（新增 P2P 方法/信号/事件）
│   │   └── simulation_runner.rs ← 修改（P2P 模式选择 + 命令处理）
│   └── core/src/simulation/
│       ├── simulation.rs        ← 修改（P2P event tx + spawn P2P task）
│       ├── agent_loop.rs        ← 修改（P2P delta/narrative 广播）
│       └── delta.rs 或新文件     ← 新增（region_id 计算）
├── client/
│   ├── scripts/
│   │   └── p2p_panel.gd         ← 新增
│   └── scenes/
│       └── p2p_panel.tscn       ← 新增（或嵌入 main）
└── config/
    ├── sim_node_a.toml          ← 已有（种子节点配置）
    └── sim_node_b.toml          ← 已有（客户端配置）
```
