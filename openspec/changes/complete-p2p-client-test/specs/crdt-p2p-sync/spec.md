# CRDT P2P Sync Spec

## Purpose

将 CRDT 操作（LWW-Register、G-Counter、OR-Set）通过 GossipSub 传输层发布和消费，实现远端状态的最终一致性同步。

## ADDED Requirements

### Requirement: CrdtOp 发布

系统 SHALL 在本地 CRDT 状态变化时通过 GossipSub 发布 CrdtOp 到对应区域 topic。

#### Scenario: 发布 LWW-Register 更新

- **WHEN** 本地 LWW-Register 值被更新
- **THEN** 系统 SHALL 构造 `NetworkMessage::CrdtOp { op: LwwSet, signature, timestamp }`
- **AND** 通过对应区域 topic 发布到 GossipSub

#### Scenario: 发布 G-Counter 增量

- **WHEN** 本地 G-Counter 执行 increment 操作
- **THEN** 系统 SHALL 构造 `NetworkMessage::CrdtOp { op: GCounterInc, signature, timestamp }`
- **AND** 通过对应区域 topic 发布到 GossipSub

#### Scenario: 发布 OR-Set 添加/删除

- **WHEN** 本地 OR-Set 执行 add 或 remove 操作
- **THEN** 系统 SHALL 构造 `NetworkMessage::CrdtOp { op: OrSetAdd/OrSetRemove, signature, timestamp }`
- **AND** 通过对应区域 topic 发布到 GossipSub

### Requirement: CrdtOp 消费

系统 SHALL 接收远端 CrdtOp 并应用到本地 SyncState。

#### Scenario: 接收并应用 CrdtOp

- **WHEN** 收到 `NetworkMessage::CrdtOp` 消息
- **AND** 消息来源不是本地 peer（回环过滤）
- **THEN** 系统 SHALL 验证操作签名
- **AND** 将 CrdtOp 解码为 sync crate 的内部操作表示
- **AND** 调用 `SyncState::apply_op()` 应用到本地状态

#### Scenario: 签名验证失败

- **WHEN** 收到 CrdtOp 但签名验证不通过
- **THEN** 系统 SHALL 丢弃该操作并记录警告日志
- **AND** 系统 SHALL 不将其应用到本地 SyncState

### Requirement: 周期性 Merkle 校验

系统 SHALL 每 100 tick 执行一次 Merkle 根校验，检测数据不一致。

#### Scenario: Merkle 根校验通过

- **WHEN** 到达第 100 tick 的整数倍
- **THEN** 系统 SHALL 计算本地 SyncState 的 Merkle 根
- **AND** 通过 `world_events` topic 广播 Merkle 根
- **AND** 收到远端 Merkle 根后与本地对比，一致则无需进一步操作

#### Scenario: Merkle 根不一致

- **WHEN** 收到远端 Merkle 根与本地不同
- **THEN** 系统 SHALL 触发完整 SyncState 同步请求
- **AND** 通过 `SyncRequest/SyncResponse` 交换全量状态
- **AND** 调用 `SyncState::merge()` 合并远端状态

### Requirement: SyncRequest/SyncResponse 处理

系统 SHALL 支持全量状态请求和响应。

#### Scenario: 请求远端全量状态

- **WHEN** Merkle 校验发现不一致
- **THEN** 系统 SHALL 构造 `NetworkMessage::SyncRequest { merkle_root, requester_peer_id }`
- **AND** 发送到触发不一致的 peer

#### Scenario: 响应全量状态请求

- **WHEN** 收到 `NetworkMessage::SyncRequest`
- **THEN** 系统 SHALL 序列化本地 SyncState 为 `NetworkMessage::SyncResponse { state_json, merkle_root }`
- **AND** 发送回请求方

#### Scenario: 合并远端全量状态

- **WHEN** 收到 `NetworkMessage::SyncResponse`
- **THEN** 系统 SHALL 解析 state_json
- **AND** 调用 `SyncState::merge()` 合并到本地
- **AND** 重新计算 Merkle 根确认一致
