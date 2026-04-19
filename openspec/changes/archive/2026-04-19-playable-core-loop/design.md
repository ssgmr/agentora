# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

Agentora采用Rust核心引擎 + Godot 4 GDExtension架构。核心引擎负责模拟逻辑（动机、决策、世界状态），通过Bridge crate的mpsc双通道将增量事件和完整快照推送到Godot客户端渲染。当前13种Action全部有handler实现（Tier 2完成），但缺少消耗循环、建筑功能、环境压力和玩家反馈，导致模拟运行但不足以构成"游戏"。

### 1.2 现状分析

- Agent有health(HP)但无生存消耗，HP只在战斗中下降
- Wait动作直接回血(+5 HP)，无消耗需求
- 三种建筑(Camp/Fence/Warehouse)只是地图贴图，无功能效果
- pressure_tick()是空壳(TODO)，世界永远平静
- 引导面板有6个抽象滑块，玩家不知道调参的实际效果
- 没有阶段性目标或成就系统
- Agent snapshot中不含satiety/hydration数据

### 1.3 关键干系人

- **Rust Core Engine** — 生存消耗、建筑效果、压力事件、里程碑的核心逻辑
- **Bridge Layer** — 新增Delta变体推送
- **Godot Client** — UI重设计、状态展示、里程碑提示
- **LLM Provider** — Prompt注入生存压力和事件信息

## 2. 设计目标

### 目标

- 建立Agent"需要吃饭喝水才能活"的生存消耗循环，形成经济驱动
- 让建筑产生可感知的战术/战略效果
- 让世界定期"出事"，创造戏剧性和决策压力
- 让玩家的引导操作产生可见的因果反馈
- 提供文明发展进度，给玩家成就感

### 非目标

- 复杂烹饪/配方系统（当前为一对一消耗）
- 建筑升级/科技树（未来增强）
- 区域封锁/自然灾害等复杂压力事件（当前3种）
- 玩家直接控制Agent动作（保持引导式设计）
- P2P多人联网（后加）
- 策略跨Agent传播/文化传承（Tier 3+）

## 3. 整体架构

### 3.1 架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                    advance_tick 新执行流程                       │
│                                                                 │
│  1. 生存消耗     satiety -= 2, hydration -= 2.5                │
│     └── 饥渴掉HP  satiety=0 → HP-2/tick, hydration=0 → HP-3  │
│                                                                 │
│  2. 建筑效果     遍历structures                                │
│     ├── Camp    → 附近Agent HP+2                               │
│     ├── Fence   → (被动，Move时检查)                            │
│     └── Warehouse→ (被动，Gather时检查上限)                     │
│                                                                 │
│  3. 动机衰减     motivation.decay() [原有]                      │
│  4. 临时偏好     tick_preferences() [原有]                      │
│                                                                 │
│  5. 压力Ticks    生成/推进压力事件                               │
│     ├── 生成: 随机40-80tick间隔                                 │
│     ├── 影响: 干旱减水/丰饶增食/瘟疫扣HP                       │
│     └── 叙事: 事件开始/结束推送NarrativeEvent                   │
│                                                                 │
│  6. 死亡检查     check_agent_death() [原有,含饥渴致死]          │
│  7. 遗产衰减     decay_legacies() [原有]                        │
│  8. 策略衰减     每50tick [原有]                                │
│  9. 里程碑检查   检测新增里程碑 [新增]                           │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件 | 职责说明 |
| --- | --- |
| `Agent.satiety/hydration` | 新增生存指标字段，每tick衰减 |
| `World.advance_tick()` | 扩展：新增生存消耗+建筑效果+压力生成+里程碑检查 |
| `World.pressure_tick()` | 从TODO升级为实际生成压力事件 |
| `World.milestones` | 新增里程碑列表字段 |
| `World.next_pressure_tick` | 新增下次压力事件触发tick |
| `structure_effects()` | 新增：遍历建筑应用效果 |
| `handle_wait()` | 修改：从回血改为饮食恢复 |
| `handle_move()` | 修改：新增Fence碰撞检查 |
| `DecisionPipeline.build_prompt()` | 修改：注入生存状态+压力事件 |
| `RuleEngine.fallback_decision()` | 修改：NPC优先满足饮食需求 |
| `AgentSnapshot` | 扩展：新增satiety/hydration字段 |
| `AgentDelta` | 扩展：新增MilestoneReached/PressureEvent变体 |
| `guide_panel.gd` | 重设计：预设按钮+高级滑块 |
| `agent_manager.gd` | 增强：显示生存状态条 |
| `milestone_ui` | 新增：里程碑进度和达成提示 |

### 3.3 数据流设计

```
┌──────────────┐    advance_tick     ┌──────────────────┐
│   World      │────────────────────▶│  生存消耗+建筑    │
│  (Rust)      │                     │  效果+压力+里程碑 │
└──────┬───────┘                     └────────┬─────────┘
       │                                      │
       │  AgentDelta推送                      │ Agent状态变更
       ▼                                      ▼
┌──────────────┐    序列化          ┌──────────────────┐
│   Bridge     │──────────────────▶│  WorldSnapshot    │
│  mpsc通道    │                    │  + DeltaEvents    │
└──────┬───────┘                    └────────┬─────────┘
       │                                      │
       ▼                                      ▼
┌──────────────┐    渲染           ┌──────────────────┐
│  Godot       │◀─────────────────│  UI组件更新       │
│  Client      │                   │  状态条/里程碑    │
└──────────────┘                   └──────────────────┘
```

## 4. 详细设计

### 4.1 数据模型

#### Agent字段扩展

```rust
// crates/core/src/agent/mod.rs
pub struct Agent {
    // ... 现有字段 ...
    pub satiety: u32,       // 饱食度 0-100，初始100
    pub hydration: u32,     // 水分度 0-100，初始100
}
```

#### Milestone结构

```rust
// crates/core/src/world/mod.rs 或单独 milestone.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub name: String,          // 里程碑标识
    pub display_name: String,  // 显示名称
    pub achieved_tick: u64,    // 达成tick，0=未达成
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MilestoneType {
    FirstCamp,         // 第一座营地
    FirstTrade,        // 贸易萌芽
    FirstFence,        // 领地意识
    FirstAttack,       // 冲突爆发
    FirstLegacyInteract, // 首次传承
    CityState,         // 城邦雏形
    GoldenAge,         // 文明黄金期
}
```

#### World字段扩展

```rust
pub struct World {
    // ... 现有字段 ...
    pub milestones: Vec<Milestone>,         // 文明里程碑
    pub next_pressure_tick: u64,            // 下次压力事件触发tick
    pub pressure_multiplier: HashMap<String, f32>, // 资源产出乘数("Water"→0.5)
}
```

#### AgentSnapshot扩展

```rust
// crates/core/src/snapshot.rs
pub struct AgentSnapshot {
    // ... 现有字段 ...
    pub satiety: u32,
    pub hydration: u32,
}
```

#### Bridge Delta扩展

```rust
// crates/bridge/src/lib.rs 或 snapshot.rs
pub enum AgentDelta {
    // ... 现有变体 ...
    SurvivalStatus { agent_id: String, satiety: u32, hydration: u32, hp: u32 },
}

pub enum WorldDelta {
    // ... 现有变体 ...
    MilestoneReached { name: String, display_name: String, tick: u64 },
    PressureStarted { pressure_type: String, description: String, duration: u32 },
    PressureEnded { pressure_type: String, description: String },
    HealedByCamp { agent_id: String, hp_restored: u32 },
}
```

### 4.2 核心算法

#### 生存消耗算法

```
// advance_tick 内新增：survival_consumption_tick()

for agent in agents.alive():
    agent.satiety = max(0, agent.satiety - 2)
    agent.hydration = max(0, agent.hydration - 2.5.round())  // 四舍五入取整

    if agent.satiety == 0:
        agent.health = max(0, agent.health - 2)
    if agent.hydration == 0:
        agent.health = max(0, agent.health - 3)
```

衰减速率：satiety -= 2/tick, hydration -= 2/tick（取整）
- satiety 100 → 0 约需50 tick
- hydration 100 → 0 约需40 tick
- 按默认决策间隔2秒，约80-100秒（1.5分钟）无摄入开始饿/渴
- 从开始掉血到死亡(100HP)：饱和饥饿+口渴 5HP/tick → 20 tick → 约40秒致死
- 总体：从完全无摄入到死亡约2-2.5分钟

#### 建筑效果算法

```
// advance_tick 内新增：structure_effects_tick()

for structure in world.structures:
    if structure.structure_type == StructureType::Camp:
        for agent in agents.alive() where manhattan_distance(agent.pos, structure.pos) <= 1:
            if agent.health < agent.max_health:
                agent.health = min(agent.max_health, agent.health + 2)
                push_delta(HealedByCamp { agent_id, hp_restored: 2 })
```

Fence和Warehouse是被动效果（Move和Gather时检查），不在tick循环中处理。

#### Fence碰撞检查

```
// handle_move() 内新增检查

fn handle_move(agent, direction, world):
    target_pos = agent.position + direction.delta()
    // 原有边界检查...

    // 新增：Fence碰撞检查
    for structure in world.structures_at(target_pos):
        if structure.structure_type == StructureType::Fence:
            let fence_owner = structure.owner_id
            if let Some(relation) = agent.relations.get(fence_owner):
                if relation.relation_type == RelationType::Enemy:
                    return Blocked("被围栏阻挡，无法通过敌对领地")

    // 原有地形检查...
```

#### Warehouse库存上限检查

```
// handle_gather() 和 Agent.gather() 修改

fn effective_inventory_limit(agent, world) -> usize:
    base = 20
    for structure in world.structures:
        if structure.structure_type == StructureType::Warehouse:
            if manhattan_distance(agent.pos, structure.pos) <= 1:
                return base + 20
    return base
```

#### 压力事件生成算法

```
// pressure_tick() 升级版

fn pressure_tick(world):
    // 生成新事件
    if world.tick >= world.next_pressure_tick:
        if world.pressure_pool.len() < 3:  // 最多3个同时活跃
            event_type = random_choice([Drought, Abundance, Plague])
            event = PressureEvent::generate(event_type, world.tick)
            apply_pressure_effect(event, world)  // 应用立即效果
            world.pressure_pool.push(event)
            push_narrative(PressureStarted)
            world.next_pressure_tick = world.tick + random_range(40, 80)
        else:
            world.next_pressure_tick = world.tick + 20  // 推迟

    // 推进现有事件
    for event in world.pressure_pool:
        event.advance()
    // 移除过期事件
    expired = world.pressure_pool.drain_filter(|e| e.is_finished())
    for event in expired:
        revert_pressure_effect(event, world)  // 撤销持续效果（干旱恢复产出等）
        push_narrative(PressureEnded)
```

#### 压力效果应用

```
fn apply_pressure_effect(event, world):
    match event.pressure_type:
        Drought:
            world.pressure_multiplier.insert("Water", 0.5)
            // Gather时: actual_gathered = base * multiplier
        Abundance:
            for node in world.resource_nodes where resource_type == Food:
                node.current_amount = min(node.max_amount, node.current_amount * 2)
        Plague:
            targets = random_sample(world.alive_agents(), random_range(1, 3))
            for agent in targets:
                agent.health = max(0, agent.health - 20)

fn revert_pressure_effect(event, world):
    match event.pressure_type:
        Drought:
            world.pressure_multiplier.remove("Water")
        // Abundance和Plague是单次效果，无需回退
```

#### 里程碑检测算法

```
fn check_milestones(world):
    milestones_to_check = [
        (FirstCamp, world.structures.any(|s| s.structure_type == Camp)),
        (FirstTrade, world.trade_history_count > 0),  // 需记录交易次数
        (FirstFence, world.structures.any(|s| s.structure_type == Fence)),
        (FirstAttack, world.attack_history_count > 0),  // 需记录攻击次数
        (FirstLegacyInteract, world.legacy_interact_count > 0),
        (CityState, world.structures.alive_count() >= 3
                    && world.ally_pair_count >= 2
                    && world.structures.any(|s| s.structure_type == Warehouse)),
        (GoldenAge, 前六个全部达成),
    ]
    for (milestone_type, condition) in milestones_to_check:
        if condition && !world.milestones.achieved(milestone_type):
            world.milestones.achieve(milestone_type, world.tick)
            push_delta(MilestoneReached)
            push_narrative(MilestoneAchieved)
```

为记录里程碑条件，World需新增计数器：
- `total_trades: u32` — 每次TradeAccept成功时+1
- `total_attacks: u32` — 每次Attack成功时+1
- `total_legacy_interacts: u32` — 每次InteractLegacy成功时+1

### 4.3 Prompt注入设计

#### 生存状态注入

在`prompt.rs`或`decision.rs`的感知段构建中新增：

```
当前状态：
  饱食度: {satiety}/100{if satiety <= 30 then " [饥饿！]" else ""}
  水分度: {hydration}/100{if hydration <= 30 then " [口渴！]" else ""}
  生命值: {hp}/{max_hp}
```

#### 压力事件注入

```
当前世界事件：
  - 干旱来袭，水源产出减半（剩余15 tick）
  - ...
```

### 4.4 NPC决策增强

在`rule_engine.rs`的`fallback_decision()`开头新增生存优先逻辑：

```
fn fallback_decision(agent, world_state):
    // 生存优先：饥饿/口渴时优先满足
    if agent.satiety <= 30 || agent.hydration <= 30:
        if agent.satiety <= 30 && agent.inventory.get("Food") > 0:
            return Wait  // 吃饭
        if agent.hydration <= 30 && agent.inventory.get("Water") > 0:
            return Wait  // 喝水
        // 背包没有就去找
        if agent.satiety <= 30:
            nearest = find_nearest_resource(world_state, agent.position, "Food")
            if nearest != null:
                return move_toward(nearest)
            return Explore  // 随机探索找食物
        if agent.hydration <= 30:
            nearest = find_nearest_resource(world_state, agent.position, "Water")
            if nearest != null:
                return move_toward(nearest)
            return Explore

    // 原有动机驱动逻辑...
```

### 4.5 Godot客户端设计

#### 前端架构

- 框架：Godot 4 GDScript
- 数据源：SimulationBridge Rust GDExtension
- 通信：每帧poll Bridge的delta/snapshot通道

#### 引导面板重设计 (guide_panel.gd)

```
┌─────────────────────────────────┐
│  引导面板                        │
│                                 │
│  ┌─────┐ ┌─────┐ ┌─────┐      │
│  │ 生  │ │ 社  │ │ 探  │      │
│  │ 存  │ │ 交  │ │ 索  │      │
│  └─────┘ └─────┘ └─────┘      │
│  ┌─────┐ ┌─────┐ ┌─────┐      │
│  │ 创  │ │ 征  │ │ 传  │      │
│  │ 造  │ │ 服  │ │ 承  │      │
│  └─────┘ └─────┘ └─────┘      │
│                                 │
│  ▶ 高级自定义                   │
│  ┌─────────────────────────┐   │
│  │ 生存 ─────●────── 0%    │   │
│  │ 社交 ───────●──── 0%    │   │
│  │ 认知 ─────●────── 0%    │   │
│  │ 表达 ───────●──── 0%    │   │
│  │ 权力 ─────●────── 0%    │   │
│  │ 传承 ───────●──── 0%    │   │
│  └─────────────────────────┘   │
└─────────────────────────────────┘
```

#### Agent详情面板 (agent_manager.gd 或新面板)

```
┌─────────────────────────────────┐
│  Alice                          │
│                                 │
│  HP     ██████████░░ 80/100     │
│  饱食度  ██████░░░░░░ 60/100    │
│  水分度  ████░░░░░░░░ 40/100    │
│                                 │
│  背包:                          │
│    Food: 5  Water: 2  Wood: 3   │
│    Iron: 1  Stone: 4            │
└─────────────────────────────────┘
```

#### 里程碑进度UI (新节点，挂在CanvasLayer下)

```
┌──────────────────────────────────────────┐
│  🏆 3/7                                   │
│  [营地✓] [贸易✓] [领地✓] [冲突✗] [传承✗] │
│  [城邦✗] [黄金✗]                          │
└──────────────────────────────────────────┘

达成时弹出提示(居中，2秒消失):
┌─────────────────────────────┐
│  🏆 第一座营地 已达成！      │
└─────────────────────────────┘
```

### 4.6 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| Agent satiety/hydration溢出(>100) | 截断至100 |
| Agent HP因饥渴降至0 | 正常进入死亡流程(check_agent_death) |
| 瘟疫目标不足存活Agent | 瘟疫事件无效，不生成或跳过 |
| 干旱期间没有Water节点 | 事件正常生成，但无实际影响（乘数无节点可乘） |
| Warehouse范围内库存超出上限后离开 | 超出资源保留，但不可再采集新资源 |
| Camp回血与Wait回血冲突 | Wait已改为不回血，无冲突 |
| 里程碑计数器跨重置 | World创建时初始化为0，随World生命周期增长 |

## 5. 技术决策

### 决策1：生存指标用u32而非f32

- **选型方案**：satiety: u32, hydration: u32 (0-100整数)
- **选择理由**：与现有health(u32)保持一致；整数运算精确无浮点误差；Godot端显示简单
- **备选方案**：f32 (0.0-100.0)
- **放弃原因**：浮点衰减累积误差；与现有HP系统不一致

### 决策2：压力事件影响用全局乘数而非修改ResourceNode属性

- **选型方案**：World.pressure_multiplier: HashMap<String, f32>，Gather时读取
- **选择理由**：解耦压力系统和资源节点；干旱结束只需删乘数，无需逐节点恢复；扩展性好
- **备选方案**：直接修改ResourceNode.regeneration_rate
- **放弃原因**：需要记录原始值以便恢复；节点数量多时修改成本高

### 决策3：里程碑用简单计数器而非事件溯源

- **选型方案**：World新增total_trades/total_attacks/total_legacy_interacts计数器
- **选择理由**：实现简单，O(1)检查；与现有World结构一致
- **备选方案**：从NarrativeEvent历史中回溯
- **放弃原因**：历史事件量大时O(n)扫描；事件格式不一定稳定

### 决策4：引导面板预设按钮优先于滑块

- **选型方案**：6个预设按钮为主，自定义滑块折叠为高级选项
- **选择理由**：降低玩家理解成本；按钮文字(生存/社交)比滑块数值直观；保持"引导式"定位
- **备选方案**：保留当前6滑块设计
- **放弃原因**：抽象数值(0.3 vs 0.8)对玩家不友好

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| 生存消耗太快导致Agent过早死亡 | 中 | 初始100 + 2/tick衰减 ≈ 50 tick缓冲；NPC优先满足饮食；Wait可快速恢复 |
| 压力事件叠加(干旱+瘟疫)过强 | 低 | 最多3个同时活跃；间隔40-80tick；干旱只影响产出不影响HP |
| Fence碰撞检查性能 | 低 | 每次Move只检查目标格的structures(已空间索引) |
| Warehouse库存上限动态变化导致UI混乱 | 低 | 超出部分保留不可新增；UI始终显示当前有效上限 |
| LLM不遵守生存相关Prompt指令 | 中 | RuleEngine兜底：satiety/hydration低时强制NPC优先饮食；LLM Agent也有RuleEngine fallback |

## 7. 迁移方案

### 7.1 部署步骤

1. Rust Core各模块按Phase顺序实现并测试（生存→建筑→压力→里程碑）
2. Bridge Delta扩展与Core同步
3. Godot客户端最后更新（依赖Core层完成）
4. 更新config/sim.toml新增消耗速率和压力间隔配置项

### 7.2 回滚方案

- 生存消耗可通过配置开关控制（sim.toml新增enable_survival_consumption = true）
- 建筑效果同理（enable_structure_effects = true）
- 压力事件已有Pool但当前不生成，加个开关即可

## 8. 待定事项

- [ ] 生存消耗衰减速率是否需要可配置（当前硬编码2/tick和2.5/tick）
- [ ] 丰饶事件结束后已翻倍的食物是否需要回退（当前设计不回退）
- [ ] 城邦雏形里程碑的"盟友对数≥2"定义需确认（双向信任>50的对数？还是Ally关系对数？）