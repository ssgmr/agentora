# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

项目采用 Godot 4 GDScript 客户端 + Rust GDExtension 桥接的双线程架构：

- **Godot 主线程**：负责渲染、输入处理、UI 交互
- **Rust 模拟线程**：通过 `std::thread::spawn` 创建，内嵌 Tokio 运行时，运行世界模拟
- **通信机制**：两个 mpsc 通道
  - `Sender<WorldSnapshot>` → 模拟线程推送完整世界快照给 Godot
  - `Sender<SimCommand>` ← Godot 发送控制命令（Start/Pause/调速等）

最终产品目标是 P2P 分布式架构：每个玩家控制自己的 Agent，通过 P2P 异步同步世界状态。

### 1.2 现状分析

当前模拟循环（`crates/bridge/src/lib.rs:367-393`）采用 **World-driven 全量同步模型**：

```
loop:
  world.advance_tick()           // 世界推进一tick
  for agent in agents:           // 顺序遍历所有agent
    decision(agent).await         // 等待LLM/规则引擎决策
    apply_action(agent)           // 应用动作
  snapshot = world.snapshot()    // 生成完整快照
  tx.send(snapshot)              // 推送给Godot
  sleep(tick_interval)           // 等待固定间隔
```

**存在的问题**：
1. 玩家的 Agent 决策完成后不能立即渲染，必须等其他所有 Agent 执行完 + snapshot 生成
2. 5个 Agent 串行 LLM 调用（每个 2-10s），tick 总耗时远超 5s 间隔
3. 与未来 P2P 架构不兼容——分布式系统不存在"所有 Agent 执行完"的概念
4. Godot 端 `_on_world_updated` 收到 snapshot 后遍历所有 Agent 做"存在则更新/不存在则创建/多余则删除"，虽然逻辑上已经是增量更新，但触发时机不对（必须等全量 snapshot）

### 1.3 关键干系人

- Rust bridge crate：`crates/bridge/src/lib.rs` — 模拟循环和通信
- Rust core crate：`crates/core/src/` — World、Agent、决策管道
- Godot 客户端：`client/scripts/agent_manager.gd` — Agent 渲染管理
- Godot 客户端：`client/scripts/simulation_bridge.gd` — GDScript 侧的桥接（与 GDExtension 并存）

## 2. 设计目标

### 目标

- 每个 Agent 独立决策循环，决策完成后立即推送 delta 事件给 Godot
- Godot 端支持增量更新，收到 delta 后立即渲染
- 保留定期 snapshot 机制用于一致性校验
- NPC 使用规则引擎快速决策，用于开发验证
- 修复纹理资源加载错误

### 非目标

- P2P 网络同步（本变更仅为本地的 Agent 独立心跳改造，P2P 同步后续变更处理）
- 多玩家联机和跨节点状态同步
- 性能优化（LLM 调用速度、本地模型推理等）

## 3. 整体架构

### 3.1 架构概览

```
┌──────────────────────────────────────────────────────────────┐
│                     Rust 模拟线程                               │
│                                                              │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐     │
│  │  Agent A     │   │  Agent B     │   │  Agent C     │     │
│  │  独立循环     │   │  独立循环     │   │  独立循环     │     │
│  │  tick=2s     │   │  tick=3s     │   │  tick=1.5s   │     │
│  └──────┬───────┘   └──────┬───────┘   └──────┬───────┘     │
│         │                  │                  │              │
│    delta A            delta B            delta C             │
│         │                  │                  │              │
│         ▼                  ▼                  ▼              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │        agent_delta_sender (mpsc channel)              │    │
│  └────────────────────────┬────────────────────────────┘    │
│                           │                                 │
│                    ┌──────▼──────┐                          │
│                    │ snapshot_loop│ (每5s兜底)               │
│                    └──────┬──────┘                          │
│                           │                                 │
└───────────────────────────┼─────────────────────────────────┘
                            │ mpsc
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                    Godot 主线程                                │
│                                                              │
│  physics_process() 轮询:                                      │
│    优先处理 delta 事件 → 增量更新对应 Agent sprite              │
│    定期处理 snapshot → 一致性校验和脏数据清理                    │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件名 | 路径 | 职责 |
| --- | --- | --- |
| Agent 独立循环 | `crates/bridge/src/lib.rs` | 每个 Agent spawn 独立的 `tokio::spawn` 循环，决策完发 delta |
| AgentDelta 类型 | `crates/bridge/src/lib.rs` | 定义增量事件类型（Move/StateChange/Death/Spawn） |
| 双通道架构 | `crates/bridge/src/lib.rs` | delta 通道（实时）+ snapshot 通道（兜底） |
| NPC 快速决策 | `crates/bridge/src/lib.rs` | NPC 跳过 LLM，直接用规则引擎 |
| AgentManager | `client/scripts/agent_manager.gd` | 支持 delta 增量更新 + snapshot 一致性校验 |
| 纹理资源 | `client/assets/sprites/` + `client/assets/textures/` | SVG → PNG 重新生成 |

### 3.3 数据流设计

**Agent 决策到渲染的完整流程**：

```
时序图:

Agent循环                World                  delta通道              Godot
  │                       │                       │                     │
  ├─ 决策计时器到期         │                       │                     │
  ├─ 构建WorldState ──────►│                       │                     │
  ├─ 执行决策(LLM/规则)     │                       │                     │
  ├─ 应用动作到World ─────►│                       │                     │
  ├─ 构建AgentDelta ───────┼──────────────────────►│                     │
  │                        │   try_recv(delta)     │                     │
  │                        │ ◄─────────────────────┤                     │
  │                        │                       │                     │
  │                        │                       │   增量更新sprite     │
  │                        │                       │   _update_agent()    │
  │                        │                       │                     │
  │                        │                       │   (不等其他Agent)    │
```

**NPC 快速决策流程**：

```
NPC循环
  ├─ 决策计时器到期 (默认1s)
  ├─ 构建WorldState
  ├─ 规则引擎直接生成动作 (不调LLM, <3ms)
  ├─ 应用动作到World
  └─ 发送AgentDelta
```

**NPC 数量可配置**：开发验证阶段通过配置参数控制 NPC 数量（建议 5-10 个），正式分发时设为 0。用户只控制自己的 Agent，不需要 NPC。

```rust
struct NpcConfig {
    count: usize,         // NPC 数量，MVP=5-10，生产=0
    decision_interval: u64, // 决策间隔（秒）
}
```

## 4. 详细设计

### 4.1 新增数据类型

#### AgentDelta 枚举

定义在 `crates/bridge/src/lib.rs`：

```rust
/// Agent 增量事件
#[derive(Debug, Clone)]
pub enum AgentDelta {
    /// Agent 移动或状态变化
    AgentMoved {
        id: String,
        name: String,
        position: (u32, u32),
        health: u32,
        max_health: u32,
        is_alive: bool,
        age: u32,
        current_action: String,
        motivation: [f32; 6],
    },
    /// Agent 死亡
    AgentDied {
        id: String,
        name: String,
        position: (u32, u32),
        age: u32,
    },
    /// 新 Agent 诞生
    AgentSpawned {
        id: String,
        name: String,
        position: (u32, u32),
        health: u32,
        max_health: u32,
        motivation: [f32; 6],
    },
}
```

#### 双通道

```rust
// 新增: AgentDelta 通道（实时）
let (delta_tx, delta_rx) = mpsc::channel::<AgentDelta>();
// 保留: WorldSnapshot 通道（兜底）
let (snapshot_tx, snapshot_rx) = mpsc::channel::<WorldSnapshot>();
```

### 4.2 核心算法

#### Agent 独立循环

```rust
/// 启动所有 Agent 的独立决策循环
fn spawn_agent_loops(
    world: &mut World,
    delta_tx: Sender<AgentDelta>,
    pipeline: &DecisionPipeline,
) -> Vec<JoinHandle<()>> {
    let agent_ids: Vec<_> = world.agents.keys().cloned().collect();
    let mut handles = Vec::new();

    for agent_id in agent_ids {
        let tx = delta_tx.clone();
        // 每个 agent 在自己的 tokio task 中运行
        let handle = tokio::spawn(run_agent_loop(
            world.clone(),  // 需要 Arc<Mutex<World>> 或改为无锁设计
            agent_id,
            pipeline,
            tx,
        ));
        handles.push(handle);
    }

    handles
}
```

**关键设计决策：World 的并发访问**

当前 `World` 不是 `Send + Sync`，需要改造。有两种方案：

| 方案 | 做法 | 优点 | 缺点 |
|------|------|------|------|
| A. Arc<Mutex<World>> | 全局锁 | 改动最小 | 串行化 World 访问，性能瓶颈 |
| B. 读写分离 | 决策读取 WorldState 快照，动作通过 channel 串行 apply | 真正的并发决策 | 需要重构 World 访问模式 |

**选择方案 B**：决策时不直接读 World，而是每个 agent 决策前获取一次 World 的快照（WorldState），决策完成后将动作通过 channel 发送到 apply 循环，由 apply 循环串行应用。

```
┌─────────────────────────────────────────────────────────────┐
│                    并发决策架构 (方案B)                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Agent A task          Agent B task          Agent C task   │
│  ┌─────────┐          ┌─────────┐          ┌─────────┐    │
│  │ 读World  │          │ 读World  │          │ 读World  │    │
│  │ 快照     │          │ 快照     │          │ 快照     │    │
│  └────┬────┘          └────┬────┘          └────┬────┘    │
│       │                    │                    │          │
│  ┌────▼────┐          ┌────▼────┐          ┌────▼────┐    │
│  │ LLM决策  │          │ LLM决策  │          │ LLM决策  │    │
│  │ (并发!)  │          │ (并发!)  │          │ (并发!)  │    │
│  └────┬────┘          └────┬────┘          └────┬────┘    │
│       │                    │                    │          │
│       └────────────────────┼────────────────────┘          │
│                            ▼                               │
│                   ┌──────────────┐                         │
│                   │ action_rx    │                         │
│                   └──────┬───────┘                         │
│                          ▼                                 │
│                   ┌──────────────┐                         │
│                   │ Apply循环     │ ← 串行应用动作            │
│                   │ (独占World)   │                         │
│                   └──────┬───────┘                         │
│                          │                                 │
│                   ┌──────▼───────┐                         │
│                   │ 发AgentDelta  │                         │
│                   └──────────────┘                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

伪代码：

```rust
// 主循环：不再遍历 agent，而是管理 apply 循环和 snapshot
async fn run_simulation(
    delta_tx: Sender<AgentDelta>,
    snapshot_tx: Sender<WorldSnapshot>,
    cmd_rx: Receiver<SimCommand>,
    llm_provider: Option<Box<dyn LlmProvider>>,
) {
    let mut world = World::new(&seed);
    create_initial_agents(&mut world);

    let pipeline = build_pipeline(llm_provider);

    // 共享的 World（用 Arc 包装，决策 task 只读）
    let world_arc = Arc::new(Mutex::new(world));

    // 动作接收通道
    let (action_tx, mut action_rx) = mpsc::channel::<(AgentId, Action)>();

    // 为每个 Agent spawn 决策 task
    let agent_ids: Vec<_> = world_arc.lock().unwrap().agents.keys().cloned().collect();
    let mut tasks = Vec::new();

    for agent_id in agent_ids {
        let w = world_arc.clone();
        let tx = action_tx.clone();
        let p = pipeline.clone();
        tasks.push(tokio::spawn(async move {
            agent_loop(w, agent_id, p, tx).await;
        }));
    }

    // Apply 循环：串行应用动作，发 delta
    let delta_tx_clone = delta_tx.clone();
    let w = world_arc.clone();
    tokio::spawn(async move {
        while let Ok((agent_id, action)) = action_rx.recv().await {
            let mut world = w.lock().unwrap();
            world.apply_action(&agent_id, &action);

            // 构建 delta 事件
            if let Some(agent) = world.agents.get(&agent_id) {
                let delta = AgentDelta::AgentMoved { ... };
                let _ = delta_tx_clone.send(delta);
            }
        }
    });

    // 定期 snapshot（兜底）
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let world = world_arc.lock().unwrap();
        let snapshot = world.snapshot();
        let _ = snapshot_tx.send(snapshot);
    }
}

// 每个 Agent 的独立决策循环
async fn agent_loop(
    world: Arc<Mutex<World>>,
    agent_id: AgentId,
    pipeline: DecisionPipeline,
    action_tx: Sender<(AgentId, Action)>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));

    loop {
        interval.tick().await;

        // 检查是否存活
        {
            let world = world.lock().unwrap();
            let agent = match world.agents.get(&agent_id) {
                Some(a) if a.is_alive => a.clone(),
                _ => break, // 死亡或不存在，退出循环
            };
        }

        // 构建 WorldState（快速快照）
        let world_state = {
            let world = world.lock().unwrap();
            build_world_state(&world, &agent_id)
        };

        // LLM 决策（并发，不阻塞其他 agent）
        let action = {
            let world = world.lock().unwrap();
            let agent = world.agents.get(&agent_id).unwrap();
            // ... 执行决策管道
            agent_decision(&pipeline, &world, &agent_id).await
        };

        // 发送动作到 apply 循环
        let _ = action_tx.send((agent_id, action));
    }
}
```

### 4.3 Godot 端修改

#### 移除 simulation_bridge.gd

`client/scripts/simulation_bridge.gd` 是 GDScript 模拟桥接的占位实现，生成随机 Agent 移动和假数据。场景文件 `main.tscn` 中 `SimulationBridge` 节点使用的是 Rust GDExtension 类型（`type="SimulationBridge"`），与 GDScript 脚本无关。**直接删除该文件**。

#### AgentManager 增量更新

当前 `_on_world_updated` 已经做了增量更新（`_update_agent` 只改存在的 agent），但问题是只在 snapshot 到达时触发。修改后：

```gdscript
# 新增：处理 delta 事件
func _on_agent_delta(delta_data: Dictionary) -> void:
    var event_type = delta_data.get("type", "")

    match event_type:
        "agent_moved":
            var agent_data = delta_data.get("data", {})
            var agent_id = delta_data.get("id", "")
            _update_or_create_agent(agent_id, agent_data)

        "agent_died":
            var agent_id = delta_data.get("id", "")
            _remove_agent(agent_id)
            # 在死亡位置创建 Legacy
            var pos = delta_data.get("position", Vector2.ZERO)
            _create_legacy_at(pos)

        "agent_spawned":
            var agent_data = delta_data.get("data", {})
            var agent_id = delta_data.get("id", "")
            _create_agent_node(agent_id, agent_data)

# 修改：snapshot 用于一致性校验
func _on_world_updated(snapshot: Dictionary) -> void:
    var agents: Dictionary = snapshot.get("agents", {})

    # 一致性校验：创建 snapshot 中有但本地缺失的 agent
    for agent_id in agents.keys():
        if not _agent_nodes.has(agent_id):
            var agent_data = agents[agent_id]
            _create_agent_node(agent_id, agent_data)
            print("[AgentManager] 一致性修复：创建缺失的 Agent ", agent_id)

    # 一致性校验：删除本地有但 snapshot 中不存在的 agent（幽灵 agent）
    for existing_id in _agent_nodes.keys():
        if not agents.has(existing_id):
            _remove_agent(existing_id)
            print("[AgentManager] 一致性修复：移除幽灵 Agent ", existing_id)
```

#### SimulationBridge 双通道

修改 `SimulationBridge` 结构体，增加 delta 接收通道：

```rust
pub struct SimulationBridge {
    base: Base<Node>,
    command_sender: Option<Sender<SimCommand>>,
    snapshot_receiver: Option<Receiver<WorldSnapshot>>,
    delta_receiver: Option<Receiver<AgentDelta>>,  // 新增
    current_tick: i64,
    // ...
}
```

在 `physics_process` 中优先处理 delta：

```rust
fn physics_process(&mut self, _delta: f64) {
    // 1. 优先处理 delta（实时）
    if let Some(receiver) = &self.delta_receiver {
        while let Ok(delta) = receiver.try_recv() {
            let delta_dict = Self::delta_to_dict(&delta);
            self.base_mut().emit_signal("agent_delta", &[delta_dict.to_variant()]);
        }
    }

    // 2. 再处理 snapshot（一致性校验）
    if let Some(receiver) = &self.snapshot_receiver {
        if let Ok(snapshot) = receiver.try_recv() {
            // ... 现有 snapshot 处理逻辑
        }
    }
}
```

### 4.4 纹理资源修复

#### 纹理尺寸决策

通过实际截图验证（16x16 terrain 纹理），在当前渲染尺度下视觉效果正常，**不需要调整尺寸**。

#### SVG → PNG 重新生成

修改 `client/assets/svg_to_png.py`：

1. 确保所有 SVG 源文件被正确导出
2. `agent.svg` 纳入导出列表，输出为 `agent.png`（32x32）
3. 纹理尺寸从 16x16 提升到 32x32（当前 16x16 太小，16x16 的 PNG 只有 200-400 字节，细节严重不足）

#### Godot 导入缓存修复

1. 删除 `.godot/imported/` 目录
2. 重新生成 PNG 后，Godot 启动时自动重新导入
3. 验证启动日志无纹理加载错误

### 4.6 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| Agent 决策 panic | catch_unwind 包裹，记录错误并终止该 agent 的循环 |
| delta 通道满 | delta 通道使用有界 channel（容量 1024），满时丢弃旧事件，下次 snapshot 兜底修复 |
| Godot 端 delta 处理慢 | physics_process 中用 `while let Ok` 循环，每帧最多处理 100 个 delta，剩余留给下一帧 |
| 纹理文件损坏 | 保留颜色回退机制（`_agent_idle_texture == null` 时使用 AGENT_COLOR） |
| LLM 持续失败 | FallbackChain 已处理，最终回退到规则引擎 |
| Arc<Mutex<World>> 死锁 | 每次 lock 后立即 unlock，不在持有锁期间 await |

## 5. 技术决策

### 决策1：World 并发访问方案

- **选型方案**：Arc<Mutex<World>> + 读写分离（决策读快照，动作通过 channel 串行 apply）
- **选择理由**：
  - 决策阶段（LLM 调用）是耗时操作，不持有 World 锁
  - Apply 阶段串行，保证状态一致性
  - 天然映射到 P2P 架构：每个节点独立决策自己的 Agent
- **备选方案**：Arc<Mutex<World>> 全锁
- **放弃原因**：决策和 apply 都持锁，退化为顺序执行，无并发收益

### 决策2：事件通道设计

- **选型方案**：双通道（delta 实时 + snapshot 兜底）
- **选择理由**：
  - delta 通道保证实时性（决策完即推）
  - snapshot 通道保证一致性（修复丢失/重复的 agent）
  - 两通道解耦，互不影响
- **备选方案**：单通道混合事件
- **放弃原因**：混合事件需要 godot 端区分事件类型并设置优先级，逻辑更复杂

### 决策3：NPC 决策策略

- **选型方案**：NPC 用规则引擎直接决策，跳过 LLM
- **选择理由**：
  - NPC 用于开发验证，不需要 LLM 的"人性化"决策
  - 规则引擎 <3ms，远低于 LLM 的秒级延迟
  - NPC 数量多时可以快速填充世界
- **备选方案**：NPC 也用 LLM
- **放弃原因**：MVP 阶段 LLM 资源有限，NPC 全走 LLM 会严重拖慢系统

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| Arc<Mutex<World>> 锁竞争 | 中 | 决策阶段不持锁（只快速快照），apply 阶段极短（仅 apply_action） |
| delta 通道溢出导致事件丢失 | 低 | 容量 1024 + snapshot 兜底校验 |
| Godot physics_process 处理大量 delta 卡顿 | 低 | 每帧限制处理 100 个 delta，剩余留给下一帧 |
| SVG 转 PNG 后导入仍然失败 | 低 | 验证 PNG 格式合法性，必要时手动创建最小 PNG 测试 |
| 重构后现有功能回归 | 中 | snapshot 通道保留，一致性校验机制保证最终状态正确 |

## 7. 迁移方案

### 7.1 部署步骤

1. 删除 `.godot/imported/` 缓存
2. 执行 `svg_to_png.py` 重新生成 PNG
3. 修改 `crates/bridge/src/lib.rs`：添加 AgentDelta 类型、双通道、Agent 独立循环
4. 修改 `client/scripts/agent_manager.gd`：支持 delta 增量更新 + snapshot 一致性校验
5. 修改 `SimulationBridge`：增加 delta 接收和信号发射
6. `cargo bridge` 编译 GDExtension
7. Godot 编辑器打开验证

### 7.2 回滚方案

- 保留现有 `snapshot` 通道的完整逻辑不变
- 新增的 delta 通道是增量功能，如果出问题可以暂时禁用 delta，仅靠 snapshot 仍能工作（回退到当前行为）
- Git 回滚：`git checkout` 回退到变更前 commit

## 8. 待定事项

无（以下事项已确认）：
- ~~NPC 数量和 spawn 策略~~ → 数量通过配置参数控制，MVP 阶段 5-10，生产环境 0
- ~~纹理尺寸~~ → 截图验证 16x16 在当前渲染尺度下视觉效果正常，不需要调整
- ~~simulation_bridge.gd 是否保留~~ → 删除，场景中 SimulationBridge 节点使用 Rust GDExtension 类型，与 GDScript 脚本无关
