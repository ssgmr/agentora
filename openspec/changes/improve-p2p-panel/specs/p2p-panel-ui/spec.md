# 功能规格说明

## MODIFIED Requirements

### Requirement: P2P 面板显示已连接节点

P2P 面板 SHALL 显示真正的已连接节点列表，而非 relay_reservations（中继预留）。

#### Scenario: 显示节点列表

- **WHEN** P2P 面板刷新（每3秒）
- **THEN** 调用 `bridge.get_connected_peers()` 获取连接列表
- **AND** 解析 JSON 并渲染到 peers_list 容器中
- **AND** 每个节点显示：完整 PeerId、agent_version、连接方式、连接时间

#### Scenario: 区分节点类型显示

- **WHEN** 渲染节点列表项
- **THEN** 根据 `is_relay_server` 字段区分显示：
  - `is_relay_server = true` → 显示 "[中继服务]" 标签
  - `is_relay_server = false` → 显示 "[玩家]" 标签
- **AND** 使用不同颜色区分（玩家: 绿色, 中继: 黄色）

#### Scenario: 连接方式显示

- **WHEN** 渲染节点列表项
- **THEN** 根据 `connection_type` 字段显示连接方式：
  - `"direct"` → "直连"
  - `"relay"` → "中继"
  - `"dcutr"` → "打洞"
- **AND** 对于 relay 连接，显示经由的中继服务器名称

#### Scenario: 无连接时显示提示

- **WHEN** 获取到的连接列表为空
- **THEN** peers_list 显示 "无已连接节点"
- **AND** 使用灰色文字显示提示信息

### Requirement: P2P 面板显示订阅 Topic

P2P 面板 SHALL 新增"订阅 Topic"区域，显示当前订阅的 GossipSub topic 列表。

#### Scenario: 显示订阅列表

- **WHEN** P2P 面板刷新
- **THEN** 调用新增的 `bridge.get_subscribed_topics()` 方法
- **AND** 在 UI 中显示订阅的 topic 列表
- **AND** 每个 topic 显示为带勾选标记的项（✅ topic_name）

#### Scenario: 无订阅时显示提示

- **WHEN** 获取到的订阅列表为空
- **THEN** 显示 "暂无订阅的 Topic"
- **AND** 使用灰色文字显示提示信息

## ADDED Requirements

### Requirement: NAT 状态优化显示

P2P 面板 SHALL 优化 NAT 状态显示，增加探测进度和详细地址信息。

#### Scenario: 显示探测中状态

- **WHEN** NAT 状态为 "unknown" 或正在探测
- **THEN** 显示 "NAT: 正在探测..." 并带有进度动画
- **AND** 禁用"连接种子节点"按钮（等待NAT探测完成）

#### Scenario: 显示公网地址

- **WHEN** NAT 状态为 "public" 并有地址
- **THEN** 显示 "NAT: 公网可达 (地址: /ip4/x.x.x.x/tcp/4001)"
- **AND** 使用绿色文字表示可直连

#### Scenario: 显示私有网络状态

- **WHEN** NAT 状态为 "private"
- **THEN** 显示 "NAT: 内网（需要中继或打洞）"
- **AND** 使用黄色文字表示需要穿透

#### Scenario: 显示未启用状态

- **WHEN** P2P 模式未启用
- **THEN** 显示 "NAT: 未启用（中心化模式）"
- **AND** 隐藏种子地址输入框和连接按钮

### Requirement: 连接节点信息面板布局

P2P 面板 SHALL 使用新的布局显示详细信息。

#### Scenario: 信息分组显示

- **WHEN** P2P 面板渲染
- **THEN** 信息按以下分组显示：
  1. 本节点信息（PeerId、端口、NAT状态）
  2. 已连接节点列表（详细信息）
  3. 订阅的 Topic 列表
  4. 操作区域（种子地址输入、连接按钮）

#### Scenario: 节点详情展开显示

- **WHEN** 渲染每个已连接节点
- **THEN** 使用多行显示详细信息：
  - 第一行：PeerId（完整） + 节点类型标签
  - 第二行：agent: {agent_version}
  - 第三行：连接方式: {connection_type} | 时间: {connected_at}