# 实施任务清单

## 1. 数据结构定义

新增 ConnectedPeer 结构体和相关类型定义。

- [x] 1.1 定义 ConnectedPeer 结构体
  - 文件: `crates/network/src/config.rs`
  - 添加 ConnectedPeer 结构体，包含 peer_id、agent_version、connection_type、connected_at、is_relay_server、listen_addr 字段
  - 添加 Serialize derive 以支持 JSON 序列化

- [x] 1.2 确保 ConnectionType 枚举支持序列化
  - 文件: `crates/network/src/nat.rs`
  - 为 ConnectionType 枚举添加 Serialize/Deserialize derive

## 2. Swarm 层事件跟踪

在 Swarm 事件循环中跟踪连接和订阅事件。

- [x] 2.1 新增 connected_peers 和 subscribed_topics 参数
  - 文件: `crates/network/src/swarm.rs`
  - 修改 `run_swarm_event_loop` 函数签名，添加两个新参数
  - 添加导入 chrono 用于时间格式化

- [x] 2.2 处理 ConnectionEstablished 事件
  - 文件: `crates/network/src/swarm.rs`
  - 在 handle_swarm_event 中处理 ConnectionEstablished
  - 创建 ConnectedPeer 结构体并添加到 connected_peers
  - 依赖: 1.1, 2.1

- [x] 2.3 处理 ConnectionClosed 事件
  - 文件: `crates/network/src/swarm.rs`
  - 在 handle_swarm_event 中处理 ConnectionClosed
  - 从 connected_peers 中移除对应的 peer
  - 依赖: 2.2

- [x] 2.4 处理 Identify::Received 事件更新 agent_version
  - 文件: `crates/network/src/swarm.rs`
  - 在 Identify 事件处理中更新 connected_peers 中的 agent_version
  - 判断 is_relay_server（agent_version 包含 "relay")
  - 依赖: 2.2

- [x] 2.5 跟踪 Subscribe 命令成功
  - 文件: `crates/network/src/swarm.rs`
  - 在 handle_swarm_command 中，Subscribe 成功后添加到 subscribed_topics
  - 依赖: 2.1

- [x] 2.6 跟踪 Unsubscribe 命令成功
  - 文件: `crates/network/src/swarm.rs`
  - 在 handle_swarm_command 中，Unsubscribe 成功后从 subscribed_topics 移除
  - 依赖: 2.5

## 3. Libp2pTransport API 新增

在 Libp2pTransport 中添加查询方法和共享状态字段。

- [x] 3.1 添加 connected_peers 和 subscribed_topics 字段
  - 文件: `crates/network/src/libp2p_transport.rs`
  - 在 Libp2pTransport 结构体中添加两个新字段
  - 在 Clone 实现中正确 clone 这两个 Arc
  - 依赖: 1.1

- [x] 3.2 实现 get_connected_peers() 方法
  - 文件: `crates/network/src/libp2p_transport.rs`
  - 添加公开方法读取 connected_peers 并返回 Vec<ConnectedPeer>
  - 依赖: 3.1

- [x] 3.3 实现 get_subscribed_topics() 方法
  - 文件: `crates/network/src/libp2p_transport.rs`
  - 添加公开方法读取 subscribed_topics 并返回 Vec<String>
  - 依赖: 3.1

- [x] 3.4 创建共享状态并传递给 Swarm
  - 文件: `crates/network/src/libp2p_transport.rs`
  - 在 `new()` 和 `with_key()` 中创建 Arc<RwLock>
  - 传递给 `run_swarm_event_loop` 的参数
  - 依赖: 2.1, 3.1

## 4. SimulationRunner 查询更新

修改 simulation_runner.rs 中的 peers 查询逻辑。

- [x] 4.1 修改 "peers" 查询调用新方法
  - 文件: `crates/bridge/src/simulation_runner.rs`
  - 将 "peers" 查询从 `get_relay_reservations()` 改为 `get_connected_peers()`
  - JSON 序列化 ConnectedPeer 结构体
  - 依赖: 3.2

- [x] 4.2 新增 "topics" 查询处理
  - 文件: `crates/bridge/src/simulation_runner.rs`
  - 在 SimCommand::QueryPeerInfo 中添加 "topics" 查询类型
  - 调用 `get_subscribed_topics()` 并返回 JSON
  - 依赖: 3.3

## 5. SimulationBridge API 更新

更新 Bridge 的 GDExtension API。

- [x] 5.1 更新 get_connected_peers() 返回值格式
  - 文件: `crates/bridge/src/bridge.rs`
  - 确保返回的 JSON 格式包含所有 ConnectedPeer 字段
  - 验证 Godot 能正确解析
  - 依赖: 4.1

- [x] 5.2 新增 get_subscribed_topics() 方法
  - 文件: `crates/bridge/src/bridge.rs`
  - 添加 #[func] 方法返回订阅的 topic 列表 JSON
  - 依赖: 4.2

## 6. Godot UI 实现

改进 P2P 面板的显示和交互。

- [x] 6.1 更新 _refresh_peer_info() 解析逻辑
  - 文件: `client/scripts/p2p_panel.gd`
  - 解析新的 JSON 格式（包含 agent_version, connection_type, connected_at, is_relay_server）
  - 更新 _update_peers_list() 渲染逻辑
  - 依赖: 5.1

- [x] 6.2 新增 _refresh_topics_info() 方法
  - 文件: `client/scripts/p2p_panel.gd`
  - 调用 bridge.get_subscribed_topics()
  - 渲染订阅的 topic 列表
  - 依赖: 5.2

- [x] 6.3 改进节点列表显示格式
  - 文件: `client/scripts/p2p_panel.gd`
  - 每个节点显示 3 行：PeerId+类型、agent_version、连接方式+时间
  - 区分颜色：玩家绿色、中继黄色
  - 依赖: 6.1

- [x] 6.4 新增订阅 topic 显示区域
  - 文件: `client/scripts/p2p_panel.gd`, `client/scenes/p2p_panel.tscn`
  - 在 VBox 中添加 TopicsList 容器
  - 渲染订阅的 topic 列表（带勾选标记）
  - 依赖: 6.2

- [x] 6.5 改进 NAT 状态显示
  - 文件: `client/scripts/p2p_panel.gd`
  - 根据状态显示不同文字和颜色（探测中/公网/内网/未启用）
  - 显示具体地址（如果有）

- [x] 6.6 改进本节点信息显示
  - 文件: `client/scripts/p2p_panel.gd`, `client/scenes/p2p_panel.tscn`
  - 显示完整 PeerId、监听端口、NAT 状态
  - 调整布局分组

## 7. 测试与验证

- [x] 7.1 单元测试 - ConnectedPeer 序列化
  - 测试 ConnectedPeer 的 JSON 序列化/反序列化

- [x] 7.2 编译验证 - Network Crate
  - 运行 `cargo build -p agentora-network` 验证编译成功

- [x] 7.3 编译验证 - Bridge Crate
  - 运行 `cargo build -p agentora-bridge` 验证编译成功

- [x] 7.4 验收测试 - Godot 客户端显示
  - 启动 Godot 客户端
  - 验证 P2P 面板显示正确（开启 P2P 模式）
  - 验证节点列表显示完整信息
  - 验证订阅 topic 列表显示

- [x] 7.5 验收测试 - 连接建立/关闭
  - 验证连接建立时节点添加到列表
  - 验证连接关闭时节点从列表移除

## 任务依赖关系

```
1.x (数据结构) → 2.x (Swarm事件) → 3.x (Transport API)
    ↓               ↓               ↓
    └───────────────┼───────────────┼──→ 4.x (SimulationRunner)
                    │               │       ↓
                    │               │   5.x (Bridge API)
                    │               │       ↓
                    └───────────────┼──→ 6.x (Godot UI)
                                    │
                                7.x (测试)
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.x | 定义数据结构，为后续实现提供基础 |
| 阶段二 | 2.x | Swarm 事件跟踪，捕获连接和订阅状态 |
| 阶段三 | 3.x | Libp2pTransport API，提供查询方法 |
| 阶段四 | 4.x, 5.x | SimulationRunner 和 Bridge，打通查询链路 |
| 阶段五 | 6.x | Godot UI，完成前端显示 |
| 阶段六 | 7.x | 测试验证，确保功能正确 |

## 文件结构总览

```
crates/network/src/
├── config.rs          [修改] 新增 ConnectedPeer 结构体
├── nat.rs             [修改] ConnectionType 添加 Serialize
├── swarm.rs           [修改] 事件跟踪、参数传递
├── libp2p_transport.rs [修改] 新增字段和查询方法

crates/bridge/src/
├── simulation_runner.rs [修改] 查询逻辑
├── bridge.rs           [修改] 新增/修改 GDExtension API

client/
├── scripts/p2p_panel.gd  [修改] UI 显示逻辑
├── scenes/p2p_panel.tscn [修改] UI 布局调整
```