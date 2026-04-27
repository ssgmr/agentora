# 功能规格说明

## ADDED Requirements

### Requirement: Swarm跟踪订阅事件

Swarm 事件循环 SHALL 跟踪 GossipSub 的订阅和退订事件，维护订阅的 topic 列表。

#### Scenario: 订阅成功时添加到列表

- **WHEN** Swarm 接收到 `GossipsubEvent::Subscribed { peer_id, topic }` 事件（本地订阅）
- **THEN** 系统将 topic 名称添加到 `subscribed_topics` 共享状态中
- **AND** 记录日志 "订阅 topic: {topic}"

#### Scenario: 退订时从列表移除

- **WHEN** Swarm 接收到 `GossipsubEvent::Unsubscribed { peer_id, topic }` 事件（本地退订）
- **THEN** 系统从 `subscribed_topics` 共享状态中移除该 topic
- **AND** 记录日志 "退订 topic: {topic}"

#### Scenario: Subscribe 命令成功时更新列表

- **WHEN** 处理 `SwarmCommand::Subscribe { topic }` 命令
- **AND** gossipsub.subscribe() 成功
- **THEN** 系统将 topic 添加到 `subscribed_topics` 共享状态

#### Scenario: Unsubscribe 命令成功时更新列表

- **WHEN** 处理 `SwarmCommand::Unsubscribe { topic }` 命令
- **AND** gossipsub.unsubscribe() 成功
- **THEN** 系统从 `subscribed_topics` 共享状态中移除该 topic

### Requirement: 订阅状态共享

系统 SHALL 通过 `Arc<RwLock<Vec<String>>>` 共享订阅的 topic 列表，供 Libp2pTransport 查询。

#### Scenario: 读取订阅列表

- **WHEN** Libp2pTransport 调用 `get_subscribed_topics()`
- **THEN** 从共享状态读取当前订阅列表
- **AND** 返回 Vec<String> 克隆

#### Scenario: 初始订阅列表

- **WHEN** Swarm 初始化时
- **THEN** `subscribed_topics` 初始为空 Vec
- **AND** 后续通过 init_p2p_network 自动订阅 world_events 和 region_0