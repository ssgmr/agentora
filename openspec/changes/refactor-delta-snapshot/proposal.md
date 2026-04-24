# 需求说明书

## 背景概述

当前项目中 Delta 和 Snapshot 机制存在严重的数据重复和语义混乱问题。`AgentDelta::AgentMoved` 包含13个字段，与 `AgentSnapshot` 完全重复，导致数据构建逻辑分散在5个地方（delta_emitter.rs、delta.rs、conversion.rs、snapshot.rs、shadow.rs），维护成本极高。同时，Delta 有14种变体，其中 `AgentDied`、`AgentSpawned`、`HealedByCamp` 等都是 `AgentMoved` 的语义子集，命名和分类混乱。

此外，Snapshot 每次发送完整状态（含65KB地形网格），效率低下；`conversion.rs` 遗漏了 events/legacies/pressures/milestones 字段，客户端实际收不到这些数据；叙事事件未通过 P2P 广播，远程 Agent 的行为在本地"静默"，用户看不到其他玩家的 Agent 在做什么。

项目的终极目标是：每个用户客户端只运行一个本地 Agent，其他世界 Agent 通过去中心化 P2P 方式交互。这要求 Delta 成为远程 Agent 状态的唯一数据来源，而当前架构无法支持。

## 变更目标

- **统一数据模型**：建立单一的 Agent 状态表示，消除 Delta/Snapshot 重复
- **简化 Delta 分类**：将 Delta 分为 Agent 状态变化和世界事件两类
- **叙事频道系统**：支持本地/附近/世界三个频道，叙事通过 P2P 按区域广播
- **Agent 过滤**：叙事面板支持按 Agent 过滤，方便开发测试和用户追踪
- **客户端统一入口**：简化客户端状态管理，只通过 Delta 接收数据

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `agent-state-model` | 统一的 Agent 状态数据模型，替代 AgentSnapshot 和 AgentDelta::AgentMoved |
| `narrative-channel` | 叙事频道系统（本地/附近/世界），支持 P2P 区域广播 |
| `agent-filter` | 叙事面板 Agent 过滤功能，支持按 Agent ID 筛选叙事流 |
| `world-event-system` | 世界事件系统，用于里程碑、压力、建筑、资源等非 Agent 状态变化 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `delta-system` | Delta 从14种变体简化为 AgentStateChanged + WorldEvent 两类 |
| `snapshot-system` | Snapshot 退化为 WorldInit（初始化）+ StateSnapshot（兜底），不再每次发送完整状态 |
| `state-manager` | 客户端 StateManager 统一通过 Delta 接收数据，简化处理逻辑 |
| `p2p-gossip` | P2P Gossip 区域订阅实际生效，支持叙事按区域广播 |

## 影响范围

- **代码模块**：
  - `crates/core/src/snapshot.rs` — 数据模型重构
  - `crates/core/src/simulation/delta.rs` — Delta 分类简化
  - `crates/core/src/simulation/delta_emitter.rs` — 使用统一数据构建
  - `crates/core/src/agent/shadow.rs` — 使用统一数据模型
  - `crates/core/src/world/snapshot.rs` — Snapshot 简化
  - `crates/bridge/src/conversion.rs` — 转换逻辑简化
  - `client/scripts/state_manager.gd` — 统一 Delta 处理
  - `client/scripts/narrative_feed.gd` — 频道切换 + Agent 过滤

- **API接口**：
  - Delta 信号结构变更（type 字段语义调整）
  - 新增 narrative_channel、agent_source 字段
  - Snapshot 信号结构简化（移除 events/legacies 等字段）

- **依赖组件**：
  - P2P GossipSub region topic 实际启用
  - 新增 world_events 全局 topic

- **关联系统**：
  - Agent 渲染系统（统一 AgentState）
  - 叙事流 UI（频道切换 + Agent 过滤）

## 验收标准

- [ ] AgentSnapshot 和 AgentDelta::AgentMoved 字段合并为单一 AgentState 结构
- [ ] Delta 变体从14种减少到 AgentStateChanged + WorldEvent 两类
- [ ] 叙事事件通过 P2P 按区域广播，附近频道能看到远程 Agent 的叙事
- [ ] 叙事面板支持本地/附近/世界三个 Tab 切换
- [ ] 叙事面板支持按 Agent ID 过滤（点击 Agent 或下拉选择）
- [ ] conversion.rs 完整转换所有字段，客户端能收到完整数据
- [ ] Snapshot 不再每次发送 terrain_grid，只在初始化发送
- [ ] 单元测试覆盖新数据模型的转换逻辑