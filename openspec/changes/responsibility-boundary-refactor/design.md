# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

Agentora 项目是一个端侧多模态大模型驱动的去中心化文明模拟器，架构为 Rust 后端 + Godot 前端，通过 GDExtension Bridge 桥接。当前技术栈：
- **后端**: Rust workspace（core/ai/network/sync/bridge 五个 crates）
- **前端**: Godot 4 + GDScript
- **通信**: GDExtension + mpsc channel

约束条件：
- 不引入新的外部依赖
- 不修改现有功能行为
- 必须保持向后兼容（现有测试和客户端继续工作）

### 1.2 现状分析

当前架构存在 8 个职责边界问题（详见 proposal.md），主要表现为：

| 问题 | 当前状态 | 影响 |
| --- | --- | --- |
| World 上帝对象 | mod.rs 340行，apply_action 110行 | 难以维护、测试困难 |
| decision/prompt 边界模糊 | build_perception_summary 在 decision.rs | 职责不清、代码分散 |
| WorldState 手动构建 | agent_loop.rs 80行组装代码 | 重复代码、易出错 |
| agent_loop 职责链过长 | 单函数 340行 10+职责 | 复杂度高、难调试 |
| Action 反馈散乱 | 字符串格式、多处解析 | 格式不一致 |
| Bridge/Simulation 边界不清 | Bridge 创建 runtime | 职责重叠 |
| 前端信号冗余 | 3组件独立监听 world_updated | 状态不一致风险 |
| Delta 类型重复 | AgentDelta + WorldDelta | 维护负担 |

### 1.3 关键干系人

- **后端开发者**: 负责 Rust 核心模块重构
- **前端开发者**: 负责 Godot StateManager 实现
- **LLM 集成**: DecisionPipeline 接口变更需适配
- **未来 P2P**: Delta 类型需预留广播接口

## 2. 设计目标

### 目标

- 目标1：World 模块拆分为 5 个子系统，mod.rs < 300行
- 目标2：decision.rs 职责收缩 < 500行，感知逻辑移出
- 目标3：WorldState 自动构建，agent_loop 不再手动组装
- 目标4：agent_loop 拆分为 6 阶段流水线，主函数 < 100行
- 目标5：Action 反馈结构化，统一 ActionResult Schema
- 目标6：Bridge 不创建 runtime，只负责信号桥接
- 目标7：前端 StateManager Autoload，统一状态分发
- 目标8：Delta 类型统一，删除 WorldDelta，预留 P2P 接口
- 目标9（P2P适配）：架构支持本地运行部分 Agent + 远程 Agent 通过 P2P 同步

### 非目标

- 非目标1：不修改 LLM Prompt 的具体内容或规则
- 非目标2：不修改前端 UI 布局或交互设计
- 非目标3：不引入新的外部库或框架
- 非目标4：不修改 P2P 网络模块（当前未启用）
- 非目标5：不强制"单客户端单 Agent"，本地仍可通过配置运行多个 Agent

## 3. 整体架构

### 3.1 架构概览

重构后的模块关系：

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              重构后架构                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Godot 客户端                                 │   │
│  │  ┌─────────────────┐                                                │   │
│  │  │ StateManager    │ ← Autoload，唯一状态分发中心                    │   │
│  │  │ (Autoload)      │                                                │   │
│  │  └─────────────────┘                                                │   │
│  │         ↓ state_updated                                             │   │
│  │  ┌─────────┐ ┌─────────┐ ┌──────────┐ ┌─────────┐                   │   │
│  │  │WorldRdr │ │AgentMgr │ │NarrFeed  │ │AgentPanel│                   │   │
│  │  └─────────┘ └─────────┘ └──────────┘ └─────────┘                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              ↑ world_updated                                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Bridge (GDExtension)                         │   │
│  │  ┌─────────────────────────────────────────────────────────────┐    │   │
│  │  │ SimulationBridge                                            │    │   │
│  │  │  - physics_process(): 接收 channel → 发射 Godot 信号         │    │   │
│  │  │  - 不创建 runtime，不初始化 LLM                               │    │   │
│  │  └─────────────────────────────────────────────────────────────┘    │   │
│  │                              ↑ channels                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Simulation (编排层)                          │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ Simulation                                                    │  │   │
│  │  │  - 持有 World、DecisionPipeline                               │  │   │
│  │  │  - spawn AgentLoopController、TickLoop、SnapshotLoop          │  │   │
│  │  │  - 提供 snapshot_sender()、delta_sender()                     │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Core 模块                                   │   │
│  │                                                                     │   │
│  │  ┌───────────────────────────┐   ┌──────────────────────────────┐  │   │
│  │  │ World (协调者)             │   │ World subsystems            │  │   │
│  │  │  - advance_tick()         │   │  ├─ WorldMap (地形查询)      │  │   │
│  │  │  - snapshot()             │   │  ├─ WorldAgents (Agent存储) │  │   │
│  │  │  - 持有子系统引用          │   │  ├─ WorldResources (资源)   │  │   │
│  │  └───────────────────────────┘   │  ├─ WorldStructures (建筑) │  │   │
│  │                                  │  └─ ActionExecutor (动作)   │  │   │
│  │                                  └──────────────────────────────┘  │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ DecisionPipeline (收缩后)                                     │  │   │
│  │  │  - execute(state, perception, memory, feedback) → Decision    │  │   │
│  │  │  - 不包含 build_perception_summary                            │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ PerceptionBuilder (新增)                                      │  │   │
│  │  │  - build_perception_summary(world_state) → String             │  │   │
│  │  │  - build_path_recommendation(world_state) → String            │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ WorldStateBuilder (新增)                                      │  │   │
│  │  │  - build(world, agent_id, radius) → WorldState                │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ AgentLoopController (拆分后)                                  │  │   │
│  │  │  run_agent_loop() 协调以下阶段：                               │  │   │
│  │  │  1. WorldStateBuilder.build()                                 │  │   │
│  │  │  2. PerceptionBuilder.build()                                 │  │   │
│  │  │  3. DecisionPipeline.execute()                                │  │   │
│  │  │  4. ActionExecutor.apply()                                    │  │   │
│  │  │  5. MemoryRecorder.record()                                   │  │   │
│  │  │  6. DeltaEmitter.emit()                                       │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ AgentDelta (统一)                                             │  │   │
│  │  │  - 所有增量事件类型                                           │  │   │
│  │  │  - for_broadcast() → 紧凑 JSON                                │  │   │
│  │  │  - 删除 snapshot.rs 中的 WorldDelta                           │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 P2P 模式架构

重构后架构需要同时支持两种运行模式：

| 运行模式 | 本地 Agent | 远程 Agent | 适用场景 |
| --- | --- | --- | --- |
| **集中式** | 所有 Agent（配置决定数量） | 无 | 单机开发测试、本地演示 |
| **P2P 分布式** | 部分 Agent（玩家自己的 Agent） | 通过 P2P 网络同步 | 多客户端去中心化运行 |

#### P2P 模式架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        P2P 模式架构（客户端 A 视角）                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Godot 客户端                                 │   │
│  │  ┌─────────────────┐                                                │   │
│  │  │ StateManager    │ ← Autoload，统一状态分发                         │   │
│  │  │ (Autoload)      │   local_agents + remote_agents → 渲染           │   │
│  │  └─────────────────┘                                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              ↑ state_updated                                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Bridge (GDExtension)                         │   │
│  │  ┌─────────────────────────────────────────────────────────────┐    │   │
│  │  │ SimulationBridge                                            │    │   │
│  │  │  - physics_process(): 接收本地 delta → 发射信号              │    │   │
│  │  │  - 接收 P2P 消息 → 更新 remote_agents → 发射信号             │    │   │
│  │  └─────────────────────────────────────────────────────────────┘    │   │
│  │                              ↑ channels                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Simulation (编排层)                          │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ Simulation                                                    │  │   │
│  │  │  - SimMode: Centralized | P2P                                 │  │   │
│  │  │  - P2P 模式：持有 local_agents + remote_agents               │  │   │
│  │  │  - 只为 local_agents spawn agent_loop                         │  │   │
│  │  │  - DeltaDispatcher 双通道分发                                 │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Core 模块                                   │   │
│  │                                                                     │   │
│  │  ┌───────────────────────────┐   ┌──────────────────────────────┐  │   │
│  │  │ World (协调者)             │   │ Agent 存储拆分               │  │   │
│  │  │  - advance_tick()         │   │  ├─ local_agents: HashMap    │  │   │
│  │  │  - snapshot()             │   │  │   （完整状态，可修改）      │  │   │
│  │  │  - 持有子系统引用          │   │  ├─ remote_agents: HashMap   │  │   │
│  │  └───────────────────────────┘   │  │   （ShadowAgent，只读）     │  │   │
│  │                                  │  └─ 模式切换：集中式=全local   │  │   │
│  │                                  └──────────────────────────────┘  │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ DeltaDispatcher (新增)                                       │  │   │
│  │  │  - dispatch(delta): 同时发送到本地 mpsc + P2P GossipSub      │  │   │
│  │  │  - P2P 模式：启用双通道                                       │  │   │
│  │  │  - 集中式模式：只发送本地 mpsc                                │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ P2PMessageHandler (新增)                                     │  │   │
│  │  │  - handle(message): 解析远程 AgentDelta → 更新 remote_agents │  │   │
│  │  │  - 过滤：只处理相邻区域的 Delta                               │  │   │
│  │  │  - 超时淘汰：last_seen_tick > N 移除 remote_agent             │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ AgentDelta (统一)                                            │  │   │
│  │  │  - for_local(): 完整字段 → Godot 渲染                         │  │   │
│  │  │  - for_broadcast(): 精简字段 → P2P 广播                       │  │   │
│  │  │  - 包含 source_peer_id 用于过滤本地回环                       │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Network + Sync 层                            │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ GossipSub (region_topic)                                     │  │   │
│  │  │  - 订阅相邻区域 topic                                        │  │   │
│  │  │  - 只接收相邻区域的 AgentDelta                                │  │   │
│  │  │  - 消息处理链路打通：→ P2PMessageHandler                      │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │ SyncState (补全后)                                           │  │   │
│  │  │  - key schema: "agent:{id}:position" → LWW-Register          │  │   │
│  │  │  - OrSetRemove 实现完成                                      │  │   │
│  │  │  - get_shadow_agent(id) → ShadowAgent 便捷方法                │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│                        P2P 网络层                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  客户端 A                客户端 B                客户端 C            │   │
│  │  local: Agent1          local: Agent2          local: Agent3        │   │
│  │  remote: Agent2,3       remote: Agent1,3       remote: Agent1,2     │   │
│  │       ↕                      ↕                      ↕              │   │
│  │  GossipSub region_topic 广播 AgentDelta                             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### 运行模式切换

```rust
// crates/core/src/simulation/config.rs
pub enum SimMode {
    /// 集中式：所有 Agent 本地运行（开发测试模式）
    Centralized,
    /// P2P 分布式：部分 Agent 本地运行，其他通过 P2P 同步
    P2P {
        local_agent_ids: Vec<AgentId>,  // 本地运行的 Agent ID 列表
        region_size: u32,               // 区域划分粒度
    },
}

// SimConfig 扩展
pub struct SimConfig {
    // 已有字段...
    pub mode: SimMode,  // 新增：运行模式
}
```

### 3.4 核心组件

| 组件名 | 文件位置 | 职责说明 |
| --- | --- | --- |
| WorldMap | world/map.rs | 地形查询、边界检查、单元格信息 |
| WorldAgents | world/agents.rs | Agent 存储、位置索引维护 |
| WorldResources | world/resources.rs | 资源节点管理、采集逻辑 |
| WorldStructures | world/structures.rs | 建筑管理、效果范围计算 |
| ActionExecutor | world/actions.rs | 动作路由、执行、反馈生成（重构后） |
| PerceptionBuilder | decision/perception.rs | 感知摘要构建、路径推荐 |
| WorldStateBuilder | simulation/state_builder.rs | World → WorldState 自动构建 |
| DecisionPipeline | decision.rs | 决策流程执行（收缩后） |
| PromptBuilder | prompt.rs | Prompt 组装（扩展后） |
| AgentLoopController | simulation/agent_loop.rs | 决策循环协调（拆分后） |
| DeltaEmitter | simulation/delta_emitter.rs | Delta 构建和发送 |
| Simulation | simulation/simulation.rs | 编排层 API |
| SimulationBridge | bridge/bridge.rs | 前端桥接（收缩后） |
| StateManager | client/scripts/state_manager.gd | 前端状态分发中心 |
| **ShadowAgent** | agent/shadow.rs | **P2P：远程 Agent 精简状态** |
| **DeltaDispatcher** | simulation/delta_dispatcher.rs | **P2P：双通道分发（本地 + P2P）** |
| **P2PMessageHandler** | simulation/p2p_handler.rs | **P2P：接收远程 Delta 更新 remote_agents** |

### 3.5 数据流设计

决策周期数据流：

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Agent 决策周期数据流                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────┐                                                         │
│   │    World     │                                                         │
│   │  (Arc<Mutex>)│                                                         │
│   └──────────────┘                                                         │
│          │                                                                 │
│          ↓ lock().await                                                    │
│   ┌──────────────────┐                                                     │
│   │ WorldStateBuilder│ ← 自动构建，不再手动组装                              │
│   │   .build()       │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ WorldState                                                      │
│   ┌──────────────────┐                                                     │
│   │ PerceptionBuilder│ ← 从 decision.rs 移出                               │
│   │   .build()       │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ perception_summary                                              │
│   ┌──────────────────┐                                                     │
│   │ DecisionPipeline │ ← 职责收缩，接收预构建感知                            │
│   │   .execute()     │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ DecisionResult                                                  │
│   ┌──────────────────┐                                                     │
│   │ ActionExecutor   │ ← 从 world/mod.rs 拆出                              │
│   │   .apply()       │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ ActionResult                                                    │
│   ├─────────────────────────────────────────────────────────────────────┤ │
│   │                                                                     │ │
│   │  ┌───────────────┐  ┌───────────────┐  ┌──────────────────────┐    │ │
│   │  │MemoryRecorder │  │ DeltaEmitter  │  │ NarrativeEmitter     │    │ │
│   │  │ .record()     │  │ .emit()       │  │ .emit()              │    │ │
│   │  └───────────────┘  └───────────────┘  └──────────────────────┘    │ │
│   │                                                                     │ │
│   └─────────────────────────────────────────────────────────────────────┘ │
│          │                                                                 │
│          ↓ channels                                                        │
│   ┌──────────────────┐                                                     │
│   │ SimulationBridge │ ← 只接收 channel，发射 Godot 信号                   │
│   │ physics_process()│                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ Godot signals                                                   │
│   ┌──────────────────┐                                                     │
│   │   StateManager   │ ← Autoload，唯一分发中心                             │
│   │   (Autoload)     │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ state_updated                                                   │
│   ┌─────────────┐ ┌─────────────┐ ┌──────────────┐                        │
│   │ WorldRenderer│ │AgentManager │ │ NarrativeFeed│                        │
│   └─────────────┘ └─────────────┘ └──────────────┘                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### P2P 模式数据流

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        P2P 模式 Agent 决策周期数据流                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────┐                                                         │
│   │    World     │                                                         │
│   │  local_agents│ ← 只持有本地 Agent                                       │
│   │  remote_agents│ ← 远程 Agent 影子状态（只读）                            │
│   └──────────────┘                                                         │
│          │                                                                 │
│          ↓ lock().await                                                    │
│   ┌──────────────────┐                                                     │
│   │ WorldStateBuilder│ ← 构建 local_agent + nearby remote_agents           │
│   │   .build()       │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ WorldState（包含 nearby_agents 从 remote_agents 查询）          │
│   ┌──────────────────┐                                                     │
│   │ PerceptionBuilder│                                                     │
│   │   .build()       │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ perception_summary                                              │
│   ┌──────────────────┐                                                     │
│   │ DecisionPipeline │ ← 只为 local_agent 决策                             │
│   │   .execute()     │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ DecisionResult                                                  │
│   ┌──────────────────┐                                                     │
│   │ ActionExecutor   │ ← 只修改 local_agent                                │
│   │   .apply()       │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ ActionResult                                                    │
│   ├─────────────────────────────────────────────────────────────────────┤ │
│   │                                                                     │ │
│   │  ┌───────────────┐  ┌────────────────────┐  ┌──────────────┐        │ │
│   │  │MemoryRecorder │  │ DeltaDispatcher    │  │NarrativeEmit │        │ │
│   │  │ .record()     │  │ .dispatch()        │  │.emit()       │        │ │
│   │  └───────────────┘  └────────────────────┘  └──────────────┘        │ │
│   │                           │                                        │ │
│   │                    ┌──────┴──────┐                                 │ │
│   │                    │             │                                 │ │
│   │              ┌─────┴─────┐ ┌─────┴─────┐                          │ │
│   │              │ 本地 mpsc │ │ P2P       │                          │ │
│   │              │ channel   │ │ GossipSub │                          │ │
│   │              └───────────┘ └───────────┘                          │ │
│   │                    │             │                                 │ │
│   │                    ↓             ↓                                 │ │
│   │              Bridge        其他客户端                               │ │
│   │                    │             │                                 │ │
│   │                    ↓             ↓                                 │ │
│   │              StateManager   P2PMessageHandler                      │ │
│   │                    │             │                                 │ │
│   │                    ↓             ↓                                 │ │
│   │               渲染更新       更新 remote_agents                     │ │
│   │                                                                     │ │
│   └─────────────────────────────────────────────────────────────────────┤ │
│                                                                             │
│   远程 Agent Delta 接收路径：                                               │
│   ┌──────────────┐                                                         │
│   │ GossipSub    │ ← 接收 region_topic 消息                                │
│   │ message_rx   │                                                         │
│   └──────────────┘                                                         │
│          │                                                                 │
│          ↓ NetworkMessage::AgentDelta                                      │
│   ┌──────────────────┐                                                     │
│   │ P2PMessageHandler│ ← 过滤：source_peer_id != local                    │
│   │ .handle()        │ ← 更新 remote_agents                                │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ remote_agents 更新                                              │
│   ┌──────────────────┐                                                     │
│   │ DeltaEmitter     │ ← 发送本地 mpsc 通知渲染                            │
│   │ .emit_remote()   │                                                     │
│   └──────────────────┘                                                     │
│          │                                                                 │
│          ↓ Bridge → StateManager → 渲染更新                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 4. 详细设计

### 4.1 接口设计

#### 接口：WorldStateBuilder::build

```rust
// 新增接口 - simulation/state_builder.rs
pub struct WorldStateBuilder;

impl WorldStateBuilder {
    /// 从 World 自动构建 WorldState
    pub fn build(
        world: &World,
        agent_id: &AgentId,
        vision_radius: u32,
    ) -> WorldState {
        // 实现：获取 Agent + scan_vision + 压力事件 + 临时偏好
    }
}
```

#### 接口：PerceptionBuilder::build_perception_summary

```rust
// 新增接口 - decision/perception.rs
pub struct PerceptionBuilder;

impl PerceptionBuilder {
    /// 从 WorldState 构建感知摘要
    pub fn build_perception_summary(world_state: &WorldState) -> String {
        // 实现：生存状态、背包、位置、相邻格、资源、建筑、Agent
    }
    
    /// 构建路径推荐
    pub fn build_path_recommendation(world_state: &WorldState) -> String {
        // 实现：根据生存压力推荐优先资源方向
    }
}
```

#### 接口：DecisionPipeline::execute（修改后）

```rust
// 修改接口 - decision.rs
impl DecisionPipeline {
    /// 执行决策管道（参数简化）
    pub async fn execute(
        &self,
        agent_id: &AgentId,
        world_state: &WorldState,
        perception_summary: &str,  // 新增：预构建的感知
        memory_summary: Option<&str>,
        action_feedback: Option<&str>,
    ) -> DecisionResult {
        // 不再调用 build_perception_summary
    }
}
```

#### 接口：ActionResult Schema

```rust
// 新增数据结构 - world/action_result.rs
#[derive(Debug, Clone, Serialize)]
pub enum ActionResult {
    Success {
        action_type: String,
        changes: Vec<FieldChange>,
    },
    Blocked {
        error_code: String,
        reason: String,
        suggestion: Option<ActionSuggestion>,
    },
    AlreadyAtPosition { detail: String },
    InvalidAgent,
    AgentDead,
    OutOfBounds,
    NotImplemented,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldChange {
    field: String,      // 如 "inventory.food", "position.x"
    before: serde_json::Value,
    after: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionSuggestion {
    action_type: String,
    params: HashMap<String, serde_json::Value>,
}
```

#### 接口：AgentDelta for_broadcast

```rust
// 扩展接口 - simulation/delta.rs
impl AgentDelta {
    /// 生成 P2P 广播格式（精简）
    pub fn for_broadcast(&self) -> serde_json::Value {
        match self {
            AgentDelta::AgentMoved { id, position, .. } => json!({
                "type": "agent_moved",
                "id": id,
                "position": position,
                // 不包含 name、color_code 等渲染字段
            }),
            // 其他变体类似
        }
    }
    
    /// 获取 source_peer_id 用于过滤本地回环
    pub fn source_peer_id(&self) -> Option<String> {
        // 从 delta 中提取来源 peer_id
    }
}
```

#### 接口：ShadowAgent（P2P 远程 Agent）

```rust
// 新增数据结构 - agent/shadow.rs
/// 远程 Agent 的影子状态（精简版，只保留渲染和感知必需字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowAgent {
    pub id: AgentId,
    pub name: String,
    pub position: Position,
    pub health: u32,
    pub max_health: u32,
    pub satiety: u32,
    pub hydration: u32,
    pub is_alive: bool,
    pub age: u32,
    pub level: u32,
    pub last_seen_tick: u64,      // 最后同步的 tick，用于超时淘汰
    pub source_peer_id: PeerId,   // 来源客户端标识
}

impl ShadowAgent {
    /// 从 AgentDelta 更新影子状态
    pub fn apply_delta(&mut self, delta: &AgentDelta) {
        match delta {
            AgentDelta::AgentMoved { position, health, .. } => {
                self.position = Position::from_tuple(*position);
                self.health = *health;
                self.last_seen_tick = current_tick();
            }
            AgentDelta::AgentDied { .. } => {
                self.is_alive = false;
            }
            // 其他变体...
        }
    }
    
    /// 检查是否超时（超过 N tick 未更新）
    pub fn is_expired(&self, current_tick: u64, timeout_ticks: u64) -> bool {
        current_tick - self.last_seen_tick > timeout_ticks
    }
}
```

#### 接口：DeltaDispatcher（双通道分发）

```rust
// 新增接口 - simulation/delta_dispatcher.rs
use std::sync::mpsc::Sender;
use crate::network::Libp2pTransport;

/// Delta 双通道分发器（本地 mpsc + P2P 广播）
pub struct DeltaDispatcher {
    local_tx: Sender<AgentDelta>,
    p2p_transport: Option<Arc<Libp2pTransport>>,  // P2P 模式下启用
    mode: SimMode,
}

impl DeltaDispatcher {
    pub fn new(local_tx: Sender<AgentDelta>, mode: SimMode) -> Self {
        Self {
            local_tx,
            p2p_transport: None,
            mode,
        }
    }
    
    /// 设置 P2P 传输层（仅在 P2P 模式下调用）
    pub fn set_p2p_transport(&mut self, transport: Arc<Libp2pTransport>) {
        if matches!(self.mode, SimMode::P2P { .. }) {
            self.p2p_transport = Some(transport);
        }
    }
    
    /// 分发 Delta：本地 channel + P2P 广播（如果启用）
    pub fn dispatch(&self, delta: &AgentDelta) {
        // 1. 发送到本地 mpsc channel（用于 Bridge 渲染）
        if let Err(e) = self.local_tx.send(delta.clone()) {
            tracing::error!("Failed to send delta to local channel: {}", e);
        }
        
        // 2. P2P 广播（仅在 P2P 模式下）
        if let Some(transport) = &self.p2p_transport {
            let broadcast_msg = delta.for_broadcast();
            // 计算当前区域 topic
            let region_topic = self.get_region_topic(&delta);
            transport.publish(region_topic, broadcast_msg.to_string().as_bytes());
        }
    }
    
    fn get_region_topic(&self, delta: &AgentDelta) -> String {
        // 根据 delta 中的位置计算 region topic
        format!("region_{}", self.calculate_region(delta))
    }
}
```

#### 接口：P2PMessageHandler（处理远程 Delta）

```rust
// 新增接口 - simulation/p2p_handler.rs
use crate::sync::SyncState;

/// P2P 消息处理器：接收远程 AgentDelta → 更新 remote_agents
pub struct P2PMessageHandler {
    world: Arc<Mutex<World>>,
    local_peer_id: PeerId,
    sync_state: SyncState,
    timeout_ticks: u64,  // 远程 Agent 超时淘汰阈值
}

impl P2PMessageHandler {
    pub fn new(
        world: Arc<Mutex<World>>,
        local_peer_id: PeerId,
        sync_state: SyncState,
    ) -> Self {
        Self {
            world,
            local_peer_id,
            sync_state,
            timeout_ticks: 100,  // 默认 100 tick 超时
        }
    }
    
    /// 处理接收到的 P2P 消息
    pub async fn handle(&self, message: &NetworkMessage) {
        match message {
            NetworkMessage::AgentDelta(delta) => {
                // 1. 过滤本地回环（不处理自己发出的 delta）
                if delta.source_peer_id() == Some(self.local_peer_id.clone()) {
                    return;
                }
                
                // 2. 更新 remote_agents
                let mut world = self.world.lock().await;
                world.apply_remote_delta(delta);
                
                // 3. 更新 SyncState（CRDT 同步）
                self.sync_state.apply_delta(delta);
            }
            NetworkMessage::SyncRequest(request) => {
                // 响应同步请求（全量状态）
                self.handle_sync_request(request);
            }
            _ => {}
        }
    }
    
    /// 检查并淘汰超时的远程 Agent
    pub async fn prune_expired_agents(&self, current_tick: u64) {
        let mut world = self.world.lock().await;
        world.remote_agents.retain(|_, agent| {
            !agent.is_expired(current_tick, self.timeout_ticks)
        });
    }
}
```

#### 接口：World Agent 存储拆分

```rust
// 修改接口 - world/mod.rs
pub struct World {
    // 原有字段保持不变...
    
    /// 本地 Agent 存储（完整状态，可修改）
    /// 集中式模式：所有 Agent
    /// P2P 模式：只有本地运行的 Agent
    pub local_agents: HashMap<AgentId, Agent>,
    
    /// 远程 Agent 存储（影子状态，只读）
    /// 集中式模式：空
    /// P2P 模式：其他客户端的 Agent
    pub remote_agents: HashMap<AgentId, ShadowAgent>,
    
    /// 运行模式
    pub mode: SimMode,
}

impl World {
    /// 应用远程 Agent 的 Delta（只更新 remote_agents）
    pub fn apply_remote_delta(&mut self, delta: &AgentDelta) {
        let agent_id = delta.agent_id();
        
        if self.local_agents.contains_key(&agent_id) {
            // 本地 Agent 不处理远程 delta
            return;
        }
        
        // 更新或创建 ShadowAgent
        match self.remote_agents.get_mut(&agent_id) {
            Some(shadow) => shadow.apply_delta(delta),
            None => {
                // 新发现的远程 Agent，创建影子状态
                let shadow = ShadowAgent::from_delta(delta);
                self.remote_agents.insert(agent_id, shadow);
            }
        }
    }
    
    /// 获取所有 Agent（local + remote）用于渲染
    pub fn all_agents(&self) -> Vec<AgentSnapshot> {
        let local = self.local_agents.values()
            .map(|a| AgentSnapshot::from_agent(a));
        let remote = self.remote_agents.values()
            .map(|s| AgentSnapshot::from_shadow(s));
        local.chain(remote).collect()
    }
    
    /// 只为 local_agents 执行生存消耗（tick_loop）
    pub fn advance_tick_local_only(&mut self) {
        for agent in self.local_agents.values_mut() {
            if agent.is_alive {
                agent.satiety = agent.satiety.saturating_sub(1);
                agent.hydration = agent.hydration.saturating_sub(1);
                // 其他生存消耗逻辑...
            }
        }
    }
}
```

### 4.2 数据模型

#### WorldState 字段（统一后）

```rust
// rule_engine.rs - WorldState 保持不变，但构建方式改变
pub struct WorldState {
    pub map_size: u32,
    pub agent_position: Position,
    pub agent_inventory: HashMap<ResourceType, u32>,
    pub agent_satiety: u32,
    pub agent_hydration: u32,
    pub terrain_at: HashMap<Position, TerrainType>,
    pub self_id: AgentId,
    pub existing_agents: HashSet<AgentId>,
    pub resources_at: HashMap<Position, (ResourceType, u32)>,
    pub nearby_agents: Vec<NearbyAgentInfo>,
    pub nearby_structures: Vec<NearbyStructureInfo>,
    pub nearby_legacies: Vec<NearbyLegacyInfo>,
    pub active_pressures: Vec<String>,
    pub last_move_direction: Option<Direction>,
    pub temp_preferences: Vec<(String, f32, u32)>,
    pub agent_personality: Option<PersonalitySeed>,
}
```

#### AgentDelta 变体（统一后）

```rust
// simulation/delta.rs - 统一所有增量事件
pub enum AgentDelta {
    AgentMoved {
        id: String,
        name: String,
        position: (u32, u32),  // 统一使用 tuple
        health: u32,
        max_health: u32,
        is_alive: bool,
        age: u32,
    },
    AgentDied { id: String, name: String, position: (u32, u32), age: u32 },
    AgentSpawned { id: String, name: String, position: (u32, u32), health: u32, max_health: u32 },
    StructureCreated { x: u32, y: u32, structure_type: String, owner_id: String },
    StructureDestroyed { x: u32, y: u32, structure_type: String },
    ResourceChanged { x: u32, y: u32, resource_type: String, amount: u32 },
    TradeCompleted { from_id: String, to_id: String, items: String },
    AllianceFormed { id1: String, id2: String },
    AllianceBroken { id1: String, id2: String, reason: String },
    HealedByCamp { agent_id: String, agent_name: String, hp_restored: u32 },
    SurvivalWarning { agent_id: String, agent_name: String, satiety: u32, hydration: u32, hp: u32 },
    MilestoneReached { name: String, display_name: String, tick: u64 },
    PressureStarted { pressure_type: String, description: String, duration: u32 },
    PressureEnded { pressure_type: String, description: String },
}
```

### 4.3 核心算法

#### WorldStateBuilder.build() 算法

```rust
pub fn build(world: &World, agent_id: &AgentId, vision_radius: u32) -> WorldState {
    // 1. 获取 Agent 基本信息
    let agent = world.agents.get(agent_id).expect("Agent exists");
    
    // 2. 执行视野扫描
    let vision = scan_vision(world, agent_id, vision_radius);
    
    // 3. 构建库存映射
    let inventory: HashMap<ResourceType, u32> = agent.inventory.iter()
        .map(|(k, v)| (ResourceType::from_str(k).unwrap(), *v))
        .collect();
    
    // 4. 构建完整 WorldState
    WorldState {
        map_size: world.map.size().0,
        agent_position: agent.position,
        agent_inventory: inventory,
        agent_satiety: agent.satiety,
        agent_hydration: agent.hydration,
        terrain_at: vision.terrain_at,
        self_id: agent_id.clone(),
        existing_agents: world.agents.keys().cloned().collect(),
        resources_at: vision.resources_at,
        nearby_agents: vision.nearby_agents,
        nearby_structures: vision.nearby_structures,
        nearby_legacies: vision.nearby_legacies,
        active_pressures: world.pressure_pool.iter().map(|p| p.description.clone()).collect(),
        last_move_direction: agent.last_position.and_then(|last| calculate_direction(&last, &agent.position)),
        temp_preferences: agent.temp_preferences.iter().map(|p| (p.key.clone(), p.boost, p.remaining_ticks)).collect(),
        agent_personality: Some(agent.personality.clone()),
    }
}
```

#### run_agent_loop() 拆分后（主函数 < 100行）

```rust
pub async fn run_agent_loop(
    world: Arc<Mutex<World>>,
    agent_id: AgentId,
    pipeline: Arc<DecisionPipeline>,
    delta_tx: Sender<AgentDelta>,
    narrative_tx: Sender<NarrativeEvent>,
    is_npc: bool,
    interval_secs: u32,
    vision_radius: u32,
    is_paused: Arc<AtomicBool>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    
    loop {
        interval.tick().await;
        if is_paused.load(SeqCst) { continue; }
        
        // Phase 1: 构建状态
        let (world_state, agent_clone) = {
            let w = world.lock().await;
            let state = WorldStateBuilder::build(&w, &agent_id, vision_radius);
            let agent = w.agents.get(&agent_id).cloned();
            (state, agent)
        };
        
        if agent_clone.is_none() || !agent_clone.unwrap().is_alive { break; }
        
        // Phase 2: 构建感知
        let perception = PerceptionBuilder::build_perception_summary(&world_state);
        let memory = agent_clone.unwrap().memory.get_summary(infer_state_mode(&world_state));
        
        // Phase 3: 决策
        let decision = if is_npc {
            RuleEngine::new().survival_fallback(&world_state).map(Action::from_candidate)
        } else {
            pipeline.execute(&agent_id, &world_state, &perception, memory.as_deref(), agent_clone.unwrap().last_action_result.as_deref()).await
        };
        
        // Phase 4-6: 应用、记录、发射（需再次获取锁）
        if let Some(action) = decision.and_then(|r| r.selected_action.map(Action::from_candidate)) {
            let events = {
                let mut w = world.lock().await;
                w.apply_action(&agent_id, &action);
                MemoryRecorder::record(&mut w, &agent_id, &action);
                let events = NarrativeEmitter::extract(&w);
                DeltaEmitter::emit(&delta_tx, &w, &agent_id, &action);
                events
            };
            NarrativeEmitter::send_events(&narrative_tx, events);
        }
    }
}
```

### 4.6 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| Agent 不存在 | WorldStateBuilder 返回 None，agent_loop 退出 |
| Agent 死亡 | 检查 is_alive，退出循环 |
| LLM 调用超时 | DecisionPipeline 返回 Error，不执行动作，记录反馈 |
| 动作校验失败 | 不应用动作，设置 last_action_result 反馈错误 |
| Delta 发送失败 | log error，继续运行（不阻塞决策循环） |
| StateManager 未初始化 | 前端组件使用 fallback 数据，等待 Autoload 就绪 |

### 4.7 前端设计

#### 技术栈

- 框架：Godot 4.x + GDScript
- 状态管理：StateManager Autoload（新增）
- 通信：SimulationBridge signals → StateManager → 各组件

#### 目录结构

```
client/scripts/
├── state_manager.gd        ← 新增 Autoload
├── main.gd                 ← 修改：订阅 StateManager
├── world_renderer.gd       ← 修改：订阅 StateManager.on_terrain_changed
├── agent_manager.gd        ← 修改：订阅 StateManager.on_agent_changed
├── narrative_feed.gd       ← 修改：订阅 StateManager.on_narrative
├── agent_detail_panel.gd   ← 修改：查询 StateManager.get_agent_data()
├── milestone_panel.gd      ← 修改：查询 StateManager.get_milestones()
└── bridge_accessor.gd      ← 保持：获取 Bridge 节点
```

#### StateManager 组件设计

| 组件名 | 类型 | 文件路径 | 说明 |
| --- | --- | --- | --- |
| StateManager | Autoload | scripts/state_manager.gd | 全局状态管理器 |
| StateData | 内部类 | state_manager.gd | 状态数据容器 |
| StateSignals | 内部信号 | state_manager.gd | 状态变更信号 |

```gdscript
# StateManager 主要接口
class_name StateManager extends Node

signal state_updated(snapshot: Dictionary)
signal agent_changed(agent_id: String, data: Dictionary)
signal terrain_changed(x: int, y: int, terrain: String)
signal resource_changed(x: int, y: int, type: String, amount: int)
signal narrative_added(event: Dictionary)

var _agents: Dictionary = {}
var _terrain: Dictionary = {}
var _resources: Dictionary = {}
var _structures: Dictionary = {}
var _map_size: Vector2i = Vector2i(-1, -1)

func get_agent_data(agent_id: String) -> Dictionary
func get_terrain_at(x: int, y: int) -> String
func get_resource_at(x: int, y: int) -> Dictionary
func get_map_size() -> Vector2i

func _on_world_updated(snapshot: Dictionary)  # 解析并分发
func _on_agent_delta(delta: Dictionary)       # 增量更新
func _on_narrative_event(event: Dictionary)    # 叙事追加
```

#### 交互逻辑

1. SimulationBridge 发射 world_updated → StateManager.on_world_updated
2. StateManager 解析 snapshot，更新内部状态
3. StateManager 发射 state_updated → 各组件更新显示
4. SimulationBridge 发射 agent_delta → StateManager.on_agent_delta
5. StateManager 增量更新对应 Agent/Resource/Structure
6. StateManager 发射具体 change 信号（如 agent_position_changed）
7. 各组件只订阅 StateManager，不直接监听 Bridge

## 5. 技术决策

### 决策1：World 拆分策略

- **选型方案**：将 World 拆分为多个子系统模块（WorldMap、WorldAgents 等），World 作为协调者
- **选择理由**：
  - 保持现有 API 兼容（World 仍然存在，只是内部职责转移）
  - 逐步拆分，降低风险
  - 子系统可独立测试
- **备选方案**：完全删除 World，改为多个独立子系统组合
- **放弃原因**：改动太大，破坏现有代码的引用，风险高

### 决策2：感知构建模块位置

- **选型方案**：创建独立的 PerceptionBuilder 模块（decision/perception.rs）
- **选择理由**：
  - 与 PromptBuilder 同级，便于协作
  - 可独立测试和修改
  - 清晰的职责边界
- **备选方案**：将感知逻辑合并到 PromptBuilder
- **放弃原因**：PromptBuilder 已经较大（600+行），合并后职责不清

### 决策3：Delta 类型统一方式

- **选型方案**：统一为 AgentDelta，删除 WorldDelta，添加 for_broadcast() 方法
- **选择理由**：
  - 避免两套定义维护负担
  - for_broadcast() 预留 P2P 接口
  - 前端和 P2P 使用同一类型，只是输出格式不同
- **备选方案**：保留两套 Delta，通过 trait 统一接口
- **放弃原因**：增加抽象层，维护成本更高

### 决策4：前端状态管理方式

- **选型方案**：创建 StateManager Autoload 作为唯一分发中心
- **选择理由**：
  - Godot Autoload 是官方推荐的单例模式
  - 集中状态管理，消除冗余监听
  - 增量更新支持，减少全量刷新
- **备选方案**：使用 Godot Resource 作为共享状态
- **放弃原因**：Resource 更适合静态配置，不适合实时状态同步

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| World 拆分导致引用断裂 | 高 | 保持 World 公开 API 不变，只改内部实现 |
| agent_loop 拆分引入新模块 | 中 | 逐步拆分，每个模块独立测试后再集成 |
| 前端 StateManager 引入 | 中 | 先添加 Autoload，逐步迁移各组件订阅 |
| Delta 类型删除 WorldDelta | 低 | 确认无代码引用后删除 |
| DecisionPipeline 接口变更 | 中 | 提供过渡期兼容接口 |
| **P2P：Agent 存储拆分** | 中 | 通过 SimMode 切换，集中式模式行为不变 |
| **P2P：GossipSub 消息链路** | 中 | 先补全链路，再启用 P2P 模式测试 |
| **P2P：远程 Agent 超时淘汰** | 低 | 使用 last_seen_tick 机制，定时清理 |

## 7. 迁移方案

### 7.1 部署步骤

按以下顺序分阶段迁移：

1. **Phase 1: 后端模块拆分**（不修改接口）
   - 创建 WorldStateBuilder、PerceptionBuilder、DeltaEmitter 等新模块
   - agent_loop 内调用新模块，但保持主流程不变
   - 运行 cargo test 验证

2. **Phase 2: DecisionPipeline 职责收缩**
   - 修改 DecisionPipeline.execute() 接收预构建感知
   - agent_loop 先调用 PerceptionBuilder，再传给 Pipeline
   - 运行 cargo test 验证

3. **Phase 3: Action 反馈结构化**
   - 定义 ActionResult Schema
   - 修改各 handler 返回结构化数据
   - 修改 generate_action_feedback() 使用 Schema 解析
   - 运行 cargo test 验证

4. **Phase 4: Delta 统一**
   - 扩展 AgentDelta 添加 for_broadcast()
   - 删除 snapshot.rs 中的 WorldDelta
   - 更新 conversion.rs
   - 运行 cargo test 验证

5. **Phase 5: Bridge 职责收缩**
   - 修改 Bridge 不创建 runtime
   - Simulation 在模拟线程内创建 runtime
   - 运行客户端验证

6. **Phase 6: 前端 StateManager**
   - 创建 state_manager.gd Autoload
   - 逐步迁移各组件订阅
   - 删除组件直接监听 Bridge
   - 运行客户端验证

7. **Phase 7: World 拆分**
   - 创建 WorldAgents、WorldResources 等子系统
   - World 内部调用子系统
   - 保持 World 公开 API 不变
   - 运行 cargo test + 客户端验证

8. **Phase 8: P2P 适配基础**（架构支持 P2P 模式）
   - 拆分 World 的 Agent 存储为 local_agents + remote_agents
   - 新增 ShadowAgent 结构体
   - 新增 SimMode 枚举（Centralized/P2P）
   - 修改 agent_loop 只为 local_agents 创建循环
   - 运行 cargo test 验证（默认集中式模式）

9. **Phase 9: DeltaDispatcher 双通道分发**
   - 新增 DeltaDispatcher 模块
   - 扩展 AgentDelta 添加 source_peer_id 字段
   - 本地 mpsc channel + P2P GossipSub 双通道
   - 集中式模式：只发送本地 mpsc
   - 运行 cargo test 验证

10. **Phase 10: P2P 消息处理链路**
    - 补全 GossipSub 消息处理（从 NullMessageHandler 到实际消费）
    - 新增 P2PMessageHandler 模块
    - NetworkMessage 增加 AgentDelta 变体
    - 更新 SyncState 补全 OrSetRemove 实现
    - 运行集成测试（多客户端模拟）

### 7.2 灰度策略

- 每个 Phase 完成后运行完整测试套件
- Phase 1-7 为核心重构，保持集中式模式正常工作
- Phase 8-10 为 P2P 适配，不影响集中式模式
- P2P 模式通过配置切换，默认集中式
- 每个 Phase 的代码变更在独立 commit，便于回滚

### 7.3 回滚方案

- 每个 Phase 的代码变更在独立 commit
- 如发现问题，回滚对应 commit
- 保持功能测试通过作为每个 Phase 的门禁

## 8. 待定事项

- [ ] WorldAgents 子系统是否需要独立的 agent_positions 反向索引，还是在 World 中保留
- [ ] PerceptionBuilder 是否需要支持不同 SparkType 的定制感知模板
- [ ] AgentDelta.for_broadcast() 的字段取舍（哪些是 P2P 必需的）
- [ ] StateManager 是否需要支持历史状态快照（用于回放）
- [ ] **P2P：远程 Agent 的"生死状态"同步策略（LWW vs 特殊广播）**
- [ ] **P2P：区域划分粒度（region_size）与地图尺寸的匹配**
- [ ] **P2P：本地 Agent 数量上限（配置 vs 系统限制）**
- [ ] **P2P：是否需要 Vector Clock 补偿时钟同步偏差**