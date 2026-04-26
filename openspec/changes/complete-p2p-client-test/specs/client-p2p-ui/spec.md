# Client P2P UI Spec

## Purpose

为 Godot 客户端提供 P2P 连接管理和状态展示的 UI 面板，使玩家能连接种子节点、查看网络状态、确认远端 Agent 可见。

## ADDED Requirements

### Requirement: P2P 连接面板

Godot 客户端 SHALL 新增 P2P 控制面板（Control 节点），包含种子地址输入框、连接按钮、状态展示区域。

#### Scenario: 面板初始化

- **WHEN** 游戏场景加载完成
- **THEN** P2P 面板 SHALL 显示在屏幕右上角（不遮挡叙事流和 Agent 详情面板）
- **AND** 面板 SHALL 默认折叠状态，点击标题栏展开/收起
- **AND** 种子地址输入框 SHALL 预填当前 sim.toml 配置的 seed_peer 值（如有）

#### Scenario: 点击连接按钮

- **WHEN** 用户在种子地址输入框输入地址后点击"连接"按钮
- **THEN** 按钮 SHALL 显示 loading 状态并禁用
- **AND** 系统 SHALL 调用 Bridge 的 `connect_to_seed(addr)`
- **AND** 连接完成后 SHALL 恢复按钮状态

#### Scenario: 连接成功反馈

- **WHEN** Bridge 发射 `peer_connected` 信号
- **THEN** P2P 面板 SHALL 在已连接 peers 列表中添加该 peer_id
- **AND** 连接按钮 SHALL 恢复可点击状态
- **AND** 状态区域 SHALL 显示 "已连接 X 个 peers"

#### Scenario: 连接失败反馈

- **WHEN** Bridge 发射 `p2p_status_changed` 信号且 status 包含 error
- **THEN** P2P 面板 SHALL 在状态区域显示红色错误信息
- **AND** 连接按钮 SHALL 恢复可点击状态

### Requirement: Peer 信息展示

P2P 面板 SHALL 展示本地 peer_id、NAT 状态、已连接 peers 列表。

#### Scenario: 展示本地信息

- **WHEN** P2P 面板展开
- **THEN** 面板 SHALL 显示：
  - `Peer ID: <本地peer_id>`（可复制）
  - `NAT 状态: <public/private/unknown>`

#### Scenario: Peer 列表更新

- **WHEN** `peer_connected` 或 `p2p_status_changed` 信号触发
- **THEN** 面板 SHALL 刷新已连接 peers 列表
- **AND** 每个 peer 条目 SHALL 显示：peer_id（截断）、connection_type（图标区分 direct/dcutr/relay）

### Requirement: 远端 Agent 视觉区分

Godot 客户端 SHALL 对来自 P2P 的远端 Agent 进行视觉区分。

#### Scenario: 渲染远端 Agent

- **WHEN** 收到包含 `source_peer_id` 的 agent_delta 信号
- **AND** 该 Agent 不在本地 world 中
- **THEN** 创建的 Agent 节点 SHALL 使用不同的颜色标识（如半透明或不同色调）
- **AND** Agent 标签 SHALL 显示 `[P2P]` 前缀

#### Scenario: 远端 Agent 状态更新

- **WHEN** 收到远端 Agent 的位置/状态更新
- **THEN** 已有 Agent 节点 SHALL 平滑移动到新位置（不瞬移）

### Requirement: 信号订阅

P2P UI 控制器 SHALL 订阅 Bridge 的 P2P 相关信号。

#### Scenario: 信号连接

- **WHEN** P2P 面板 `_ready()` 执行
- **THEN** 系统 SHALL 连接以下信号：
  - `Bridge.peer_connected` → `_on_peer_connected(peer_id)`
  - `Bridge.p2p_status_changed` → `_on_p2p_status_changed(status)`

#### Scenario: 信号处理空值

- **WHEN** Bridge 尚未初始化（P2P 未启用）
- **AND** P2P 面板尝试连接信号
- **THEN** 系统 SHALL 静默跳过，不报错
- **AND** P2P 面板 SHALL 显示 "P2P 未启用" 提示
