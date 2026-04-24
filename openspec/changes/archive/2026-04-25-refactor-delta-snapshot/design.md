# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

项目是一个去中心化数文明模拟器，终极架构目标为：
- 每个用户客户端只运行一个本地 Agent
- 其他世界 Agent 通过 P2P GossipSub 进行状态同步
- 本地 Agent 完整决策（LLM + 规则引擎 + 记忆系统）
- 远程 Agent 只存储渲染状态（ShadowAgent）

当前技术栈：
- Rust core（决策、世界、Agent、Delta/Snapshot）
- libp2p GossipSub（P2P 广播）
- Godot 4 GDScript（客户端渲染）
- Bridge GDExtension（Rust → Godot 数据转换）

### 1.2 现状分析

**数据重复问题**：
- AgentSnapshot 和 AgentDelta::AgentMoved 包含相同的13个字段
- 数据构建逻辑分散在5个文件（delta_emitter.rs、delta.rs、conversion.rs、snapshot.rs、shadow.rs）
- 新增字段需修改5处，维护成本极高

**Delta 分类混乱**：
- AgentDelta 有14种变体，语义重叠
- AgentDied、AgentSpawned、HealedByCamp 都是 AgentMoved 的语义子集
- 命名误导（AgentMoved 实际是"Agent完整状态变化"，不是"移动"）

**Snapshot 问题**：
- 每次 Snapshot 发送65KB地形网格（terrain_grid），浪费带宽
- conversion.rs 遗漏了 events/legacies/pressures/milestones 字段
- 客户端实际收不到这些数据

**叙事静默问题**：
- NarrativeEvent 未通过 P2P 广播
- 远程 Agent 的行为叙事在本地不可见
- 用户看不到视野内其他玩家的 Agent 在做什么

**P2P 区域订阅空实现**：
- RegionTopicManager 定义了区域订阅，但 MessageHandler 是 NullMessageHandler
- 实际没有处理远程消息

### 1.3 关键干系人

| 角色 | 影响范围 |
| --- | --- |
| 核心引擎 | delta.rs, snapshot.rs, shadow.rs, delta_emitter.rs |
| Bridge层 | conversion.rs, bridge.rs |
| 客户端 | state_manager.gd, narrative_feed.gd |
| P2P网络 | gossip.rs, p2p_handler.rs |

## 2. 设计目标

### 目标

1. **统一数据模型**：建立单一的 AgentState 结构，消除 AgentSnapshot 和 AgentDelta::AgentMoved 重复
2. **简化 Delta 分类**：将 Delta 从14种变体简化为 AgentStateChanged + WorldEvent 两类
3. **叙事频道系统**：支持本地/附近/世界三个频道，叙事通过 P2P 按区域广播
4. **Agent 过滤**：叙事面板支持按 Agent ID 过滤，方便开发测试和用户追踪
5. **客户端统一入口**：StateManager 统一通过 Delta 接收数据

### 非目标

- 不修改 Agent 决策逻辑（DecisionPipeline）
- 不修改 LLM 调用方式
- 不修改 CRDT 同步机制（本次仅关注数据模型）
- 不实现完整的跨区域 Agent 交互（仅实现视野内同步）

## 3. 整体架构

### 3.1 架构概览

```
┌─────────────────────────────────────────────────────────────────────┐
│                     新架构数据流                                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐                                                  │
│  │ AgentState   │  ← 统一的Agent状态表示                             │
│  │ (单一结构)   │                                                   │
│  └──────────────┘                                                  │
│         │                                                          │
│         ├──────────────────────┬──────────────────────┐            │
│         │                      │                      │            │
│         ▼                      ▼                      ▼            │
│  ┌──────────────┐       ┌──────────────┐       ┌──────────────┐   │
│  │ Delta        │       │ Snapshot     │       │ ShadowAgent  │   │
│  │ (状态变化)   │       │ (初始化/兜底) │       │ (远程存储)   │   │
│  └──────────────┘       └──────────────┘       └──────────────┘   │
│         │                      │                      │           │
│         │ P2P broadcast         │ local only           │           │
│         ▼                      ▼                      ▼           │
│  ┌──────────────┐       ┌──────────────┐                       │
│  │ 远程客户端   │       │ 本地客户端   │                       │
│  │ (Delta接收) │       │ (Delta+Snap) │                       │
│  └──────────────┘       └──────────────┘                       │
│                                                                     │
│  ┌──────────────┐                                                  │
│  │ WorldEvent   │  ← 世界级事件（里程碑、压力、建筑、叙事）          │
│  │ (独立分类)   │                                                   │
│  └──────────────┘                                                  │
│         │                                                          │
│         ├─ region topic (附近)                                     │
│         └─ world_events topic (全局)                               │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件名 | 职责说明 |
| --- | --- |
| AgentState | 统一的 Agent 状态数据结构 |
| Delta | 简化为 AgentStateChanged + WorldEvent 两类 |
| Snapshot | 退化为 WorldInit（初始化）+ StateSnapshot（兜底） |
| NarrativeChannel | 叙事频道分类（Local/Nearby/World） |
| StateManager | 客户端统一状态管理，Delta 为主要数据来源 |

### 3.3 数据流设计

```
┌─────────────────────────────────────────────────────────────────────┐
│                     启动阶段                                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Simulation.start()                                                 │
│       │                                                             │
│       ▼                                                             │
│  WorldInit { terrain_grid, initial_agents }                        │
│       │                                                             │
│       ├─ local: Bridge.world_updated → StateManager                 │
│       └─ P2P: 不广播（本地初始化）                                   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                     运行阶段（Agent 状态变化）                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Agent 执行动作                                                     │
│       │                                                             │
│       ├─ agent.to_state(action, result, reasoning)                  │
│       │                                                             │
│       ▼                                                             │
│  AgentStateChanged { state, change_hint }                          │
│       │                                                             │
│       ├─ local: Bridge.agent_delta → StateManager                   │
│       │                                                             │
│       └─ P2P: region_<id> topic 广播                                │
│           │                                                         │
│           ▼                                                         │
│       远程客户端 P2PHandler → ShadowAgent → 渲染                    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                     运行阶段（叙事广播）                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Agent 行为产生叙事                                                  │
│       │                                                             │
│       ├─ NarrativeEvent { channel, agent_source }                   │
│       │                                                             │
│       ├─ Local 频道 → local narrative_tx (不广播)                   │
│       │                                                             │
│       ├─ Nearby 频道 → region_<id> topic 广播                       │
│       │                                                             │
│       └─ World 频道 → world_events topic 广播                       │
│                                                                     │
│  客户端接收：                                                        │
│       ├─ Bridge.narrative_event → StateManager._narratives         │
│       └─ NarrativeFeed 按频道+Agent过滤显示                         │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                     兜底阶段（5秒）                                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  snapshot_loop 每5秒触发                                            │
│       │                                                             │
│       ▼                                                             │
│  StateSnapshot { agents, structures, resources, pressures }        │
│       │                                                             │
│       └─ local: Bridge.world_updated → StateManager（一致性校验）   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## 4. 详细设计

### 4.1 数据模型设计

#### AgentState 结构（Rust）

```rust
/// 统一的 Agent 状态表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub id: String,
    pub name: String,
    pub position: (u32, u32),
    pub health: u32,
    pub max_health: u32,
    pub satiety: u32,
    pub hydration: u32,
    pub age: u32,
    pub level: u32,
    pub is_alive: bool,
    pub inventory_summary: HashMap<String, u32>,
    pub current_action: String,
    pub action_result: String,
    pub reasoning: Option<String>,  // 本地有，远程为 None
}
```

#### ChangeHint 枚举（Rust）

```rust
/// Agent 状态变化标记
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeHint {
    Spawned,       // 新 Agent 首次出现
    Moved,         // 位置变化
    ActionExecuted, // 动作执行后
    Died,          // 死亡
    SurvivalLow,   // 生存状态警告
    Healed,        // 营地治愈
}
```

#### Delta 结构（Rust）

```rust
/// 简化的 Delta 枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Delta {
    AgentStateChanged {
        agent_id: String,
        state: AgentState,
        change_hint: ChangeHint,
    },
    WorldEvent(WorldEvent),
}
```

#### WorldEvent 枚举（Rust）

```rust
/// 世界级事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldEvent {
    StructureCreated { pos: (u32, u32), structure_type: String, owner_id: String },
    StructureDestroyed { pos: (u32, u32), structure_type: String },
    ResourceChanged { pos: (u32, u32), resource_type: String, amount: u32 },
    TradeCompleted { from_id: String, to_id: String, items: String },
    AllianceFormed { id1: String, id2: String },
    AllianceBroken { id1: String, id2: String, reason: String },
    MilestoneReached { name: String, display_name: String, tick: u64 },
    PressureStarted { pressure_type: String, description: String, duration: u32 },
    PressureEnded { pressure_type: String, description: String },
    AgentNarrative { narrative: NarrativeEvent },
}
```

#### NarrativeChannel 枚举（Rust）

```rust
/// 叙事频道分类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NarrativeChannel {
    Local,   // 本地频道（不广播）
    Nearby,  // 附近频道（按区域广播）
    World,   // 世界频道（全局广播）
}
```

#### NarrativeEvent 扩展结构（Rust）

```rust
/// 叙事事件（含频道和来源）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEvent {
    pub tick: u64,
    pub agent_id: String,
    pub agent_name: String,
    pub event_type: String,
    pub description: String,
    pub color_code: String,
    pub channel: NarrativeChannel,       // 新增
    pub agent_source: AgentSource,        // 新增
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentSource {
    Local,
    Remote { peer_id: String },
}
```

### 4.2 接口设计

#### Agent::to_state() 方法

```rust
impl Agent {
    /// 转换为 AgentState（统一数据构建入口）
    pub fn to_state(
        &self,
        current_action: Option<&str>,
        action_result: Option<&str>,
        reasoning: Option<&str>,
    ) -> AgentState {
        AgentState {
            id: self.id.as_str().to_string(),
            name: self.name.clone(),
            position: (self.position.x, self.position.y),
            health: self.health,
            max_health: self.max_health,
            satiety: self.satiety,
            hydration: self.hydration,
            age: self.age,
            level: self.level,
            is_alive: self.is_alive,
            inventory_summary: self.inventory.clone(),
            current_action: current_action
                .map(|s| s.to_string())
                .or_else(|| self.last_action_type.clone())
                .unwrap_or_default(),
            action_result: action_result
                .map(|s| s.to_string())
                .or_else(|| self.last_action_result.clone())
                .unwrap_or_default(),
            reasoning: reasoning.map(|s| s.to_string()),
        }
    }
}
```

#### AgentState::to_delta() 方法

```rust
impl AgentState {
    /// 转换为 Delta（统一 Delta 构建入口）
    pub fn to_delta(&self, change_hint: ChangeHint) -> Delta {
        Delta::AgentStateChanged {
            agent_id: self.id.clone(),
            state: self.clone(),
            change_hint,
        }
    }
}
```

#### ShadowAgent::from_delta() 方法

```rust
impl ShadowAgent {
    /// 从 Delta 创建 ShadowAgent（统一接收入口）
    pub fn from_delta(delta: &Delta, source_peer_id: &str, current_tick: u64) -> Option<Self> {
        match delta {
            Delta::AgentStateChanged { state, .. } => {
                Some(ShadowAgent {
                    id: state.id.clone(),
                    name: state.name.clone(),
                    position: state.position,
                    // ... 其他字段直接复制
                    last_seen_tick: current_tick,
                    source_peer_id: source_peer_id.to_string(),
                })
            }
            _ => None,
        }
    }
}
```

### 4.3 核心算法

#### 叙事频道判定算法

```rust
/// 根据事件类型判定叙事频道
fn determine_narrative_channel(event_type: EventType) -> NarrativeChannel {
    match event_type {
        // 本地专属（玩家只想看自己的详细思考）
        EventType::Wait => NarrativeChannel::Local,
        
        // 附近可见（视野内的交互）
        EventType::Move | EventType::MoveToward | EventType::Gather |
        EventType::Talk | EventType::TradeOffer | EventType::TradeAccept |
        EventType::Attack | EventType::Build | EventType::Explore |
        EventType::Eat | EventType::Drink | EventType::Healed => NarrativeChannel::Nearby,
        
        // 世界可见（全局事件）
        EventType::Milestone | EventType::PressureStart | EventType::PressureEnd |
        EventType::Death => NarrativeChannel::World,
        
        _ => NarrativeChannel::Nearby,  // 默认附近可见
    }
}
```

#### 客户端叙事过滤算法（GDScript）

```gdscript
## StateManager 叙事过滤接口
func get_filtered_narratives() -> Array:
    var filtered = _narratives.duplicate()
    
    # 1. 频道过滤
    if _narrative_channel != null:
        filtered = filtered.filter(func(e):
            return e.channel == _narrative_channel
        )
    
    # 2. Agent过滤（可选）
    if _narrative_agent_filter != null and _narrative_agent_filter != "":
        filtered = filtered.filter(func(e):
            return e.agent_id == _narrative_agent_filter
        )
    
    # 3. 世界频道忽略 Agent 过滤
    if _narrative_channel == "world":
        filtered = _narratives.filter(func(e):
            return e.channel == "world"
        )
    
    return filtered
```

### 4.4 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| Delta 解析失败 | 客户端跳过该事件，记录错误日志 |
| 远程 Agent 过期 | World.cleanup_expired_shadows() 清理 |
| 叙事广播失败 | P2P 降级，叙事仅在本地显示 |
| Snapshot 兜底失败 | 客户端继续使用 Delta 数据，显示警告 |
| 区域订阅失败 | 回退到全局 topic，显示全部叙事 |

### 4.5 前端设计

#### 技术栈

- 框架：Godot 4 GDScript
- UI组件：内置 Control节点
- 状态管理：StateManager Autoload

#### 页面设计

| 页面 | 路径 | 说明 |
| --- | --- | --- |
| 主界面 | Main.tscn | 包含叙事面板、Agent面板、地图 |
| 叙事面板 | UI/NarrativeFeed | 频道切换 + Agent过滤 |

#### 组件设计

| 组件名 | 类型 | 文件路径 | 说明 |
| --- | --- | --- | --- |
| ChannelTabs | 组合组件 | NarrativeFeed/ChannelTabs.gd | 本地/附近/世界 Tab切换 |
| AgentFilter | 组合组件 | NarrativeFeed/AgentFilter.gd | Agent选择下拉/点击 |
| NarrativeItem | 原子组件 | NarrativeFeed/NarrativeItem.gd | 单条叙事显示（含来源图标） |
| NarrativeList | 组合组件 | NarrativeFeed/NarrativeList.gd | 叙事列表滚动 |

#### ChannelTabs Props

```gdscript
# ChannelTabs.gd
var current_channel: String = "nearby"  # "local"|"nearby"|"world"
signal channel_changed(channel: String)
```

#### AgentFilter Props

```gdscript
# AgentFilter.gd
var selected_agent_id: String = ""  # 空=全部
var available_agents: Dictionary = {}  # agent_id → agent_name
signal agent_selected(agent_id: String)
```

#### 交互逻辑

1. 用户点击 Tab → ChannelTabs.channel_changed 信号 → StateManager._narrative_channel 更新 → NarrativeList 刷新
2. 用户点击 Agent Sprite → AgentFilter.agent_selected 信号 → StateManager._narrative_agent_filter 更新 → NarrativeList 刷新
3. StateManager 收到 narrative 信号 → 追加 _narratives → 发射 state_updated → NarrativeList 检查过滤条件后更新

#### 前端接口对接

| 信号 | 来源 | 处理 |
| --- | --- | --- |
| bridge.world_updated | SimulationBridge | StateManager._on_world_updated（初始化+兜底） |
| bridge.agent_delta | SimulationBridge | StateManager._on_delta（统一处理） |
| bridge.narrative_event | SimulationBridge | StateManager._on_narrative（追加叙事） |

## 5. 技术决策

### 决策1：AgentState vs 保持 AgentSnapshot/AgentDelta 分离

- **选型方案**：统一为 AgentState
- **选择理由**：
  - 消除字段重复，新增字段只改一处
  - 数据构建逻辑集中化，调用统一方法
  - ShadowAgent 直接使用 AgentState，减少转换
- **备选方案**：保持分离，仅添加转换方法
- **放弃原因**：仍需维护两套结构，新增字段仍需多处修改

### 决策2：Delta 简化 vs 保持14种变体

- **选型方案**：简化为 AgentStateChanged + WorldEvent 两类
- **选择理由**：
  - AgentStateChanged 统一表示所有 Agent 状态变化
  - WorldEvent 清晰区分世界级事件
  - change_hint 标记变化原因，客户端无需推断
- **备选方案**：保持14种变体，仅合并重复字段
- **放弃原因**：语义仍混乱，AgentDied/AgentSpawned 等仍是 AgentStateChanged 子集

### 决策3：叙事作为 WorldEvent vs 独立通道

- **选型方案**：叙事作为 WorldEvent(AgentNarrative)
- **选择理由**：
  - 叙事本质是"Agent做了什么"，属于世界事件
  - 状态变化和叙事一起发送，减少通道数量
  - 客户端处理更统一
- **备选方案**：叙事独立通道 + NarrativeEnvelope
- **放弃原因**：增加复杂度，两个广播通道

### 决策4：Agent 过滤实现方式

- **选型方案**：点击 Agent Sprite + 下拉选择
- **选择理由**：
  - 点击 Agent 最直观，与渲染联动
  - 下拉选择支持远程 Agent 追踪
  - 两种方式互补
- **备选方案**：仅下拉选择
- **放弃原因**：点击更自然，减少用户操作步骤

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| 客户端兼容性 | 高 | conversion.rs 保持向后兼容，旧字段用默认值 |
| P2P 区域订阅生效 | 中 | 先实现消息处理，后优化区域过滤 |
| 叙事广播带宽 | 中 | Nearby 频道按区域过滤，减少广播范围 |
| Snapshot 缺失字段迁移 | 高 | conversion.rs 补全字段，客户端测试验证 |
| ShadowAgent 过期清理 | 低 | 保留现有 cleanup_expired_shadows() 逻辑 |

## 7. 迁移方案

### 7.1 部署步骤

1. **Phase 1：数据模型重构（Rust）**
   - 创建 AgentState 结构
   - 实现 Agent::to_state()、AgentState::to_delta()
   - 修改 DeltaEmitter 使用统一方法
   - 修改 World.snapshot() 使用统一方法
   - 测试：单元测试覆盖转换逻辑

2. **Phase 2：Delta 简化（Rust）**
   - 定义 Delta::AgentStateChanged + WorldEvent
   - 实现 ChangeHint 枚举
   - 修改 delta.rs、conversion.rs
   - 测试：Delta 序列化/反序列化测试

3. **Phase 3：叙事频道系统（Rust + P2P）**
   - 扩展 NarrativeEvent 结构
   - 实现 determine_narrative_channel()
   - 启用 RegionTopicManager 消息处理
   - 创建 world_events topic
   - 测试：P2P 叙事广播测试

4. **Phase 4：客户端重构（Godot）**
   - 修改 StateManager._on_delta() 统一处理
   - 实现 NarrativeFeed 频道切换
   - 实现 AgentFilter 选择器
   - 测试：多 Agent 本地测试

5. **Phase 5：清理与文档**
   - 移除旧的 AgentDelta 变体
   - 移除 Snapshot 的 events/legacies 字段
   - 更新 CLAUDE.md 文档

### 7.2 灰度策略

- 先在单 Agent 本地模式测试
- 后在多 Agent 本地模式测试 Agent 过滤
- 最后在 P2P 双节点测试叙事广播

### 7.3 回滚方案

- AgentState 结构兼容 AgentSnapshot 字段，旧客户端可接收
- Delta 新增 type 字段，旧客户端通过默认处理跳过
- Snapshot 遗漏字段补全后，新旧客户端均可用

## 8. 待定事项

- [ ] 叙事面板是否需要搜索功能？
- [ ] 叙事是否需要时间范围过滤（只看最近N条）？
- [ ] 是否需要"屏蔽"某个 Agent 的叙事？
- [ ] 远程 Agent 的 reasoning 是否显示？（当前设计为不显示）