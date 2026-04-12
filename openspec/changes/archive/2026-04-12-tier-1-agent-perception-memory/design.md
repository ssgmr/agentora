# Design: Agent 感知与记忆接线

## 上下文

当前 Agent 决策时处于"失明+失忆"状态：
- Vision 扫描只遍历东北1/4象限（`saturating_add` Bug）
- `perceive_nearby()` 定义完整但从未被调用，Agent 探测分支是 TODO
- `memory.record()` 零调用，记忆系统从未写入
- 关系数据（`relations`）不进入决策 Prompt
- `build_perception_summary()` 只有 Agent 数量，无位置/名字/关系

目标是接通这些断线，让 Agent 决策时拥有完整的周边环境感知、短期记忆和社交关系上下文。

## 决策一：Vision 扫描方案 — `scan_vision()` 独立函数

**选择：在 `core` 中新增 `vision.rs` 模块，提供 `scan_vision(&World, &AgentId, radius)` 纯函数**

### 删除 `perceive_nearby()` 及关联类型

`movement.rs` 中的 `perceive_nearby()`、`PerceivedAgent`、`PerceivedResource`、`PerceptionResult` 全部删除。

原因：
- 从未被调用，是半成品
- 闭包接口别扭（`Fn(&AgentId)` 需要外部传入所有 ID 列表才能遍历）
- Agent 探测分支是空 TODO
- 不包含地形扫描
- `PerceivedAgent` 缺少 name、relation_type、trust 字段

### 新设计

#### 核心问题：空间查询 vs ID 索引

当前 `World` 的数据组织方式存在根本不匹配：

```
World 的空间数据结构分析：

┌───────────────────────────────────────────────────────┐
│  CellGrid (地形)                                      │
│    Vec<TerrainType>，一维数组模拟2D网格                  │
│    get_terrain(pos) → O(1) 天然空间索引，已有，OK       │
├───────────────────────────────────────────────────────┤
│  resources: HashMap<Position, ResourceNode>            │
│    按位置精确查找 → O(1) ✓                             │
│    范围查询 → 遍历全部 values → O(N) ✗                 │
│    ← 设计债                                            │
├───────────────────────────────────────────────────────┤
│  agents: HashMap<AgentId, Agent>                       │
│    按身份精确查找 → O(1) ✓                             │
│    范围查询 → 遍历全部 values，算距离 → O(N) ✗          │
│    ← 设计债                                            │
├───────────────────────────────────────────────────────┤
│  structures: HashMap<Position, Structure>              │
│    同 resources，范围查询需遍历 → O(N) ✗                │
│    ← 设计债                                            │
└───────────────────────────────────────────────────────┘
```

`scan_vision` 需要按空间范围查询（"5格内有哪些实体"），但 `resources` 和 `agents` 都不支持范围查询。

#### 关键洞察：遍历位置，不遍历实体

```
错误思路：遍历 world.resources（9830个节点），对每个算距离 → O(N)
错误思路：遍历 world.agents（50个），对每个算距离    → O(N)

正确思路：遍历中心±r的方形区域（11×11=121个位置）
         → 每个位置查 HashMap
         → O(r²) = O(1)，跟实体总数无关

    ┌───────────────────┐
    │  · · · · · · · ·  │  ← 不遍历这些
    │  · ┌───────┐ · ·  │
    │  · │ █ R █ │ · ·  │  ← 只遍历这个 11×11 区域
    │  · │ R █ R │ · ·  │      曼哈顿距离 ≤ 5 的位置
    │  · │ █ R █ │ · ·  │      查 HashMap 是否有实体
    │  · └───────┘ · ·  │
    │  · · · · · · · ·  │
    └───────────────────┘
       █ = Agent  R = Resource
```

但 Agent 有额外的复杂度：`HashMap<AgentId, Agent>` 按 ID 索引，同一位置可能有多个 Agent，无法通过位置直接查找。

#### 解决方案：`agent_positions` 反向索引

```rust
// World 新增字段
pub struct World {
    // ... 现有字段不变
    pub agent_positions: HashMap<Position, Vec<AgentId>>,  // 位置 → Agent ID列表
}
```

`scan_vision` 实现逻辑：

```rust
pub fn scan_vision(world: &World, agent_id: &AgentId, radius: u32) -> VisionScanResult {
    let agent = world.agents.get(agent_id).unwrap();
    let cx = agent.position.x;
    let cy = agent.position.y;
    let min_x = cx.saturating_sub(radius);
    let max_x = (cx + radius).min(world.map.size().0 - 1);
    let min_y = cy.saturating_sub(radius);
    let max_y = (cy + radius).min(world.map.size().1 - 1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let pos = Position::new(x, y);
            // 曼哈顿距离过滤
            if pos.manhattan_distance(&agent.position) > radius {
                continue;
            }
            if pos == agent.position { continue; } // 跳过自身位置

            // 1. 地形：O(1)
            let terrain = world.map.get_terrain(pos);

            // 2. 资源：O(1) HashMap 查询
            if let Some(node) = world.resources.get(&pos) {
                // 收集 (ResourceType, current_amount)
            }

            // 3. Agent：通过反向索引 O(1)
            if let Some(ids) = world.agent_positions.get(&pos) {
                for id in ids {
                    if id == agent_id { continue; } // 跳过自己
                    let other = world.agents.get(id).unwrap();
                    // 查 agent.relations 获取关系数据
                    // 填充 NearbyAgentInfo
                }
            }
        }
    }
}
```

**复杂度：O(r²) = O(25) = O(1)，跟世界大小和实体总数无关。**

不需要引入 EntityGrid、四叉树等复杂结构——121 次 HashMap 查询已经足够高效。未来如果需要更大的视野半径或更密集的世界，再引入空间索引也不迟，`scan_vision` 接口无需改变。

#### 反向索引维护

```
Agent 移动时（apply_action → Move）：
  1. 从 agent_positions[旧位置] 的 Vec 中移除旧 ID
  2. 更新 agent.position
  3. 将 ID 推入 agent_positions[新位置] 的 Vec

Agent 生成时（generate_agents）：
  agent_positions.entry(pos).or_default().push(id)

Agent 死亡时（check_agent_death）：
  agent_positions[旧位置].retain(|id| *id != dead_id)

结构体创建/销毁时：
  同 Agent 逻辑，但 structures 按 Position 索引，
  不需要额外反向索引（直接遍历位置查 HashMap 即可）
```

#### `scan_vision` 接口

```rust
// crates/core/src/vision.rs

pub struct NearbyAgentInfo {
    pub id: AgentId,
    pub name: String,
    pub position: Position,
    pub distance: u32,                   // 曼哈顿距离
    pub motivation_summary: [f32; 6],
    pub relation_type: RelationType,     // 对自己的关系
    pub trust: f32,
}

pub struct VisionScanResult {
    pub self_position: Position,
    pub terrain_at: HashMap<Position, TerrainType>,
    pub resources_at: HashMap<Position, (ResourceType, u32)>,
    pub nearby_agents: Vec<NearbyAgentInfo>,
}

pub fn scan_vision(world: &World, agent_id: &AgentId, radius: u32) -> VisionScanResult
```

bridge 调用方式：

```
bridge/src/lib.rs (run_agent_loop)
    │
    ├── scan_vision(&world, &agent_id, 5)  ← 一行调用，零手写扫描逻辑
    │
    ├── VisionScanResult 映射到 WorldState
    │
    ├── DecisionPipeline::execute(...)
    │
    └── 序列化 WorldSnapshot 推送 Godot
```

### 方案对比

```
┌──────────────────┬─────────────────┬─────────────────────┬─────────────────────┐
│                  │ 方案A           │ 方案B                │ 方案C（选中）        │
│                  │ 在bridge手写修复 │ 调用perceive_nearby │ scan_vision独立函数  │
├──────────────────┼─────────────────┼─────────────────────┼─────────────────────┤
│ 改动量            │ 小              │ 中（补全TODO+改接口）│ 中（新模块+新类型）   │
│ 分层纯净度        │ 违反（业务逻辑  │ 符合（感知在core）   │ 符合（感知在core）   │
│                  │   写在bridge）   │                     │                     │
│ 可测试性          │ 低（嵌入async   │ 高（闭包调用）       │ 高（纯函数&World）   │
│                  │   task中）       │                     │                     │
│ 代码复用          │ 无              │ 中（挂在Agent上）    │ 高（独立函数）       │
│ 地形扫描          │ 仍需手写         │ 需额外扩展接口       │ 天然包含            │
│ 查询复杂度        │ O(N)遍历Agent   │ O(N)遍历Agent       │ O(r²) 常数时间      │
│ 扩展性            │ 差              │ 中（受闭包接口限制）  │ 高（参数化，接口稳定）│
│ bridge复杂度      │ 增加             │ 减少                │ 最少（一行调用）     │
└──────────────────┴─────────────────┴─────────────────────┴─────────────────────┘
```

选择方案C的理由：
1. bridge 只做桥接，不写业务逻辑
2. 纯函数，可独立单元测试
3. 天然包含地形/资源/Agent 三类扫描
4. 查询复杂度 O(r²)，跟世界大小和实体总数无关
5. 关系数据在 scan_vision 内部填充
6. 接口稳定，未来引入空间索引时无需改动调用方

### 扩展性预留

```
scan_vision(world, agent_id, radius)
  ├── 动态视野半径：不同 Agent 可有不同 radius（参数化，已支持）
  ├── 地形阻挡：新增 blocking 参数，扫描时跳过阻挡方向
  ├── 非 Agent 视角：scan_vision_at_pos(world, pos, radius)
  │     复用同一套扫描逻辑，只是中心点不是 Agent 位置
  ├── 空间索引升级：当 r 很大或实体很密集时
  │     → World 内部换 EntityGrid/四叉树
  │     → scan_vision 接口不变，内部实现替换
  └── 多模态感知：
        scan_hearing(world, agent_id, range)  ← 声音传播
        scan_smell(world, agent_id, range)    ← 气味痕迹
```

## 决策二：WorldState 扩展

**选择：扩展现有结构，不新建中间层**

在 `WorldState` 中新增字段：

```rust
pub struct WorldState {
    // ... 现有字段
    pub nearby_agents: Vec<NearbyAgentInfo>,  // 新增
    pub resources_at: HashMap<Position, (ResourceType, u32)>,  // 改为带数量
}
```

`scan_vision()` 的返回类型 `VisionScanResult` 与 `WorldState` 字段一一对应，bridge 只需字段映射。

## 决策三：记忆接入方案

**选择：锁内纯计算 + 锁外 I/O**

`MemorySystem` 需要初始化 ChronicleDB（SQLite）和 ChronicleStore（文件系统），这在 Bridge 的 `run_agent_loop` 中每次决策都做会非常昂贵。

方案：
1. 在 Agent 创建时初始化一次记忆系统（`World::generate_agents` 中调用 `init_chronicle_db` + `init_chronicle_store`）
2. 每次 `apply_action` 后调用 `memory.record()` 写入短期记忆 + ChronicleDB
3. 每次决策时通过 `agent.memory.get_summary(spark_type)` 获取摘要注入 Prompt

但 `MemorySystem` 不是 `Clone` 友好（SQLite 连接不能 Clone），而 `run_agent_loop` 中通过 `world.lock()` 获取 Agent 的 clone。

**关键问题**：`get_summary()` 内部执行 SQLite 查询（ChronicleDB）和文件读取（ChronicleStore），这些 I/O 操作如果在持锁期间执行会显著延长持锁时间，阻塞 apply_loop 和其他 Agent 的决策。

**解决方案**：

```
run_agent_loop:
  1. lock world
  2. scan_vision(&world, &agent_id, 5) → VisionScanResult  ← 纯计算，快
  3. clone Agent 的必要字段（id, name, position, motivation, relations, memory）
  4. 释放 lock                                              ← 尽早释放
  5. agent.memory.get_summary(spark_type) → memory_str      ← 锁外 I/O
  6. 构建 WorldState（从 VisionScanResult 映射）
  7. pipeline.execute()                                     ← LLM 调用，本身不持锁
```

`MemorySystem` 的 `Clone` 实现已经会重新打开 SQLite 连接（`clone_chronicle_db` → `ChronicleDB::new(&db_path)`），所以 clone 后独立使用是可行的。每次决策打开一次 SQLite 连接的开销可接受（本地 SQLite 连接建立约 1-5ms），远低于 LLM 调用（数秒到数十秒）。

**为什么不共享 MemorySystem 引用**：
- `Arc<MemorySystem>` 方案需要额外的同步机制，而 SQLite 连接本身是线程安全的（rusqlite 默认 `SQLITE_OPEN_FULL_MUTEX`）
- clone 重连方案更简单，且与现有 `Agent: Clone` 兼容
- 如果未来发现重连开销成为瓶颈，再改为 `Arc<ChronicleDB>` 共享连接池

## 决策四：关系数据传递

**选择：在 `scan_vision` 内部填充关系数据**

`scan_vision()` 遍历 121 个位置时，通过 `world.agent_positions` 反向索引获取每个位置的 Agent ID：
1. 跳过自身 Agent
2. 通过 `world.agents.get(id)` 获取 Agent 详情
3. 查**当前 Agent 的 relations** 获取 `relation_type` 和 `trust`
4. 不存在则为 `Neutral` / `0.0`

关系数据填充在 `core` 中完成，不依赖 bridge 传递。

## 决策六：`agent_positions` 反向索引维护 — 统一入口

**选择：在 `apply_action` 统一处理位置变化，不分散到各动作分支**

`apply_action` 中会改变 Agent 位置的动作不止 Move，还有 Explore（在 `handle_special_action` 中随机移动 1-3 步）。如果反向索引的维护散落在各个动作分支中，未来新增移动类动作时极易遗漏。

### 统一维护方案

```rust
pub fn apply_action(&mut self, agent_id: &AgentId, action: &Action) -> ActionResult {
    // 记录旧位置
    let old_position = self.agents.get(agent_id).map(|a| a.position);

    // 执行原有动作逻辑
    let result = match &action.action_type { ... };

    // 统一检查位置变化并维护反向索引
    if let (Some(old_pos), Some(agent)) = (old_position, self.agents.get(agent_id)) {
        if old_pos != agent.position {
            // 从旧位置移除
            if let Some(ids) = self.agent_positions.get_mut(&old_pos) {
                ids.retain(|id| *id != *agent_id);
                if ids.is_empty() { self.agent_positions.remove(&old_pos); }
            }
            // 加入新位置
            self.agent_positions.entry(agent.position)
                .or_default().push(agent_id.clone());
        }
    }

    result
}
```

### 需要维护的场景

| 场景 | 维护方式 | 位置 |
|------|----------|------|
| Agent 生成 | `agent_positions.entry(pos).or_default().push(id)` | `generate_agents` |
| Agent 移动/探索 | 统一在 `apply_action` 出口比较旧新位置 | `apply_action` |
| Agent 死亡 | `agent_positions[位置].retain(\|id\| *id != dead_id)` | `check_agent_death` |
| NPC 创建 | 同 Agent 生成 | `create_npc_agents`（bridge 中） |

### NPC 创建的遗漏修复

`create_npc_agents` 在 bridge 中直接 `world.agents.insert(...)`，绕过了 `World::generate_agents`。需要在 NPC 插入后同步初始化 `agent_positions`：

```rust
// bridge/src/lib.rs: create_npc_agents
world.agents.insert(aid.clone(), agent);
world.agent_positions.entry(Position::new(x, y)).or_default().push(aid.clone());
```

为此需要在 `World` 中公开 `agent_positions` 字段（`pub`），或提供 `insert_agent_at(&mut self, id, agent)` 方法统一处理插入+索引初始化。选择后者，封装性更好。

## 变更数据流

```
┌─────────────────────────────────────────────────────────────┐
│  run_agent_loop (修复后)                                     │
│                                                             │
│  【持锁阶段 — 纯计算，快速完成】                               │
│  1. lock world                                              │
│  2. scan_vision(&world, &agent_id, 5) → VisionScanResult   │  ← 新，O(r²) 纯计算
│  3. clone Agent 必要字段（id, name, position, motivation,    │
│        relations, memory）                                    │
│  4. 释放 lock                                               │  ← 尽早释放，不阻塞其他 task
│                                                             │
│  【锁外阶段 — I/O + LLM】                                    │
│  5. agent.memory.get_summary(spark_type) → memory_str       │  ← SQLite/文件 I/O
│  6. 从 VisionScanResult 构建 WorldState                      │
│  7. pipeline.build_prompt_with_memory(..., memory_str)       │
│     ├─ perception: 位置 + 附近Agent(名字/距离/关系)          │
│     │               + 资源(位置/类型/数量) + 地形            │
│     ├─ memory: 最近行动记录 + Chronicle 摘要                 │
│     └─ 动机 + Spark                                         │
│  8. LLM 决策                                                │
│  9. 发送 action 到 apply_loop                                │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  run_apply_loop (新增)                                       │
│  1. lock world                                              │
│  2. apply_action()                                          │
│     ├─ 记录旧位置                                            │
│     ├─ 执行原有动作逻辑                                       │
│     └─ 统一维护 agent_positions（位置变化时）   ← 新          │
│  3. agent.memory.record(MemoryEvent { ... }) ← 新增         │
│  4. unlock world                                            │
└─────────────────────────────────────────────────────────────┘
```

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `crates/core/src/vision.rs` | **新建** | `scan_vision()` 函数 + `VisionScanResult` + `NearbyAgentInfo` |
| `crates/core/src/agent/movement.rs` | **修改** | 删除 `perceive_nearby()` 及关联类型 |
| `crates/core/src/world/mod.rs` | **修改** | 新增 `agent_positions` 字段 + `insert_agent_at()` 方法 + `apply_action` 统一维护反向索引 |
| `crates/core/src/rule_engine.rs` | **修改** | `WorldState` 扩展 `nearby_agents`，`resources_at` 改 tuple |
| `crates/core/src/decision.rs` | **修改** | `build_perception_summary` 扩展输出，接入 `memory_summary` |
| `crates/bridge/src/lib.rs` | **修改** | 删除手写扫描循环，改为调用 `scan_vision()`；NPC 创建改用 `insert_agent_at()` |
| `crates/core/src/world/generator.rs` | **修改** | Agent 创建时初始化记忆系统 + 改用 `insert_agent_at()` |

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| MemorySystem clone 重连开销 | 每次决策打开一次 SQLite 连接 | 本地 SQLite 连接建立约 1-5ms，远低于 LLM 调用（秒级），可接受 |
| 5 格半径扫描 11×11=121 次 HashMap 查询 | 轻微性能影响 | 可接受，决策本身 LLM 调用占主导 |
| NPC 不使用 LLM 决策，是否也需要记忆 | 不需要 | NPC 保持现状，不修改 |
| Prompt token 增加导致超限 | Prompt 有分级截断机制 | 已有 `smart_truncate` 保护 |
| `agent_positions` 反向索引不一致 | 扫描遗漏/重复 | `apply_action` 统一维护 + 死亡/生成同步 + `insert_agent_at()` 封装，加单元测试验证 |
| 同一位置多 Agent 的 Vec 增长 | 查询变慢 | 限制同位置 Agent 数量（通常 ≤ 3），未来加上限 |
| NPC 创建绕过 `generate_agents` | 遗漏 `agent_positions` 初始化 | 使用 `insert_agent_at()` 统一封装，NPC 和 LLM Agent 共用同一入口 |
| `Explore` 等动作改变位置未维护索引 | 反向索引漂移 | `apply_action` 出口统一比较旧新位置，不依赖具体动作类型 |
