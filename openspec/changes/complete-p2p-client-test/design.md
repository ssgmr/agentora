# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

Agentora 的去中心化架构依赖 P2P 网络实现 Agent 间的自主同步。当前技术栈：
- **Network Crate**：基于 rust-libp2p 0.56，完整实现了 GossipSub（区域 topic 订阅）、Kademlia DHT（节点发现）、DCUtR（NAT 穿透打洞）、Circuit Relay v2（中继保底）、AutoNAT（NAT 探测）
- **Sync Crate**：实现了 LWW-Register、G-Counter、OR-Set 三种 CRDT 类型，支持签名验证和 Merkle 校验
- **Core Crate**：`Simulation::with_p2p()` 已实现 P2P 模式构造函数、网络消息循环、Delta/Narrative 发布
- **Delta System**：已简化为 `AgentStateChanged + WorldEvent` 两类，`Delta.for_broadcast()` 已实现精简 JSON 序列化
- **Bridge Crate**：SimulationBridge GDExtension 通过 mpsc 通道与 simulation 线程通信，支持 SimCommand 命令

### 1.2 现状分析

当前存在以下关键阻断：
1. **SimulationBridge 从不创建 P2P 传输层** — 始终使用 `Simulation::new()`（中心化模式），从不检查 sim.toml 的 `[p2p]` section
2. **Delta 不广播到 P2P** — `DeltaEmitter::emit_all()` 只通过 mpsc 发到本地 delta channel，不调用 `Simulation::publish_delta_p2p()`
3. **CRDT 操作不传输** — `NetworkMessage::CrdtOp` 变体存在但从未创建或消费
4. **Godot 无 P2P 控制能力** — 无 connect、peer query、status 等 GDScript API
5. **Network Crate 代码重复** — `libp2p_transport.rs` 重复定义了 `AgentoraBehaviour`、`NatStatus`、`ConnectionType`、config 结构体，与 `behaviour.rs`、`nat.rs`、`config.rs` 重复

### 1.3 关键干系人

- **Rust Bridge 线程**：GDExtension 主线程，通过 physics_process 消费 snapshot/delta/narrative
- **Rust Simulation 线程**：独立 OS 线程中的 tokio runtime，运行 World + Agent loops + P2P network loop
- **Godot 客户端**：两个独立 Godot 进程，各加载不同的 sim 配置

## 2. 设计目标

### 目标

- 打通 Bridge → Simulation(P2P) → Network → Network → Simulation(P2P) → Bridge 完整数据链路
- 两个 Godot 客户端能互相发现、同步 Agent 状态、共享叙事事件
- CRDT 操作通过 GossipSub 传输，最终一致性同步
- 清理 network crate 重复代码

### 非目标

- WebSocket 降级传输（spec 已预留，本次不实现）
- 多区域/多玩家复杂拓扑（仅验证两个点对点客户端）
- CRDT 冲突解决的复杂策略（CRDT 本身已处理合并语义）
- 大规模性能优化（目标是功能可用，非性能最优）

## 3. 整体架构

### 3.1 架构概览

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Godot Client (2 instances)                    │
│                                                                      │
│  ┌──────────┐  ┌─────────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │state_mgr │  │agent_manager│  │p2p_panel │  │narrative_feed    │  │
│  └────┬─────┘  └──────┬──────┘  └────┬─────┘  └────────┬─────────┘  │
│       │               │              │                 │             │
│       └───────────────┴──────────────┴─────────────────┘             │
│                          │ Bridge API                                │
│  ┌───────────────────────▼───────────────────────────────────────┐  │
│  │                    SimulationBridge (GDExtension)              │  │
│  │  + connect_to_seed(addr)  + get_peer_id()                     │  │
│  │  + get_connected_peers()  + get_nat_status()                  │  │
│  │  * SimCommand: ConnectToSeed, QueryPeerInfo                   │  │
│  │  * Signals: peer_connected, p2p_status_changed                │  │
│  └───────────────────────┬───────────────────────────────────────┘  │
│                          │ mpsc channels                             │
│  ┌───────────────────────▼───────────────────────────────────────┐  │
│  │                  Simulation Thread (tokio runtime)             │  │
│  │                                                                │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │  │
│  │  │Agent Loop(s) │  │  Tick Loop   │  │  Network Message     │ │  │
│  │  │              │  │              │  │  Loop                │ │  │
│  │  │ emit_all()──┼──►apply_action  │  │  recv()              │ │  │
│  │  │ └─publish_  │  │              │  │  ├─AgentDelta        │ │  │
│  │  │   delta_p2p()│  │              │  │  ├─Narrative         │ │  │
│  │  │              │  │              │  │  └─CrdtOp            │ │  │
│  │  └──────────────┘  └──────────────┘  └──────────┬───────────┘ │  │
│  │                                                 │              │  │
│  │  ┌──────────────────────────────────────────────▼───────────┐ │  │
│  │  │              Libp2pTransport (Swarm)                      │ │  │
│  │  │  GossipSub pub/sub  Kademlia  DCUtR  AutoNAT  Relay      │ │  │
│  │  └──────────────────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件名 | 所在 Crate | 职责说明 |
| --- | --- | --- |
| `simulation_runner.rs` | bridge | 加载 sim.toml，根据 `[p2p]` section 选择 `Simulation::new()` 或 `with_p2p()` |
| `bridge.rs` | bridge | 新增 P2P GDScript 方法、信号、SimCommand 变体 |
| `Simulation` | core | 已有 `with_p2p()` 和 `publish_delta_p2p()`，需在 agent_loop 中触发 |
| `agent_loop.rs` | core | DeltaEmitter::emit_all() 后增加 P2P 广播调用 |
| `run_p2p_network_loop()` | core | 已有：消费 NetworkMessage，当前处理 AgentDelta + Narrative，需增加 CrdtOp |
| `SyncState` + `CrdtOp` | sync + network | CRDT 操作的序列化、发布、消费、合并 |
| `libp2p_transport.rs` | network | 清理重复代码，保留引用自 behaviour.rs / nat.rs / config.rs |
| `p2p_panel.gd` | client | 新增 P2P 连接面板 GDScript |

### 3.3 数据流设计

**Agent 动作 → P2P 广播 → 远端渲染**

```
Godot Client A                              Godot Client B
     │                                            │
  [点击/引导 Agent 行动]                          │
     │                                            │
     ▼                                            │
agent_loop (tokio task)                          │
     │                                            │
     ├── apply_action() → World 更新              │
     │                                            │
     ├── DeltaEmitter::emit_all()                 │
     │    └── delta_tx.send(AgentStateChanged) ──┼─────► bridge physics_process
     │                                            │       └── emit_signal("agent_delta")
     │                                            │            └── agent_manager 更新节点
     └── publish_delta_p2p()                      │
          └── transport.publish(topic, bytes) ───┼──GossipSub──►
                                                 │
                                                 ▼
                                          run_p2p_network_loop()
                                               │
                                               ├── recv AgentDelta
                                               │    └── delta_tx.send(delta) ──► bridge
                                               │
                                               └── recv CrdtOp
                                                    └── SyncState::apply_op()
```

## 4. 详细设计

### 4.1 接口设计

#### GDExtension 方法新增

**`connect_to_seed(addr: String) -> bool`**
- 调用方式：`Bridge.connect_to_seed("/ip4/127.0.0.1/tcp/4001")`
- 返回值：`true` = 命令已发送（非连接成功），`false` = P2P 未启用
- 内部：发送 `SimCommand::ConnectToSeed { addr }` 到 simulation 线程

**`get_peer_id() -> String`**
- 返回值：本地 peer_id 字符串，中心化模式返回空串
- 内部：从 `Simulation.local_peer_id` 字段直接读取（已存在于 Simulation 结构体）

**`get_connected_peers() -> Array[Dictionary]`**
- 返回值：`[{peer_id: String, connection_type: String}]`
- 内部：发送 `SimCommand::QueryPeerInfo { query_type: "peers", response_tx }`，等待 simulation 线程通过 `crossbeam_channel` 返回结果
- 注意：需使用 `crossbeam_channel` 替代 `std::sync::mpsc` 的 oneshot，或新增一个专用的 oneshot channel 字段

**`get_nat_status() -> Dictionary`**
- 返回值：`{status: String, address: String}`
- 内部：类似 get_connected_peers，通过 QueryPeerInfo 命令查询

#### GDExtension 信号新增

**`peer_connected(peer_id: String)`**
- 触发时机：Swarm 事件 `ConnectionEstablished` 且 peer 为新连接
- 内部：Simulation 线程通过专用 `mpsc::Sender<P2PEvent>` 发送事件，Bridge 的 physics_process 中消费并发射信号

**`p2p_status_changed(status: Dictionary)`**
- 触发时机：NAT 状态变更、peer 断开、连接错误
- status 字段：`{nat_status: String, peer_count: int, error: String}`

#### SimCommand 新增

```rust
pub enum SimCommand {
    Start,
    Pause,
    SetTickInterval { seconds: f32 },
    InjectPreference { agent_id, key, boost, duration_ticks },
    // 新增:
    ConnectToSeed { addr: String },
    QueryPeerInfo { query_type: String, response_tx: oneshot::Sender<String> },
}
```

#### P2PEvent 新增

```rust
#[derive(Debug, Clone)]
pub enum P2PEvent {
    PeerConnected { peer_id: String },
    StatusChanged { nat_status: String, peer_count: usize, error: String },
}
```

### 4.2 数据模型

#### SimCommand 扩展（已有结构修改）

```rust
// crates/bridge/src/bridge.rs
pub enum SimCommand {
    // ... 现有变体 ...
    ConnectToSeed { addr: String },
    QueryPeerInfo {
        query_type: String,  // "peers" | "nat_status" | "peer_id"
        response_tx: tokio::sync::oneshot::Sender<String>,
    },
}
```

#### SimulationBridge 新增字段

```rust
pub struct SimulationBridge {
    // ... 现有字段 ...
    p2p_event_receiver: Option<Receiver<P2PEvent>>,  // 新增
}
```

#### simulation_runner 中新增 channel

```rust
let (p2p_event_tx, p2p_event_rx) = mpsc::channel::<P2PEvent>();
// p2p_event_tx 传入 Simulation::with_p2p() 或 run_simulation_async
```

### 4.3 核心算法

#### Agent Loop 中触发 P2P 广播

当前 `agent_loop.rs` 中已有代码：
```rust
// 阶段 6b: 发送 Delta
DeltaEmitter::emit_all(&delta_tx, &w, &agent_id, &action, &events);
```

修改方案：在持有 world lock 的地方，增加 P2P 广播调用。但 `publish_delta_p2p()` 是 async 方法且需要 `&self`（Simulation 的引用），而 agent_loop 不持有 Simulation。

**方案选择：通过 channel 通知**

```rust
// 新增 P2P 广播通道
let (p2p_delta_tx, p2p_delta_rx) = mpsc::channel::<(Delta, u64, u32)>();
// agent_loop 参数新增: p2p_delta_tx: &Sender<(Delta, u64, u32)>

// 在 agent_loop.rs emit_all 之后:
let tick = w.tick;
let region_id = calculate_region_id(&w, &agent_id, region_size);
let delta = build_delta_from_action(&w, &agent_id, &action);
let _ = p2p_delta_tx.send((delta, tick, region_id));

// 在 Simulation 的 start() 中，spawn 一个 P2P 广播 task:
tokio::spawn(async move {
    while let Ok((delta, tick, region_id)) = p2p_delta_rx.recv().await {
        sim.publish_delta_p2p(&delta, tick, region_id).await;
    }
});
```

**关键问题：需要将 `p2p_delta_tx` 传入 `run_agent_loop`**。当前签名：
```rust
fn spawn_agent_loop(&self, agent_id: AgentId, is_npc: bool) -> JoinHandle<()>
```

修改为额外传入 `Option<Sender<(Delta, u64, u32)>>`，在 agent_loop 中如果该 sender 存在则广播。

#### 区域 ID 计算

```rust
fn calculate_region_id(world: &World, agent_id: &AgentId, region_size: u32) -> u32 {
    if let Some(agent) = world.agents.get(agent_id) {
        let map_width = world.map.width;
        let (x, y) = (agent.position.x, agent.position.y);
        let cols = map_width / region_size;
        (y / region_size) * cols + (x / region_size)
    } else {
        0
    }
}
```

#### CrdtOp 消费（run_p2p_network_loop 扩展）

```rust
// 在 network_loop 的 match msg 分支中新增:
NetworkMessage::CrdtOp(crdt_msg) => {
    if crdt_msg.source_peer_id == local_peer_id {
        continue;  // 回环过滤
    }
    if let Ok(op) = CrdtOp::from_json(&crdt_msg.op_json) {
        // 验证签名
        if !verify_signature(&op, &crdt_msg.source_peer_id) {
            tracing::warn!("[P2P] CrdtOp 签名验证失败");
            continue;
        }
        // 应用操作
        // 注意: 需要通过 channel 通知主 Simulation task 来修改 World
        // 因为 SyncState 目前独立于 World，这里先仅记录日志
        tracing::debug!("[P2P] 收到远程 CrdtOp: {:?}", op);
    }
}
```

**注意**：CRDT 的完整集成需要将 `SyncState` 与 `World` 关联，这涉及较大的数据模型变更。本次先实现 **CrdtOp 的收发和日志**，完整的 apply_op 到 World 的映射留为后续迭代。

#### 网络 crate 代码去重

当前 `libp2p_transport.rs` 重复定义了以下类型，应改为从独立模块引用：

| 重复定义 | 应引用自 |
| --- | --- |
| `AgentoraBehaviour` | `behaviour.rs` |
| `AgentoraBehaviourEvent` | `behaviour.rs` |
| `NatStatus` | `nat.rs` |
| `ConnectionType` | `nat.rs` |
| `DcutrConfig` | `config.rs` |
| `AutonatConfig` | `config.rs` |
| `HybridStrategyConfig` | `config.rs` |
| `RelayReservation` | `config.rs` |
| `SwarmCommand` | `swarm.rs` |

操作：删除 `libp2p_transport.rs` 中的重复定义，添加 `use crate::behaviour::...` / `use crate::nat::...` / `use crate::config::...` / `use crate::swarm::...` 导入。

#### Relay Reservations 修复

当前 `SwarmEvent::RelayClient(ReservationReqAccepted { ... })` 事件处理中仅打印日志，未更新 `relay_reservations` 共享状态。

修改：在事件处理中，当收到 `ReservationReqAccepted` 时：
```rust
let mut reservations = relay_reservations.write().await;
reservations.push(RelayReservation {
    relay_peer_id: relay_peer_id.to_string(),
    relay_addr: endpoint.get_remote_address().to_string(),
    listen_addr: circuit_addr.to_string(),
    active: true,
});
```

同样，在 `OutboundCircuitEstablished` 和 `InboundCircuitEstablished` 中也要更新状态。

### 4.6 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| 种子节点不可达 | 记录警告日志，发射 `p2p_status_changed` 带 error，不阻塞模拟启动 |
| P2P 模式下 transport 创建失败 | 降级到中心化模式，打印错误日志，模拟继续运行 |
| 消息通道关闭（receiver dropped） | 网络循环退出，不影响 Simulation 主循环 |
| Delta 广播失败（GossipSub publish 失败） | 记录警告，不影响本地 delta 通道和模拟继续 |
| CrdtOp 签名验证失败 | 丢弃操作，记录警告，不应用到本地状态 |
| Godot 调用 P2P 方法但非 P2P 模式 | 返回空值/空数组，不报错 |
| 两个客户端同一台机器端口冲突 | 使用不同 `port` 配置（sim_node_a: 4001, sim_node_b: 4002） |
| NAT 导致完全无法连接 | 本地开发测试走 localhost 直连，不经过 NAT |

## 5. 技术决策

### 决策1：Delta P2P 广播的触发方式

- **选型方案**：通过专用 channel `Sender<(Delta, u64, u32)>` 通知独立的 P2P 广播 task
- **选择理由**：agent_loop 不持有 Simulation 的 `&self` 引用，无法直接调用 `publish_delta_p2p()`（async 方法）。通过 channel 解耦，保持 agent_loop 的独立性
- **备选方案**：将 Simulation 的 Arc<Mutex<Self>> 传入 agent_loop
- **放弃原因**：会导致 agent_loop 持有 Simulation 的锁，可能引起死锁（agent_loop 已持有 world lock）

### 决策2：Godot P2P 查询的响应机制

- **选型方案**：使用 `tokio::sync::oneshot::Sender<String>` 随 SimCommand 传入
- **选择理由**：oneshot 是异步一次性通信的最简方式，Simulation 线程在命令循环中处理并回应
- **备选方案**：使用专用 mpsc channel 返回结果，或共享 Arc<RwLock> 状态
- **放弃原因**：mpsc 需区分多个请求的响应；Arc<RwLock> 增加复杂度

### 决策3：CRDT 集成范围

- **选型方案**：本次实现 CrdtOp 的发布、消费、签名验证，但 `SyncState::apply_op()` 暂不直接修改 World
- **选择理由**：CRDT 操作到 World 数据结构的映射（哪个 op 影响哪个 Cell/Agent/Resource）需要详细的映射层设计，超出本次"打通通信链路"的范围
- **备选方案**：完整实现 CRDT → World 的自动映射
- **放弃原因**：涉及 World 内部结构的 CRDT 化改造，工作量过大且需要与现有 Delta 机制的协调

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| libp2p Swarm 在 tokio runtime 中的兼容性 | 中 | 当前 `Libp2pTransport::new()` 已在 `tokio::spawn` 中运行 swarm event loop，经验证可行 |
| GDExtension 中 `godot_print!` 等 API 在非主线程 panic | 高 | simulation 线程中所有日志使用 `tracing::info!` 或 `eprintln!`，不调用 `godot_print!`（已在 CLAUDE.md 中注明） |
| 同一台机器两个 Godot 实例端口冲突 | 中 | sim_node_a.toml 用 4001，sim_node_b.toml 用 4002 |
| network crate 代码去重引入编译错误 | 中 | 先去重再添加功能，确保去重后 `cargo build` 通过 |
| Godot 客户端接收远端 delta 后状态不一致 | 高 | 远端 Agent 使用 ShadowAgent 表示，与本地 Agent 分离，避免状态冲突 |

## 7. 迁移方案

### 7.1 部署步骤

1. 清理 network crate 重复代码（先去重，再添加功能）
2. 修改 `simulation_runner.rs`，根据 sim.toml 选择 P2P/中心化模式
3. 修改 `bridge.rs`，新增 P2P 方法、信号、SimCommand
4. 修改 `agent_loop.rs`，添加 P2P delta 广播 channel
5. 扩展 `run_p2p_network_loop()`，增加 CrdtOp 处理
6. 新增 `client/scripts/p2p_panel.gd`
7. 修改 Godot 场景文件，添加 P2P 面板节点
8. 编译 bridge，复制到 client/bin/
9. 分别用 sim_node_a.toml 和 sim_node_b.toml 启动两个 Godot 实例验证

### 7.2 回滚方案

- 所有改动均在独立的 change 分支上，回滚只需 `git revert`
- 默认行为（无 `[p2p]` 配置）不变，中心化模式不受影响

## 8. 待定事项

- [ ] CRDT 操作到 World 数据结构的完整映射层设计（本次仅实现收发，留为后续）
- [ ] 远端 Agent 的视觉区分方案（半透明？不同颜色？边框？）待用户确认
- [ ] P2P 面板在 Godot 场景中的具体位置（右上角？左侧？）待用户确认
