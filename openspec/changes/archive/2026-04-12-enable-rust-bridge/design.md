# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

Agentora 采用 **Rust 核心引擎 + Godot 4 渲染客户端** 的架构。Rust 侧已实现：
- `crates/core/` — World 模型、Agent 实体、动机系统、决策管道、记忆系统、策略系统、遗产系统
- `crates/ai/` — LLM Provider trait、OpenAI/Anthropic Provider、FallbackChain、RetryProvider、JSON 解析器
- `crates/bridge/` — GDExtension 桥接层，包含 SimulationBridge GDExtension 类、WorldSnapshot 序列化
- `crates/network/` + `crates/sync/` — P2P 网络和 CRDT 同步（本次变更不涉及）

Godot 侧（`client/`）已实现完整的 UI 层：WorldRenderer（地形渲染）、AgentManager（Agent 可视化）、MotivationRadar（6 维雷达图）、NarrativeFeed（叙事流）、GuidePanel（控制面板）、CameraController（摄像机控制）。

**关键约束**：
- Bridge crate 类型为 `cdylib`（动态库），使用 `godot-rust` 框架
- Godot 主线程与模拟线程通过 `std::mpsc` 通道通信
- 模拟线程内嵌 Tokio 运行时
- 决策 prompt 严格控制在 2500 tokens 内，记忆预算 max 1800 chars

### 1.2 现状分析

当前 Godot 客户端通过 **autoload 加载纯 GDScript 模拟版** `res://scripts/simulation_bridge.gd`，而非 Rust GDExtension。该模拟版：
- Agent 行为完全随机（`randf_range(-2, 2)` 移动，随机选择动作字符串）
- 无 LLM 调用、无决策管道、无真实世界状态
- `adjust_motivation()` / `inject_preference()` / `set_tick_interval()` / `toggle_pause()` 等方法仅打印日志

Rust 侧 `crates/bridge/src/lib.rs` 已实现完整架构（GDExtension 类、mpsc 通道、模拟循环、agent_decision 函数），但存在以下问题：
1. **未加载**：Godot autoload 不指向 GDExtension
2. **未注入 LLM**：`DecisionPipeline::new()` 创建的管道 `llm_provider = None`
3. **Cargo.toml 缺少 `agentora-ai` 依赖**：bridge 无法引用 LLM Provider
4. **`apply_action()` 不完整**：Move/Gather/Wait/InteractLegacy 已实现，但 Trade/Talk/Attack/Build/Explore/AllyPropose 等 10+ 种动作全部走 `NotImplemented` 分支
5. **`World::snapshot()` 不完整**：`map_changes`/`events`/`legacies`/`pressures` 均为空向量
6. **bridge API 方法仅 print 占位**：`adjust_motivation()`、`inject_preference()`、`set_tick_interval()`、`get_agent_count()`、`toggle_pause()`

Agent 交互模块状态：
- `trade.rs` — 核心逻辑完整（propose/accept/reject），但 accept 未检查发起方资源
- `dialogue.rs` — 仅有骨架（3 字段结构体 + 空 talk 方法）
- `combat.rs` — 基础逻辑完整（扣血+关系标记），缺少距离检查、伤害计算、死亡处理
- `movement.rs` — 移动完整，perceive_nearby 未实现
- `inventory.rs` — 基础功能完整

### 1.3 关键干系人

- **Rust 核心引擎**：提供 World/Agent/DecisionPipeline 接口
- **AI Crate**：提供 LLM Provider 实现
- **Godot 客户端**：负责渲染和玩家交互
- **本地 LLM 服务**：需要运行的 OpenAI 兼容端点（如 LM Studio + Qwen3.5-2B）

## 2. 设计目标

### 目标

- 将 Godot SimulationBridge 从 GDScript 模拟版切换到 Rust GDExtension 实现
- 在 bridge 中注入 LLM Provider，使 Agent 能通过 LLM 进行真实决策
- 补全 `World::apply_action()` 中所有 ActionType 的执行逻辑
- 补全 dialogue.rs 和 combat.rs 的核心逻辑
- 补全 bridge 的 Godot 可调用 API
- 补全 `World::snapshot()` 填充事件/遗产/压力数据

### 非目标

- P2P 网络集成（network crate）— 后续变更
- CRDT 状态同步（sync crate）— 后续变更
- LocalProvider（mistralrs GGUF 推理）集成 — 当前仍使用占位
- 持久化存储集成 — 后续变更
- Godot 导出打包分发 — 后续变更

## 3. 整体架构

### 3.1 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                       Godot 4 客户端                         │
│                                                             │
│  ┌───────────┐  ┌───────────┐  ┌─────────────┐  ┌────────┐ │
│  │WorldView  │  │AgentMgr   │  │NarrativeFeed│  │Guide   │ │
│  │Renderer   │  │Click/Move │  │Event Log    │  │Panel   │ │
│  └─────┬─────┘  └─────┬─────┘  └──────┬──────┘  └───┬────┘ │
│        │              │               │               │      │
│        └──────────────┴───────┬───────┴───────────────┘      │
│                               │ signals                      │
│  ┌────────────────────────────▼─────────────────────────┐    │
│  │       SimulationBridge (Rust GDExtension 类)          │    │
│  │                                                      │    │
│  │  Godot 主线程 ◄── mpsc ──► 后台模拟线程 (Tokio)      │    │
│  │       │                          │                    │    │
│  │  ┌────▼────┐               ┌─────▼──────┐            │    │
│  │  │Signals  │◄──Snapshot────│ World      │            │    │
│  │  │emit     │               │ snapshot()  │            │    │
│  │  └─────────┘               └────────────┘            │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                             │
│                    后台模拟线程 (详细)                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  tick 循环:                                          │   │
│  │    1. poll SimCommand 通道                            │   │
│  │    2. World::advance_tick()                           │   │
│  │    3. 每个存活 Agent:                                 │   │
│  │       a. 构建 WorldState（地形/资源/附近Agent）        │   │
│  │       b. 构建 Spark（动机缺口）                       │   │
│  │       c. DecisionPipeline::execute()                  │   │
│  │          ├── LLM Provider ──► JSON ──► 校验 ──► 选择  │   │
│  │          └── 失败 ──► RuleEngine fallback             │   │
│  │       d. World::apply_action()                        │   │
│  │    4. World::snapshot() ──► tx.send()                 │   │
│  │    5. tokio::time::sleep(tick_interval)               │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件 | 路径 | 职责 |
|------|------|------|
| `SimulationBridge` | `crates/bridge/src/lib.rs` | GDExtension 节点，启动模拟线程，poll 快照，转发 SimCommand |
| `DecisionPipeline` | `crates/core/src/decision.rs` | 五阶段决策管道：硬约束→Prompt→LLM→校验→选择 |
| `World::apply_action` | `crates/core/src/world/mod.rs` | 执行 Agent 动作，更新世界状态 |
| `World::snapshot` | `crates/core/src/world/mod.rs` + `crates/core/src/snapshot.rs` | 生成世界快照 |
| `agent::dialogue` | `crates/core/src/agent/dialogue.rs` | 对话消息结构和生成 |
| `agent::combat` | `crates/core/src/agent/combat.rs` | 战斗伤害和关系更新 |
| `agent::trade` | `crates/core/src/agent/trade.rs` | 交易提议和执行 |
| `LlmConfig` | `crates/ai/src/config.rs` | 从 config/llm.toml 加载 Provider 配置 |
| `OpenAiProvider` | `crates/ai/src/openai.rs` | OpenAI 兼容 API 调用 |

### 3.3 数据流设计

```
Godot _ready()
  │
  ▼
SimulationBridge::ready()
  │
  ├── start_simulation()
  │     │
  │     ├── 创建 mpsc channels (tx/rx + cmd_tx/cmd_rx)
  │     ├── 加载 LLM 配置 (config/llm.toml)
  │     ├── 创建 LLM Provider (OpenAiProvider)
  │     ├── 创建 DecisionPipeline + 注入 Provider
  │     ├── 创建 World + 初始 Agent
  │     │
  │     └── spawn thread: run_simulation_async()
  │           │
  │           ├── loop:
  │           │     ├── poll cmd_rx (Pause/SetTickInterval/AdjustMotivation/InjectPreference)
  │           │     ├── World::advance_tick()  → 死亡检查、压力tick、遗产衰减
  │           │     ├── for agent in agents:
  │           │     │     ├── build WorldState (vision_radius=5)
  │           │     │     ├── build Spark (from motivation gap)
  │           │     │     ├── DecisionPipeline::execute()
  │           │     │     │     ├── 硬约束过滤 → 合法候选列表
  │           │     │     │     ├── Prompt 构建（动机+Spark+感知+策略）
  │           │     │     │     ├── LLM 调用 → JSON 解析
  │           │     │     │     ├── 规则校验（资源/距离/目标存在）
  │           │     │     │     └── 动机加权选择
  │           │     │     └── World::apply_action()
  │           │     │           ├── Move/Gather/Wait (已有)
  │           │     │           ├── TradeOffer/TradeAccept/TradeReject (新增)
  │           │     │           ├── Talk (新增)
  │           │     │           ├── Attack (增强)
  │           │     │           ├── Build (新增)
  │           │     │           ├── Explore (新增)
  │           │     │           └── AllyPropose/AllyAccept/AllyReject (新增)
  │           │     └── World::snapshot() → tx.send()
  │           │           ├── agents: 存活Agent列表 (已有)
  │           │           ├── events: tick内事件列表 (新增)
  │           │           ├── legacies: 新遗产列表 (新增)
  │           │           ├── pressures: 活跃压力 (新增)
  │           │           └── map_changes: 建筑/资源变更 (新增)
  │           │
  │           └── sleep(tick_interval)
  │
  └── physics_process(delta):
        ├── try_recv snapshot from rx
        └── emit signals:
              ├── world_updated(snapshot)
              ├── narrative_event(event) — per event
              ├── legacy_created(legacy) — per legacy
              └── pressure_update(pressures)
```

## 4. 详细设计

### 4.1 接口设计

#### bridge 侧新增/修改的 `#[func]` 方法

**当前 bridge lib.rs 已有方法（需修改）**：

| 方法签名 | 当前状态 | 修改后行为 |
|---------|---------|-----------|
| `start_simulation()` | 启动模拟但无LLM | 增加 LLM Provider 注入 |
| `get_tick()` | ✅ 已实现 | 不变 |
| `get_agent_count()` | 硬编码返回 5 | 从 snapshot 获取真实值 |
| `toggle_pause()` | 仅切换本地标志 | 发送 SimCommand 到模拟线程 |
| `adjust_motivation(agent_id, dimension, value)` | 仅 print | 发送 SimCommand 到模拟线程 |
| `inject_preference(agent_id, dimension, boost, duration)` | 仅 print | 发送 SimCommand 到模拟线程 |
| `set_tick_interval(seconds)` | 仅 print | 发送 SimCommand 到模拟线程 |

**需要新增的方法**：

| 方法签名 | 说明 |
|---------|------|
| `get_agent_data(agent_id: String) -> Godot<Dictionary>` | 从 snapshot 缓存中获取 Agent 实时数据 |

**Godot Dictionary 返回结构**（`get_agent_data`）：
```
{
  "id": String,
  "name": String,
  "position": Vector2,
  "motivation": Array[6 f32],
  "health": int,
  "max_health": int,
  "age": int,
  "is_alive": bool,
  "current_action": String,
  "inventory": Dictionary<String, int>
}
```

### 4.2 外部接口调用

#### LLM Provider 调用

- **所属系统**：本地 OpenAI 兼容端点（默认 `http://localhost:1234`）
- **请求方式**：POST
- **接口地址**：`/v1/chat/completions`
- **调用位置**：`DecisionPipeline::call_llm()` → `OpenAiProvider::generate()`
- **请求体**：
```json
{
  "model": "qwen3.5-2b",
  "messages": [{"role": "user", "content": "<决策prompt>"}],
  "max_tokens": 500,
  "temperature": 0.7,
  "response_format": {"type": "json_object"}
}
```
- **响应体**：
```json
{
  "choices": [{"message": {"content": "{\"action_type\":\"Move\",\"params\":{...},...}"}}],
  "usage": {"prompt_tokens": N, "completion_tokens": M, "total_tokens": N+M}
}
```

#### 降级链

```
OpenAiProvider (primary)
  ↓ timeout/429/5xx
AnthropicProvider (secondary, 默认 disabled)
  ↓ 全部失败
RuleEngine.fallback_action() (最终兜底)
```

### 4.3 数据模型

#### 事件记录结构（内部，非持久化）

在 `World` 结构体中新增字段 `tick_events: Vec<NarrativeEvent>`，用于记录当前 tick 的事件，在 `snapshot()` 时转至快照。

```rust
// 在 World 结构体中新增
pub tick_events: Vec<NarrativeEvent>,
```

#### 临时偏好结构

新增 `TempPreference` 结构体，挂载到 `Agent` 上：

```rust
pub struct TempPreference {
    pub dimension: usize,
    pub boost: f32,
    pub remaining_ticks: u32,
}

// 在 Agent 结构体中新增
pub temp_preferences: Vec<TempPreference>,
```

#### 待处理交易结构

```rust
pub struct PendingTrade {
    pub offer: TradeOffer,
    pub target_id: AgentId,
    pub created_tick: u32,
    pub status: TradeStatus, // Pending / Accepted / Rejected / Expired
}

pub enum TradeStatus { Pending, Accepted, Rejected, Expired }

// 在 World 结构体中新增
pub pending_trades: Vec<PendingTrade>,
```

#### 对话记录结构

```rust
pub struct DialogueLog {
    pub participants: (AgentId, AgentId),  // 排序保证唯一 key
    pub messages: Vec<DialogueMessage>,
    pub round_count: u32,
    pub created_tick: u32,
    pub is_active: bool,
}

// 在 World 结构体中新增
pub dialogue_logs: Vec<DialogueLog>,
```

### 4.4 核心算法

#### 4.4.1 bridge 注入 LLM Provider

在 `crates/bridge/Cargo.toml` 中添加 `agentora-ai` 依赖：

```toml
agentora-ai = { workspace = true }
```

在 `SimulationBridge::start_simulation()` 中：

```rust
// 加载 LLM 配置
let config = load_llm_config("config/llm.toml").unwrap_or_default();

// 创建 Primary Provider
let primary = agentora_ai::openai::OpenAiProvider::new(
    config.primary.api_base.clone(),
    config.primary.api_key.clone(),
    config.primary.model.clone(),
    config.primary.timeout_seconds,
);

// 构建降级链
let mut chain = agentora_ai::fallback::FallbackChain::new();
chain.add_provider(Box::new(primary));

// 如果 Anthropic 启用，添加为备用
if config.anthropic.enabled && !config.anthropic.api_key.is_empty() {
    let secondary = agentora_ai::anthropic::AnthropicProvider::new(
        config.anthropic.api_key.clone(),
        config.anthropic.model.clone(),
        config.anthropic.timeout_seconds,
    );
    chain.add_provider(Box::new(secondary));
}

// 创建决策管道并注入 Provider
let mut pipeline = DecisionPipeline::with_defaults();
// 注意：DecisionPipeline::with_llm_provider 接受 Box<dyn LlmProvider>
// FallbackChain 实现了 LlmProvider trait，所以可以直接注入
pipeline = pipeline.with_llm_provider(Box::new(chain));
```

#### 4.4.2 `apply_action` — TradeOffer

```rust
ActionType::TradeOffer { offer, want } => {
    // 查找同格的目标 Agent（简化：选择最近的 Agent）
    let agent = self.agents.get(agent_id).unwrap();
    let target = self.agents.values()
        .find(|a| a.is_alive && a.position == agent.position && a.id != *agent_id);

    match target {
        Some(target) => {
            // 检查发起方是否有 offer 中的资源
            for (resource, amount) in offer {
                let key = resource.as_str();
                let current = agent.inventory.get(key).copied().unwrap_or(0);
                if current < *amount {
                    return ActionResult::InsufficientResources; // 需新增
                }
            }

            // 创建待处理交易
            let trade = TradeOffer {
                proposer_id: agent.id.clone(),
                offer: offer.clone(),
                want: want.clone(),
                trade_id: uuid::Uuid::new_v4().to_string(),
            };
            self.pending_trades.push(PendingTrade {
                offer: trade.clone(),
                target_id: target.id.clone(),
                created_tick: self.tick as u32,
                status: TradeStatus::Pending,
            });

            // 记录事件
            self.record_event(agent_id, "trade",
                &format!("{} 向 {} 提议交易", agent.name, target.name));
            ActionResult::Success
        }
        None => ActionResult::NoTarget,
    }
}
```

#### 4.4.3 `apply_action` — TradeAccept

```rust
ActionType::TradeAccept { trade_id } => {
    // 查找待处理交易
    let pending = self.pending_trades.iter_mut()
        .find(|t| t.offer.trade_id == *trade_id && t.status == TradeStatus::Pending);

    match pending {
        Some(pending) => {
            // 检查目标 Agent（当前 agent_id）是否有 want 中的资源
            let target_agent = self.agents.get(agent_id).unwrap();
            for (resource, amount) in &pending.offer.want {
                let key = resource.as_str();
                let current = target_agent.inventory.get(key).copied().unwrap_or(0);
                if current < *amount {
                    pending.status = TradeStatus::Rejected;
                    return ActionResult::InsufficientResources;
                }
            }

            // 检查发起方是否有 offer 中的资源
            let proposer = self.agents.get(&pending.offer.proposer_id).unwrap();
            for (resource, amount) in &pending.offer.offer {
                let key = resource.as_str();
                let current = proposer.inventory.get(key).copied().unwrap_or(0);
                if current < *amount {
                    pending.status = TradeStatus::Rejected;
                    // 标记欺诈
                    self.record_event(agent_id, "trade_fail",
                        &format!("交易欺诈：{} 资源不足", proposer.name));
                    return ActionResult::InsufficientResources;
                }
            }

            // 执行资源交换
            // 发起方：给出 offer，获得 want
            let proposer = self.agents.get_mut(&pending.offer.proposer_id).unwrap();
            for (resource, amount) in &pending.offer.offer {
                proposer.consume(*resource, *amount);
            }
            for (resource, amount) in &pending.offer.want {
                proposer.gather(*resource, *amount);
            }

            // 接收方：给出 want，获得 offer
            let receiver = self.agents.get_mut(agent_id).unwrap();
            for (resource, amount) in &pending.offer.want {
                receiver.consume(*resource, *amount);
            }
            for (resource, amount) in &pending.offer.offer {
                receiver.gather(*resource, *amount);
            }

            // 增加信任
            // ... 关系更新逻辑

            pending.status = TradeStatus::Accepted;
            self.record_event(agent_id, "trade", "交易成功");
            ActionResult::Success
        }
        None => ActionResult::NotFound,
    }
}
```

#### 4.4.4 `apply_action` — Talk

```rust
ActionType::Talk { message } => {
    let agent = self.agents.get(agent_id).unwrap();
    let target = self.agents.values()
        .find(|a| a.is_alive && a.position == agent.position && a.id != *agent_id);

    match target {
        Some(target) => {
            let dialogue_msg = DialogueMessage {
                speaker_id: agent.id.clone(),
                content: message.clone(),
                tick: self.tick as u32,
            };

            // 查找或创建对话记录
            let log = self.dialogue_logs.iter_mut()
                .find(|l| {
                    let (a, b) = (&l.participants.0, &l.participants.1);
                    (a == &agent.id && b == &target.id) ||
                    (a == &target.id && b == &agent.id)
                });

            if let Some(log) = log {
                if log.is_active && log.round_count < 3 {
                    log.messages.push(dialogue_msg);
                    log.round_count += 1;
                } else {
                    log.is_active = false;
                }
            } else {
                self.dialogue_logs.push(DialogueLog {
                    participants: {
                        let mut ids = [agent.id.clone(), target.id.clone()];
                        ids.sort();
                        (ids[0].clone(), ids[1].clone())
                    },
                    messages: vec![dialogue_msg],
                    round_count: 1,
                    created_tick: self.tick as u32,
                    is_active: true,
                });
            }

            self.record_event(agent_id, "talk",
                &format!("{}: {}", agent.name, message));
            ActionResult::Success
        }
        None => ActionResult::NoTarget,
    }
}
```

#### 4.4.5 `apply_action` — Attack（增强版）

```rust
ActionType::Attack { target_id } => {
    let agent = self.agents.get(agent_id).unwrap();
    let target = self.agents.get(target_id);

    match target {
        Some(target) if target.is_alive && target.position == agent.position => {
            // 距离检查：必须在同格
            // 伤害计算：base(10~30) * (1.0 + power_motivation * 0.5)
            let power_motivation = agent.motivation[4]; // 权力维度
            let base_damage = rand::thread_rng().gen_range(10..=30);
            let damage = (base_damage as f32 * (1.0 + power_motivation * 0.5)) as u32;

            // 执行攻击
            let target_ref = self.agents.get_mut(&target.id).unwrap();
            let result = agent.clone().attack(target_ref, damage);

            // 掠夺资源（简化：随机 1~3 个）
            let loot_count = rand::thread_rng().gen_range(1..=3);
            // ... 资源转移逻辑

            // 记录事件
            let event_desc = if result.target_alive {
                format!("{} 攻击 {}，造成 {} 伤害", agent.name, target.name, damage)
            } else {
                format!("{} 攻击 {} 并击杀了对方", agent.name, target.name)
            };
            self.record_event(agent_id, "attack", &event_desc);

            if !result.target_alive {
                // check_agent_death 已在 advance_tick 中处理
            }

            ActionResult::Success
        }
        _ => ActionResult::NoTarget, // 目标不存在、已死亡或不在同格
    }
}
```

#### 4.4.6 `apply_action` — Build

```rust
ActionType::Build { structure } => {
    let agent = self.agents.get(agent_id).unwrap();
    let pos = agent.position;

    // 检查资源需求（简化配置）
    let required = match structure {
        StructureType::Camp => [("wood", 10), ("stone", 5)],
        StructureType::Fence => [("wood", 5)],
        StructureType::Warehouse => [("stone", 15), ("iron", 5)],
    };

    for (resource, amount) in required {
        let current = agent.inventory.get(resource).copied().unwrap_or(0);
        if current < amount {
            return ActionResult::InsufficientResources;
        }
    }

    // 扣除资源
    let agent = self.agents.get_mut(agent_id).unwrap();
    for (resource, amount) in required {
        agent.consume(ResourceType::from_str(resource).unwrap(), amount);
    }

    // 放置建筑
    let structure = structure::Structure::new(
        pos,
        structure.clone(),
        agent.id.clone(),
        self.tick,
    );
    self.structures.insert(pos, structure);

    self.record_event(agent_id, "build",
        &format!("{} 建造了 {:?}", agent.name, structure));
    ActionResult::Success
}
```

#### 4.4.7 `apply_action` — Explore

```rust
ActionType::Explore { target_region } => {
    let agent = self.agents.get_mut(agent_id).unwrap();

    // 向随机可通行方向移动
    let directions = [Direction::North, Direction::South, Direction::East, Direction::West];
    let mut moved = false;

    for dir in directions.iter().cycle().take(4) {
        let (dx, dy) = dir.delta();
        let new_x = agent.position.x as i32 + dx;
        let new_y = agent.position.y as i32 + dy;
        if new_x >= 0 && new_y >= 0 {
            let new_pos = Position::new(new_x as u32, new_y as u32);
            if self.map.is_valid(new_pos) && self.map.get_terrain(new_pos).is_passable() {
                agent.position = new_pos;
                moved = true;
                break;
            }
        }
    }

    // 感知新位置的资源/Agent
    let perception = agent.perceive_nearby(&self.agents, &self.resources, 5);

    if moved {
        self.record_event(agent_id, "explore",
            &format!("{} 探索新区域，发现 {} 个资源点", agent.name, perception.resources.len()));
    }

    ActionResult::Success
}
```

#### 4.4.8 `World::snapshot()` 补全

```rust
pub fn snapshot(&self) -> WorldSnapshot {
    // ... agents 部分不变 ...

    // 填充 events
    let events = self.tick_events.clone();

    // 填充 legacies — 本 tick 新产生的遗产
    let legacies = self.legacies.iter()
        .filter(|l| l.created_tick == self.tick)
        .map(|l| LegacyEvent {
            id: l.id.clone(),
            position: (l.position.x, l.position.y),
            legacy_type: "death".to_string(),
            original_agent_name: l.agent_name.clone(),
        })
        .collect();

    // 填充 pressures
    let pressures = self.pressure_pool.iter()
        .map(|p| PressureSnapshot {
            id: p.id.clone(),
            pressure_type: p.pressure_type.clone(),
            description: p.description.clone(),
            remaining_ticks: p.remaining_ticks,
        })
        .collect();

    // 填充 map_changes — 建筑和结构变更
    let map_changes: Vec<CellChange> = self.structures.values()
        .map(|s| CellChange {
            x: s.position.x,
            y: s.position.y,
            terrain: "structure".to_string(),
            structure: Some(format!("{:?}", s.structure_type)),
            resource_type: None,
            resource_amount: None,
        })
        .collect();

    WorldSnapshot {
        tick: self.tick,
        agents,
        map_changes,
        events,
        legacies,
        pressures,
    }
}
```

需要在 `World` 中新增事件记录辅助方法：

```rust
impl World {
    fn record_event(&mut self, agent_id: &AgentId, event_type: &str, description: &str) {
        if let Some(agent) = self.agents.get(agent_id) {
            self.tick_events.push(NarrativeEvent {
                tick: self.tick,
                agent_id: agent_id.as_str().to_string(),
                agent_name: agent.name.clone(),
                event_type: event_type.to_string(),
                description: description.to_string(),
                color_code: match event_type {
                    "trade" => "#4CAF50",
                    "talk" => "#9E9E9E",
                    "attack" => "#F44336",
                    "build" => "#FF9800",
                    "explore" => "#FFFFFF",
                    "death" => "#9C27B0",
                    "trade_fail" => "#F44336",
                    _ => "#FFFFFF",
                }.to_string(),
            });
        }
    }
}
```

#### 4.4.9 临时偏好系统

在 `Agent` 上新增方法：

```rust
impl Agent {
    /// 注入临时偏好
    pub fn inject_preference(&mut self, dimension: usize, boost: f32, duration_ticks: u32) {
        self.temp_preferences.push(TempPreference {
            dimension,
            boost,
            remaining_ticks: duration_ticks,
        });
    }

    /// tick 更新临时偏好
    pub fn tick_preferences(&mut self) {
        // 减少剩余 tick，移除到期的偏好
        self.temp_preferences.retain_mut(|p| {
            p.remaining_ticks = p.remaining_ticks.saturating_sub(1);
            p.remaining_ticks > 0
        });
    }

    /// 获取有效动机（基础 + 临时偏好）
    pub fn effective_motivation(&self) -> [f32; 6] {
        let mut eff = self.motivation.to_array();
        for pref in &self.temp_preferences {
            if pref.dimension < 6 {
                eff[pref.dimension] = (eff[pref.dimension] + pref.boost).clamp(0.0, 1.0);
            }
        }
        eff
    }
}
```

在 `World::advance_tick()` 中调用 `tick_preferences()`：

```rust
pub fn advance_tick(&mut self) {
    self.tick += 1;
    self.pressure_tick();
    self.check_agent_death();
    self.decay_legacies();

    // 更新 Agent 临时偏好
    for agent in self.agents.values_mut() {
        agent.tick_preferences();
    }

    // ... 策略衰减 ...
}
```

#### 4.4.10 dialogue 补全

```rust
impl Agent {
    pub fn talk(&self, message: &str, world_tick: u32) -> DialogueMessage {
        DialogueMessage {
            speaker_id: self.id.clone(),
            content: message.to_string(),
            tick: world_tick,
        }
    }
}
```

对话内容的 AI 生成不在 Agent 层做，而是在 bridge 的 `agent_decision` 中由 LLM 通过 Prompt 生成（Talk 动作的 `message` 参数）。LLM 不可用时使用模板兜底：

```rust
fn generate_dialogue_fallback(agent: &Agent, target: &Agent) -> String {
    let max_dim = agent.motivation.max_dimension_index();
    match max_dim {
        0 => "我需要更多资源来生存。",
        1 => "你好，愿意合作吗？",
        2 => "我发现了一些有趣的事情。",
        3 => "我想建造一些东西。",
        4 => "这里应该是我的领地。",
        5 => "我会留下我的遗产。",
        _ => "...",
    }.to_string()
}
```

#### 4.4.11 combat 增强

当前 `Agent::attack()` 已实现扣血和关系标记，但缺少距离检查和动态伤害。修改如下：

```rust
// 在 World::apply_action 中做距离检查（见 4.4.5）
// 伤害计算在 apply_action 中完成（见 4.4.5 的 damage 公式）
// Agent::attack() 保持当前签名，仅做扣血和关系更新
```

死亡处理已在 `World::check_agent_death()` 中实现（创建 Legacy + 标记 is_alive = false），但需要补充资源散落逻辑：

```rust
fn check_agent_death(&mut self) {
    let dead_agent_ids: Vec<AgentId> = self.agents
        .iter()
        .filter(|(_, agent)| agent.is_alive && (agent.age >= agent.max_age || agent.health == 0))
        .map(|(id, _)| id.clone())
        .collect();

    for agent_id in dead_agent_ids {
        let agent = self.agents.get(&agent_id).unwrap();
        if !agent.is_alive { continue; }

        // 散落背包资源到当前位置
        for (resource, amount) in &agent.inventory {
            // 在当前位置创建资源节点（或累加已有）
            let pos = agent.position;
            // ... 资源创建逻辑
        }

        // 创建遗产
        let legacy = Legacy::from_agent(agent, self.tick);
        self.legacies.push(legacy);

        // 标记死亡
        let agent = self.agents.get_mut(&agent_id).unwrap();
        agent.is_alive = false;

        // 记录事件
        self.record_event(&agent_id, "death",
            &format!("{} 已死亡，留下遗产", agent.name));
    }
}
```

#### 4.4.12 movement perceive_nearby 补全

```rust
impl Agent {
    pub fn perceive_nearby(
        &self,
        all_agents: &HashMap<AgentId, Agent>,
        all_resources: &HashMap<Position, ResourceNode>,
        vision_radius: u32,
    ) -> PerceptionResult {
        let mut nearby_agents = Vec::new();
        let mut nearby_resources = Vec::new();

        for (id, agent) in all_agents {
            if id == &self.id || !agent.is_alive { continue; }
            let dist = self.position.distance_to(agent.position);
            if dist <= vision_radius {
                nearby_agents.push(PerceivedAgent {
                    id: id.clone(),
                    name: agent.name.clone(),
                    position: agent.position,
                });
            }
        }

        for (pos, resource) in all_resources {
            let dist = self.position.distance_to(*pos);
            if dist <= vision_radius {
                nearby_resources.push(PerceivedResource {
                    position: *pos,
                    resource_type: resource.resource_type,
                    amount: resource.amount,
                });
            }
        }

        PerceptionResult {
            nearby_agents,
            nearby_resources,
        }
    }
}
```

### 4.5 异常处理

| 异常场景 | 处理策略 |
|---------|---------|
| LLM 服务不可用 | FallbackChain 自动降级 → 规则引擎兜底，Agent 继续运行 |
| JSON 解析失败 | parse_action_json 三层递进解析 → 最终降级规则引擎 |
| Agent 死亡后被选中 | GuidePanel/MotivationRadar 检测 `is_alive=false` 后显示死亡状态 |
| 目标 Agent 不在同格 | Trade/Talk/Attack 返回 `ActionResult::NoTarget`，不消耗资源 |
| 资源不足 | Trade/Build 返回 `ActionResult::InsufficientResources` |
| GDExtension DLL 加载失败 | Godot 回退至 GDScript 模拟版（运行时检测） |
| 命令通道满 | mpsc 默认无界，不存在满的情况；模拟线程退出后 tx.send 返回 Err |
| 快照消费延迟 | physics_process 每帧 try_recv 非阻塞，多帧快照仅消费最后一个 |

### 4.6 前端设计

#### Godot 侧变更

| 文件 | 变更说明 |
|------|---------|
| `project.godot` | autoload 从 `res://scripts/simulation_bridge.gd` 改为 GDExtension 注册的类型（保持 SimulationBridge 名称不变） |
| `scripts/simulation_bridge.gd` | 保留为 **回退方案**，不删除。但不再作为 autoload 加载。保留所有信号定义和公共 API 签名不变 |
| `scripts/main.gd` | 不需要修改，信号接口不变 |
| `scripts/agent_manager.gd` | 不需要修改 |
| `scripts/narrative_feed.gd` | 不需要修改 |
| `scripts/motivation_radar.gd` | 不需要修改 |
| `scripts/guide_panel.gd` | 不需要修改 |
| `scripts/camera_controller.gd` | 不需要修改 |
| `client/agentora_bridge.gdextension` | 不需要修改，已正确指向 `bin/agentora_bridge.dll` |

**关键点**：Godot 侧的 GDScript 代码**不需要修改**，因为：
1. GDExtension 的 `SimulationBridge` 类注册后，与 GDScript 版同名
2. 信号接口（`world_updated`、`agent_selected`、`narrative_event`）保持一致
3. 公共 API（`adjust_motivation`、`inject_preference` 等）签名一致

## 5. 技术决策

### 决策1：LLM Provider 注入方式

- **选型方案**：在 bridge 的 `run_simulation_async` 中创建 `LlmConfig` → 构建 `FallbackChain` → 注入 `DecisionPipeline`
- **选择理由**：
  - `DecisionPipeline` 已有 `with_llm_provider()` builder 方法
  - `FallbackChain` 实现了 `LlmProvider` trait，可直接注入
  - 配置从 `config/llm.toml` 加载，已有 `load_llm_config()` 函数
- **备选方案**：通过 GDExtension 属性传入 Provider 配置
- **放弃原因**：配置在 Rust 侧管理更简洁，GDExtension 属性序列化复杂且没必要

### 决策2：事件数据传递方式

- **选型方案**：在 `World` 结构体中新增 `tick_events: Vec<NarrativeEvent>` 字段，`snapshot()` 时填充
- **选择理由**：`WorldSnapshot` 已有 `events: Vec<NarrativeEvent>` 字段，直接填充即可。不新增通道、不修改信号签名
- **备选方案**：通过独立的 mpsc 通道发送事件
- **放弃原因**：增加复杂度但无实质收益，快照已包含事件，复用同一通道即可

### 决策3：Godot autoload 配置

- **选型方案**：修改 `project.godot`，autoload 路径从 GDScript 切换为 GDExtension 注册的 SimulationBridge 类型
- **选择理由**：GDExtension 注册的类型在 Godot 场景树中可直接使用，无需 autoload 路径
- **备选方案**：保留 GDScript autoload + 在其内部调用 GDExtension 方法
- **放弃原因**：增加一层不必要的间接性，信号路由复杂

### 决策4：待处理交易的存储位置

- **选型方案**：在 `World` 结构体中新增 `pending_trades: Vec<PendingTrade>` 字段
- **选择理由**：交易是世界级状态（涉及两个 Agent），放在 World 比放在单个 Agent 上更合理
- **备选方案**：在发起方 Agent 上存储 pending trade
- **放弃原因**：接收方需要查询和响应，放在 World 上方便统一管理

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
|--------|---------|---------|
| LLM 服务未启动导致 Agent 决策降级到规则引擎 | 中 | 规则引擎兜底保证系统可用，日志明确提示 LLM 不可用 |
| bridge 新增 `agentora-ai` 依赖引入编译错误 | 低 | agentora-ai 和 agentora-core 已有成熟依赖关系，Cargo workspace 管理 |
| `apply_action` 中新增逻辑引入 borrow checker 冲突 | 中 | 谨慎管理 Rust borrow，必要时使用 `RefCell` 或分阶段操作 |
| GDExtension DLL 版本不兼容（Godot 4.6 兼容性） | 低 | `agentora_bridge.gdextension` 已配置 `compatibility_minimum = "4.6"` |
| Tick 间隔过小导致 LLM 调用频率过高触发限流 | 中 | 默认 tick_interval = 5s，RetryProvider 已实现 429 限流处理 |
| Godot 信号处理时序问题（@onready 节点未就绪） | 低 | CLAUDE.md 已记录经验：使用 `get_node_or_null()` 延迟获取 |

## 7. 迁移方案

### 7.1 部署步骤

1. 在 `crates/bridge/Cargo.toml` 中添加 `agentora-ai` 依赖
2. 修改 `crates/bridge/src/lib.rs`：注入 LLM Provider、实现 bridge API 方法
3. 修改 `crates/core/src/world/mod.rs`：补全 `apply_action()` 和 `snapshot()`
4. 修改 `crates/core/src/agent/dialogue.rs`：补全 tick 参数
5. 修改 `crates/core/src/agent/combat.rs`：增强为 distance-checked 版本
6. 在 `crates/core/src/agent/mod.rs` 中新增 Agent 结构体字段（temp_preferences 等）
7. 修改 `client/project.godot`：切换 autoload 配置
8. `cargo build -p agentora-bridge` 编译 DLL
9. 复制 DLL 到 `client/bin/agentora_bridge.dll`
10. 启动 Godot 测试

### 7.2 灰度策略

本地开发环境测试通过后直接使用。无生产环境灰度需求。

### 7.3 回滚方案

若 GDExtension 版本导致 Godot 崩溃：
1. 将 `project.godot` autoload 改回 `res://scripts/simulation_bridge.gd`
2. 删除 `client/bin/agentora_bridge.dll`
3. Godot 将使用 GDScript 模拟版继续运行

## 8. 待定事项

- [ ] LLM 服务启动方式：用户需手动启动 LM Studio 或其他 OpenAI 兼容服务，还是集成自启动脚本？
- [ ] 多 Agent 同格 Trade 时目标选择策略：当多个 Agent 在同格时，TradeOffer 如何选择目标？（当前设计：选最近的/第一个）
- [ ] 战斗死亡后的资源散落规则：散落比例、资源类型衰减、拾取权限？（当前设计：全部散落在死亡位置）
- [ ] 压力事件的具体类型和生成逻辑：`pressure_tick()` 当前仅有 TODO，需要定义压力事件的具体实现
