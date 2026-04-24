# 实施任务清单

## 1. 数据模型重构（Rust Core）

建立统一的 AgentState 结构，消除 AgentSnapshot 和 AgentDelta::AgentMoved 重复。

- [x] 1.1 创建 AgentState 结构
  - 文件: `crates/core/src/snapshot.rs`
  - 定义 AgentState 结构（13字段）
  - 实现 Serialize/Deserialize trait

- [x] 1.2 创建 ChangeHint 枚举
  - 文件: `crates/core/src/simulation/delta.rs`
  - 定义 ChangeHint 枚举（Spawned/Moved/Died/Healed/SurvivalLow/ActionExecuted）

- [x] 1.3 实现 Agent::to_state() 方法
  - 文件: `crates/core/src/agent/mod.rs`
  - 新增 to_state() 方法返回 AgentState

- [x] 1.4 实现 AgentState::to_delta() 方法
  - 文件: `crates/core/src/snapshot.rs`
  - 实现 to_delta(change_hint) → Delta::AgentStateChanged

- [x] 1.5 实现 ShadowAgent::from_state() 方法
  - 文件: `crates/core/src/agent/shadow.rs`
  - 修改 ShadowAgent 结构使用 AgentState 字段
  - 实现 from_state() 解析方法

## 2. Delta 简化重构（Rust Core）

废弃 AgentDelta 14种变体，创建新的 Delta（AgentStateChanged + WorldEvent 两类）。

- [x] 2.1 定义新的 Delta 枚举
  - 文件: `crates/core/src/simulation/delta.rs`
  - Delta::AgentStateChanged { agent_id, state, change_hint }
  - Delta::WorldEvent(WorldEvent)
  - 废弃旧的 AgentDelta enum

- [x] 2.2 定义 WorldEvent 枚举
  - 文件: `crates/core/src/simulation/delta.rs`
  - WorldEvent 包含：StructureCreated/Destroyed、ResourceChanged、TradeCompleted、AllianceFormed/Broken、MilestoneReached、PressureStarted/Ended、AgentNarrative

- [x] 2.3 修改 DeltaEmitter
  - 文件: `crates/core/src/simulation/delta_emitter.rs`
  - emit_agent_state 使用 agent.to_state().to_delta()
  - emit_world_event 使用 WorldEvent 构建

- [x] 2.4 实现 Delta::for_broadcast()
  - 文件: `crates/core/src/simulation/delta.rs`
  - 实现 AgentStateChanged 和 WorldEvent 的 JSON 序列化

- [x] 2.5 修改 conversion.rs delta_to_dict()
  - 文件: `crates/bridge/src/conversion.rs`
  - 处理 AgentStateChanged → Godot Dictionary
  - 处理 WorldEvent 各变体 → Godot Dictionary

- [x] 2.6 更新所有 AgentDelta 引用
  - 文件: 多个
  - 替换所有使用 AgentDelta 的地方为新的 Delta

## 3. Snapshot 简化重构（Rust Core）

Snapshot 退化为 WorldInit（初始化）+ StateSnapshot（兜底）。

- [x] 3.1 修改 WorldSnapshot 结构
  - 文件: `crates/core/src/snapshot.rs`
  - agents 改用 Vec<AgentState>
  - 移除 events/legacies/milestones 字段（改为通过 WorldEvent 传输）
  - 新增 structures/resources HashMap

- [x] 3.2 修改 World::snapshot()
  - 文件: `crates/core/src/world/snapshot.rs`
  - 使用 agent.to_state() 和 shadow.to_state()
  - terrain_grid 仍然每帧发送（Godot 渲染需要）

- [x] 3.3 修改 conversion.rs snapshot_to_dict()
  - 文件: `crates/bridge/src/conversion.rs`
  - 使用 agent_to_dict(AgentState) 转换
  - 保留 terrain_grid/map_changes 转换

- [x] 3.4 修改 snapshot_loop
  - 文件: `crates/core/src/simulation/snapshot_loop.rs`
  - 保持现有行为（完整快照每5秒发送）

## 4. 叙事频道系统（Rust Core + P2P）

支持本地/附近/世界三个频道，叙事通过 P2P 按区域广播。

- [x] 4.1 扩展 NarrativeEvent 结构
  - 文件: `crates/core/src/snapshot.rs`
  - 新增 channel: NarrativeChannel (Local=0, Nearby=1, World=2)
  - 新增 agent_source: AgentSource (Local/Remote{peer_id})

- [x] 4.2 实现 determine_narrative_channel()
  - 文件: 多个（mod.rs, tick.rs, milestones.rs, pressure.rs）
  - 死亡事件 → World 频道
  - 其他事件 → Local 频道（默认）

- [ ] 4.3 创建 world_events Topic
  - 文件: `crates/network/src/gossip.rs`
  - 新增 WORLD_EVENTS_TOPIC 常量
  - Simulation 启动时自动订阅

- [ ] 4.4 实现叙事 P2P 广播
  - 文件: `crates/core/src/simulation/narrative_emitter.rs`
  - Nearby 频道 → region_<id> topic
  - World 频道 → world_events topic
  - Local 频道 → 仅本地 narrative_tx

## 5. 客户端重构（Godot）

StateManager 统一通过 Delta 接收数据，叙事面板支持频道切换和 Agent 过滤。

- [ ] 5.1 修改 StateManager._on_delta()
  - 文件: `client/scripts/state_manager.gd`
  - 处理 AgentStateChanged → 更新 _agents
  - 处理 WorldEvent → 更新 _structures/_resources/_milestones/_narratives

- [ ] 5.2 修改 StateManager._on_world_updated()
  - 文件: `client/scripts/state_manager.gd`
  - 仅处理 terrain_grid（初始化）+ agents（兜底）
  - 处理新增的 structures/resources 字段

- [ ] 5.3 实现叙事过滤状态存储
  - 文件: `client/scripts/state_manager.gd`
  - 新增 _narrative_channel: String
  - 新增 _narrative_agent_filter: String
  - 实现 get_filtered_narratives() 接口

- [ ] 5.4 创建 NarrativeFeed 频道切换
  - 文件: `client/scripts/narrative_feed.gd`
  - 实现 Tab 切换：本地/附近/世界
  - 调用 StateManager.get_filtered_narratives()

## 6. 测试与验证

- [x] 6.1 单元测试 - AgentState 转换
- [x] 6.2 单元测试 - Delta 简化
- [x] 6.3 单元测试 - 叙事频道判定
- [ ] 6.4 集成测试 - 本地多 Agent
- [ ] 6.5 验收测试 - P2P 叙事广播

## 任务依赖关系

```
1.x (数据模型) ──────────────────┬──────────────────────────┐
                                 │                          │
2.x (Delta简化) ─────────────────┼──────────────────────────┤
  依赖: 1.x                       │                          │
                                 │                          │
3.x (Snapshot简化) ──────────────┼──────────────────────────┤
  依赖: 1.x                       │                          │
                                 │                          │
4.x (叙事频道) ───────────────────┼──────────────────────────┤
  依赖: 2.x                       │                          │
                                 ▼                          ▼
5.x (客户端重构) ───────────────────────────────────────────
  依赖: 2.x, 3.x, 4.x
                                 │
                                 ▼
6.x (测试与验证)
  依赖: 5.x
```

## 已完成内容

### 核心改动（已完成）

1. **AgentState 结构** (`crates/core/src/snapshot.rs`)
   - 13字段统一 Agent 状态表示
   - 替代旧的 AgentSnapshot

2. **ChangeHint 枚举** (`crates/core/src/simulation/delta.rs`)
   - 6种状态变化标记

3. **WorldEvent 枚举** (`crates/core/src/simulation/delta.rs`)
   - 10种世界事件类型
   - 包含 AgentNarrative 用于叙事广播

4. **Delta 枚举** (`crates/core/src/simulation/delta.rs`)
   - 仅 AgentStateChanged + WorldEvent 两类
   - 完全废弃旧的 AgentDelta 14种变体

5. **DeltaEnvelope** (`crates/core/src/simulation/delta.rs`)
   - P2P 包装结构
   - source_peer_id 用于本地回环过滤

6. **NarrativeChannel/AgentSource** (`crates/core/src/snapshot.rs`)
   - 叙事频道分类（Local/Nearby/World）
   - 来源标记（Local/Remote）

7. **ShadowAgent 重构** (`crates/core/src/agent/shadow.rs`)
   - 使用 AgentState 字段
   - from_state() 创建方法
   - apply_delta() 更新方法

8. **bridge/conversion.rs 重写**
   - delta_to_dict 处理新 Delta 格式
   - agent_to_dict 处理 AgentState

9. **测试文件更新** (`tests/responsibility_boundary_tests.rs`)
   - 全部使用新 Delta/AgentState 结构

### 待完成内容

- P2P 网络层的 world_events Topic
- Godot 客户端 GDScript 适配新 Delta 格式
- 叙事 P2P 广播实现