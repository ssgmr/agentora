# 架构重构设计

## 目标架构概览

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          目标架构                                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │                    Godot Client（纯渲染层）                       │  │
│   │                                                                  │  │
│   │   world_renderer.gd                                              │  │
│   │   - 从 snapshot 接收 terrain_width/height                        │  │
│   │   - 不硬编码任何配置                                              │  │
│   │                                                                  │  │
│   │   agent_manager.gd                                               │  │
│   │   - 接收 delta 事件渲染 Agent                                    │  │
│   │                                                                  │  │
│   │   agent_detail_panel.gd                                          │  │
│   │   - 删除重复的引导按钮                                            │  │
│   │                                                                  │  │
│   │   BridgeAccessor（Autoload）                                     │  │
│   │   - 统一 SimulationBridge 获取                                  │  │
│   │                                                                  │  │
│   │   SimulationBridge节点                                           │  │
│   │   - 只发射信号，不包含业务逻辑                                    │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                    │                                   │
│                                    │ GDExtension                       │
│                                    ▼                                   │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │                    Bridge Crate（约150行）                        │  │
│   │                                                                  │  │
│   │   src/mod.rs                                                     │  │
│   │   - SimulationBridge 节点定义                                   │  │
│   │   - start() → core::Simulation::start()                         │  │
│   │   - pause() → core::Simulation::pause()                         │  │
│   │                                                                  │  │
│   │   src/conversion.rs                                              │  │
│   │   - snapshot_to_dict()                                          │  │
│   │   - delta_to_dict()                                             │  │
│   │                                                                  │  │
│   │   src/signals.rs                                                 │  │
│   │   - emit_snapshot()                                             │  │
│   │   - emit_delta()                                                │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                    │                                   │
│                                    │ Simulation API                   │
│                                    ▼                                   │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │                    Core Crate                                    │  │
│   │                                                                  │  │
│   │   ┌─────────────────────────────────────────────────────────┐   │  │
│   │   │ simulation/  [新建]                                      │   │  │
│   │   │                                                           │   │  │
│   │   │   mod.rs        - Simulation 结构体 + 公开 API          │   │  │
│   │   │   config.rs     - SimConfig（从 bridge 移入）            │   │  │
│   │   │   delta.rs      - WorldDelta/AgentDelta（从 bridge 移入）│   │  │
│   │   │   agent_loop.rs - Agent 决策循环                        │   │  │
│   │   │   tick_loop.rs  - World tick 推进                       │   │  │
│   │   │   snapshot_loop.rs - 快照生成                           │   │  │
│   │   │   npc.rs        - NPC 创建和管理                        │   │  │
│   │   │                                                           │   │  │
│   │   │   Simulation API:                                        │   │  │
│   │   │   pub fn new(config, seed) -> Simulation                 │   │  │
│   │   │   pub fn start(&self)                                    │   │  │
│   │   │   pub fn pause(&self) / resume(&self)                    │   │  │
│   │   │   pub fn inject_preference(&self, agent_id, pref)        │   │  │
│   │   │   pub fn subscribe_snapshot(&self) -> Receiver           │   │  │
│   │   │   pub fn subscribe_delta(&self) -> Receiver              │   │  │
│   │   └─────────────────────────────────────────────────────────┘   │  │
│   │                                                                  │  │
│   │   ┌─────────────────────────────────────────────────────────┐   │  │
│   │   │ world/                                                   │   │  │
│   │   │                                                           │   │  │
│   │   │   mod.rs        - World 结构体 + 基本查询（约200行）      │   │  │
│   │   │   generator.rs  - 地形/Agent/资源生成                    │   │  │
│   │   │   tick.rs       - advance_tick, survival_consumption     │   │  │
│   │   │   actions.rs    - 动作路由（调用 Agent 方法）             │   │  │
│   │   │   feedback.rs   - 反馈生成                               │   │  │
│   │   │   snapshot.rs   - 快照生成                               │   │  │
│   │   │   milestones.rs - 里程碑系统                             │   │  │
│   │   │   pressure.rs   - 压力系统                               │   │  │
│   │   │   legacy.rs     - 遗产管理（从顶层移入）                  │   │  │
│   │   │   vision.rs     - 视野扫描（从顶层移入）                  │   │  │
│   │   │   map.rs        - CellGrid                               │   │  │
│   │   │   region.rs     - Region                                 │   │  │
│   │   │   resource.rs   - ResourceNode                           │   │  │
│   │   │   structure.rs  - Structure                              │   │  │
│   │   └─────────────────────────────────────────────────────────┘   │  │
│   │                                                                  │  │
│   │   ┌─────────────────────────────────────────────────────────┐   │  │
│   │   │ agent/                                                   │   │  │
│   │   │                                                           │   │  │
│   │   │   mod.rs        - Agent 结构体 + 基本方法                │   │  │
│   │   │   inventory.rs  - gather/consume                         │   │  │
│   │   │   combat.rs     - attack() [被 World 调用]               │   │  │
│   │   │   trade.rs      - propose_trade/accept_trade             │   │  │
│   │   │   alliance.rs   - accept_alliance/reject_alliance        │   │  │
│   │   │   relation.rs   - 关系管理                               │   │  │
│   │   │                                                           │   │  │
│   │   │   设计原则:                                               │   │  │
│   │   │   - Agent 方法封装所有状态变更                           │   │  │
│   │   │   - World handler 验证条件后调用 Agent 方法              │   │  │
│   │   │   - World handler 负责叙事和统计                         │   │  │
│   │   └─────────────────────────────────────────────────────────┘   │  │
│   │                                                                  │  │
│   │   ┌─────────────────────────────────────────────────────────┐   │  │
│   │   │ decision/  [拆分 decision.rs]                            │   │  │
│   │   │                                                           │   │  │
│   │   │   mod.rs        - DecisionPipeline                       │   │  │
│   │   │   perception.rs - build_perception_summary               │   │  │
│   │   │   llm_call.rs  - call_llm, json 解析                     │   │  │
│   │   │   candidate.rs - 候选动作生成                            │   │  │
│   │   └─────────────────────────────────────────────────────────┘   │  │
│   │                                                                  │  │
│   │   memory/   （不变）                                            │  │
│   │   strategy/ （不变）                                            │  │
│   │   storage/  （不变）                                            │  │
│   │                                                                  │  │
│   │   types.rs     - 核心类型                                       │  │
│   │   seed.rs      - WorldSeed                                      │  │
│   │   snapshot.rs  - WorldSnapshot                                  │  │
│   │   narrative.rs - 叙事事件                                       │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## 组件详情

### 1. simulation/ 模块（新建）

**用途**：整个模拟的编排层

**文件结构**：

```rust
// simulation/mod.rs
pub struct Simulation {
    world: Arc<Mutex<World>>,
    config: SimConfig,
    pipeline: DecisionPipeline,
    running: AtomicBool,
    snapshot_tx: broadcast::Sender<WorldSnapshot>,
    delta_tx: broadcast::Sender<WorldDelta>,
}

impl Simulation {
    pub fn new(config: SimConfig, seed: WorldSeed) -> Self;
    pub fn start(&self);
    pub fn pause(&self);
    pub fn resume(&self);
    pub fn inject_preference(&self, agent_id: AgentId, pref: Preference);
    pub fn subscribe_snapshot(&self) -> Receiver<WorldSnapshot>;
    pub fn subscribe_delta(&self) -> Receiver<WorldDelta>;
}

// simulation/config.rs
pub struct SimConfig {
    pub initial_agent_count: u32,
    pub npc_count: u32,
    pub player_decision_interval_secs: f32,
    pub npc_decision_interval_secs: f32,
    pub snapshot_interval_secs: f32,
    pub tick_interval_secs: f32,
}

// simulation/delta.rs
pub enum WorldDelta {
    AgentMoved { id: String, x: u32, y: u32 },
    AgentDied { id: String },
    AgentSpawned { id: String, x: u32, y: u32 },
    StructureCreated { x: u32, y: u32, structure_type: String },
    // ... 所有 delta 类型
}

// simulation/agent_loop.rs
pub fn run_agent_loop(
    simulation: &Simulation,
    agent_id: AgentId,
    is_npc: bool,
) -> JoinHandle<()> {
    // 单个 Agent 的决策循环
    // 调用 pipeline.decide()
    // 调用 world.apply_action()
    // 发送 delta
}

// simulation/tick_loop.rs
pub fn run_tick_loop(simulation: &Simulation) -> JoinHandle<()> {
    // 世界时间推进
    // survival_consumption_tick
    // pressure_tick
}

// simulation/snapshot_loop.rs
pub fn run_snapshot_loop(simulation: &Simulation) -> JoinHandle<()> {
    // 定期完整快照生成
}

// simulation/npc.rs
pub fn create_npc_agents(world: &mut World, count: u32, seed: &WorldSeed);
```

### 2. world/ 模块重构

**当前 mod.rs 职责 → 新位置**：

| 当前职责 | 行数 | 新文件 |
|---------|------|--------|
| World 结构体 + 查询 | 约200 | mod.rs（保留） |
| generate_terrain | 约100 | generator.rs |
| generate_agents | 约40 | generator.rs |
| generate_resources | 约120 | generator.rs |
| advance_tick | 约40 | tick.rs |
| survival_consumption_tick | 约20 | tick.rs |
| structure_effects_tick | 约25 | tick.rs |
| pressure_tick | 约120 | pressure.rs |
| check_milestones | 约100 | milestones.rs |
| apply_milestone_feedback | 约125 | milestones.rs |
| generate_action_feedback | 约280 | feedback.rs |
| snapshot | 约125 | snapshot.rs |
| Agent 死亡处理 | 约75 | legacy.rs |
| decay_legacies | 约15 | legacy.rs |

### 3. Agent 方法统一

**当前模式（封装破坏）**：
```rust
// world/actions.rs - handle_attack
let target = self.agents.get_mut(&target_id).unwrap();
target.health = target.health.saturating_sub(damage);  // 直接操作!
if let Some(rel) = target.relations.get_mut(&agent_id) {
    rel.relation_type = RelationType::Enemy;  // 直接操作!
}
```

**目标模式（正确封装）**：
```rust
// world/actions.rs - handle_attack
// World 验证条件
if !self.validate_attack_conditions(agent_id, &target_id) {
    return ActionResult::Blocked("...");
}

// World 调用 Agent 方法
let (attacker, target) = self.get_two_agents_mut(agent_id, &target_id);
let result = attacker.attack(target, damage);

// World 处理叙事和统计
self.record_attack_event(agent_id, &result);
self.total_attacks += 1;
```

**需要修改的 World handlers**：

| Handler | 当前状态 | 目标状态 |
|---------|---------|---------|
| handle_attack | 直接操作 health/relations | 调用 `agent.attack(target, damage)` |
| handle_trade_accept | 直接操作 inventory | 调用 `agent.accept_trade()` |
| handle_trade_offer | 直接检查 inventory | 调用 `agent.can_propose_trade()` |
| handle_ally_accept | 已调用 agent 方法 ✓ | 已正确 |
| handle_ally_reject | 已调用 agent 方法 ✓ | 已正确 |

### 4. Bridge 瘦身

**当前 bridge/lib.rs（约1481行）→ 目标（约150行）**：

| 当前内容 | 行数 | 新位置 |
|---------|------|--------|
| LogConfig + init_logging | 约70 | 删除或极简 |
| AgentDelta 定义 | 约100 | simulation/delta.rs |
| SimConfig | 约95 | simulation/config.rs |
| NarrativeEvent 渲染 | 约9 | 保留（极简） |
| SimulationBridge 节点 | 约200 | 保留 |
| delta_to_dict | 约130 | 保留 |
| agent_to_dict | 约25 | 保留 |
| run_simulation | 约12 | 删除（调用 Simulation::new） |
| run_simulation_async | 约156 | simulation/mod.rs |
| create_npc_agents | 约60 | simulation/npc.rs |
| run_agent_loop | 约308 | simulation/agent_loop.rs |
| run_tick_loop | 约36 | simulation/tick_loop.rs |
| run_snapshot_loop | 约34 | simulation/snapshot_loop.rs |
| 记忆记录 | 约30 | world/feedback.rs 或 simulation/ |

### 5. Godot 客户端清理

**删除文件**：
- `scripts/guide_panel.gd` - 与 agent_detail_panel 功能重复
- `assets/scenes/agent_sprite.tscn` - 未使用的场景文件

**移除节点**（从 main.tscn）：
- `WorldView/Structures` - 空节点
- `WorldView/Legacies` - 空节点
- `UI/RightPanel/WorldInfo` - 不可见，从未激活

**BridgeAccessor Autoload**：
```gdscript
# scripts/bridge_accessor.gd
extends Node

static var _bridge: Node = null

static func get_bridge() -> Node:
    if _bridge == null:
        _bridge = Engine.get_main_loop().root.get_node_or_null("Main/SimulationBridge")
    return _bridge

static func reset():
    _bridge = null
```

**world_renderer.gd 修改**：
```gdscript
# 移除硬编码 _map_size = 256
var _map_size: int = -1  # 未知，等 snapshot 到来

func _on_world_updated(snapshot: Dictionary) -> void:
    # 使用 snapshot 数据
    if snapshot.has("terrain_width"):
        _map_size = snapshot.terrain_width
    # ...
```

**camera_controller.gd 修改**：
```gdscript
# 移除硬编码边界
var _map_bounds: Rect2 = Rect2(0, 0, 99999, 99999)  # 默认无边界

func set_map_bounds(width: int, height: int, tile_size: int):
    _map_bounds = Rect2(0, 0, width * tile_size, height * tile_size)
```

## 重构后数据流

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          正确的数据流                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   配置文件                                                               │
│   ┌─────────────────┐    ┌─────────────────┐                           │
│   │ worldseeds/     │    │ config/         │                           │
│   │ default.toml    │    │ sim.toml        │                           │
│   │ (map_size=256)  │    │ (intervals)     │                           │
│   └─────────────────┘    └─────────────────┘                           │
│            │                       │                                   │
│            ▼                       ▼                                   │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │   core::simulation::Simulation::new(seed, config)               │  │
│   │                                                                  │  │
│   │   ┌─────────────────────────────────────────────────────────┐   │  │
│   │   │ WorldSnapshot {                                          │   │  │
│   │   │   terrain_width: 256,   ← 来自 seed                      │   │  │
│   │   │   terrain_height: 256,  ← 来自 seed                      │   │  │
│   │   │   terrain_grid: [...],                                   │   │  │
│   │   │   agents: [...],                                         │   │  │
│   │   │ }                                                        │   │  │
│   │   └─────────────────────────────────────────────────────────┘   │  │
│   │                                                                  │  │
│   │   broadcast::Sender<WorldSnapshot>                              │  │
│   │   broadcast::Sender<WorldDelta>                                 │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                    │                                   │
│                                    │ channel                           │
│                                    ▼                                   │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │   bridge::SimulationBridge                                       │  │
│   │                                                                  │  │
│   │   snapshot_to_dict(snapshot) → Dictionary                       │  │
│   │   emit_signal("world_updated", dict)                            │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                    │                                   │
│                                    │ Godot signal                      │
│                                    ▼                                   │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │   Godot Scripts                                                  │  │
│   │                                                                  │  │
│   │   world_renderer._on_world_updated(snapshot):                   │  │
│   │       _map_size = snapshot.terrain_width  ← 使用后端数据        │  │
│   │                                                                  │  │
│   │   camera.set_map_bounds(_map_size, _map_size, 16)               │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│   原则：所有配置来自 Core，客户端只负责渲染                              │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## 错误处理策略

- 每阶段必须通过 `cargo test` 才能继续
- Bridge 改动后客户端必须正确渲染
- 如有问题用 `git stash` 保存工作
- 每阶段完成后提交

## 测试策略

1. **Phase 1 后**：运行 `cargo test`，验证模拟启动
2. **Phase 2 后**：运行 `cargo test`，验证 world 操作
3. **Phase 3 后**：运行 `cargo test`，验证 agent 交互
4. **Phase 4 后**：运行 Godot 客户端，验证渲染
5. **Phase 5 后**：完整集成测试