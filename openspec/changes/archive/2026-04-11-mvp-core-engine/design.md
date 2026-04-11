# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

Agentora（智纪）定位为端侧多模态大模型AI智能体驱动的去中心化数字文明模拟器。项目当前仅有方案设计文档（Agentora方案初稿.md），无任何代码实现。MVP需要从零构建完整的核心引擎和可分发客户端。

核心约束：
- 全栈Rust语言，Cargo workspace组织
- Godot 4.6 + godot-rust v0.5 (GDExtension) 作为客户端渲染
- Tokio异步运行时
- 无中心服务器，P2P架构
- Tick-Based脉冲决策，5-10秒/周期
- 端侧LLM推理 + API双模式

### 1.2 现状分析

全新项目，无遗留代码。方案设计文档定义了四层世界模型、Spark→Act→Echo→Legacy循环、6维动机引擎、双轨经济等概念，MVP需实现其中最核心的子集。

### 1.3 关键干系人

- 开发者：使用AI辅助开发，不需要考虑Rust学习曲线
- 早期用户：桌面端（Win/Mac/Linux），需要双击即可运行的体验
- Agent：AI驱动的自主实体，通过LLM做出决策
- 玩家：引导者角色，调整Agent动机权重观察世界演进

## 2. 设计目标

### 目标

- 验证AI Agent自主决策的涌现性（合作/冲突/演进）
- 跑通完整的Spark→Act→Echo→Legacy循环
- 实现多节点P2P联机与CRDT状态同步
- 交付可桌面分发的2D世界观察+引导客户端
- 为3D/移动端/经济系统预留可演进架构

### 非目标

- 3D渲染（Phase 2）
- 移动端Android/iOS（Phase 3）
- 双轨经济系统（尘/星货币）
- DAO治理与正典化
- 多模态记忆（图像/语音）
- LoreGraph叙事知识图谱完整版
- L2区块链锚定

## 3. 整体架构

### 3.1 架构概览

```
┌──────────────────────────────────────────────────────────────┐
│                    Agentora Node                              │
│                                                               │
│  ┌─── Godot Main Thread ──────────────────────────────────┐ │
│  │  SimulationBridge (GDExtension)                         │ │
│  │  ├── WorldView (TileMapLayer 2D)                        │ │
│  │  ├── AgentSprites (Sprite2D + Label 动态)              │ │
│  │  ├── DetailPanel (动机雷达图 + 引导滑块)               │ │
│  │  └── NarrativeFeed (RichTextLabel 叙事流)              │ │
│  └─────────────────┬───────────────────────────────────────┘ │
│                    │ mpsc channel                             │
│                    │ WorldSnapshot ↔ SimCommand               │
│  ┌─── Tokio Runtime Thread ───────────────────────────────┐ │
│  │  TickLoop                                               │ │
│  │  ├── core  (动机引擎 + 决策管道 + 世界模型 + 记忆)    │ │
│  │  ├── ai    (LLM Provider: API / 本地GGUF)              │ │
│  │  ├── network (rust-libp2p GossipSub + KAD + Relay)    │ │
│  │  └── sync  (CRDT: LWW + G-Counter + OR-Set)           │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  ┌─── Storage ─────────────────────────────────────────────┐ │
│  │  rusqlite (Agent状态/记忆/事件日志/世界快照)          │ │
│  │  本地文件 (WorldSeed.toml / GGUF模型 / 配置)          │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件 | Crate | 职责 |
| --- | --- | --- |
| motivation-engine | core | 6维动机向量管理、惯性衰减、事件驱动微调、Spark缺口计算 |
| decision-pipeline | core | 五阶段决策管道：硬约束→上下文→LLM→校验→选择 |
| world-model | core | 256×256网格地图、地形、区域、资源节点、环境压力 |
| agent-interaction | core | 移动/采集/交易/对话/攻击/建造/结盟/遗产交互逻辑 |
| memory-system | core | 三层记忆架构：ChronicleStore(编年史冻结快照) + ChronicleDB(SQLite+FTS5) + StrategyHub(策略库)，遗忘衰减 |
| strategy-system | core | 决策策略库、自我改进闭环(create/patch/decay)、progressive disclosure分级披露 |
| rule-engine | core | 硬约束过滤、动作合法性校验、规则引擎兜底决策 |
| llm-bridge | ai | 统一LlmProvider trait、OpenAI/Anthropic/本地GGUF三种后端、JSON兼容解析 |
| p2p-network | network | rust-libp2p集成、GossipSub广播、KAD DHT发现、Circuit Relay穿透 |
| crdt-sync | sync | LWW-Register、G-Counter、OR-Set、操作签名验证、Merkle校验 |
| sim-bridge | bridge | Tokio运行时管理、mpsc Channel桥接、WorldSnapshot序列化、GDExtension节点 |
| godot-client | - | Godot 4项目、场景树、UI面板、TileMap渲染 |

### 3.3 数据流设计

```
Tick循环数据流:

1. PERCEIVE
   WorldSnapshot(CRDT) → Agent.sense_nearby() → Perception
                                              ↓
2. SPARK
   MotivationVector + WorldSatisfaction → gap_calculation → Spark
                                                          ↓
3. LOAD MEMORY (三层架构)
   ┌─ ChronicleStore: 冻结快照注入 (<chronicle-context> 围栏)
   │  ~/.agentora/agents/<id>/CHRONICLE.md + WORLD_SEED.md
   ├─ ChronicleDB: FTS5 检索相关历史 (emotion_tag/event_type)
   │  ~/.agentora/agents/<id>/chronicle.db
   └─ StrategyHub: 匹配Spark的策略 (progressive disclosure)
      ~/.agentora/agents/<id>/strategies/<spark_type>/STRATEGY.md
                                                          ↓
4. BUILD PROMPT
   Spark + Perception + Memory(≤1800 chars) + Strategy + Social.recent() → Prompt (<2.5K tokens)
                                                                ↓
5. LLM GENERATE
   Prompt → LlmProvider.generate() → raw_text → JSON parse → Vec<ActionCandidate>
                                                                    ↓
6. VALIDATE & SELECT
   ActionCandidate[] → RuleEngine.validate() → valid[] → motivation_weighted_select(+策略对齐boost) → Action
                                                                                          ↓
7. EXECUTE
   Action → World.apply() → Effects → CRDT.generate_ops() → GossipSub.broadcast()
                     ↓
8. ECHO
   Effects → Agent.update_memory() + Agent.evolve_motivation() + StrategyHub.update(success/fail) → 新状态
                     ↓
9. SNAPSHOT
   World + Agents → WorldSnapshot → mpsc::tx.send() → Godot渲染
```

P2P同步数据流:

```
本地操作: Action → CRDT Op → 签名 → GossipSub publish
远端操作: GossipSub receive → 验签 → CRDT merge → 更新WorldState
```

## 4. 详细设计

### 4.1 接口设计

#### LlmProvider trait

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
}

pub struct LlmRequest {
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub response_format: ResponseFormat,
    pub stop_sequences: Vec<String>,
}

pub enum ResponseFormat {
    Text,
    Json { schema: Option<String> },
}

pub struct LlmResponse {
    pub raw_text: String,
    pub parsed_action: Option<Action>,
    pub usage: TokenUsage,
    pub provider_name: String,
}
```

#### Action结构化JSON Schema

```rust
pub struct Action {
    pub reasoning: String,
    pub action_type: ActionType,
    pub target: Option<String>,
    pub params: ActionParams,
    pub motivation_delta: [f32; 6],
}

pub enum ActionType {
    Move { direction: Direction },
    Gather { resource: ResourceType },
    TradeOffer { offer: HashMap<ResourceType, u32>, want: HashMap<ResourceType, u32> },
    TradeAccept { trade_id: String },
    TradeReject { trade_id: String },
    Talk { message: String },
    Attack,
    Build { structure: StructureType },
    AllyPropose,
    AllyAccept { ally_id: String },
    Explore { target_region: String },
    Wait,
    InteractLegacy { action: LegacyInteraction },
}
```

#### WorldSnapshot (Sim→Godot)

```rust
pub struct WorldSnapshot {
    pub tick: u64,
    pub agents: Vec<AgentSnapshot>,
    pub map_changes: Vec<CellChange>,
    pub events: Vec<NarrativeEvent>,
    pub legacies: Vec<LegacyEvent>,
    pub pressures: Vec<PressureEvent>,
}

pub struct AgentSnapshot {
    pub id: String,
    pub name: String,
    pub position: (u32, u32),
    pub motivation: [f32; 6],
    pub health: u32,
    pub max_health: u32,
    pub inventory_summary: HashMap<String, u32>,
    pub current_action: String,
    pub age: u32,
    pub is_alive: bool,
}

pub enum SimCommand {
    AdjustMotivation { agent_id: String, dimension: usize, value: f32 },
    InjectPreference { agent_id: String, dimension: usize, boost: f32, duration_ticks: u32 },
    Pause,
    Resume,
    SetTickInterval { seconds: f32 },
}
```

#### CRDT操作

```rust
pub enum CrdtOp {
    LwwSet { key: String, value: Vec<u8>, timestamp: u64, peer_id: PeerId },
    GCounterInc { key: String, amount: u64, peer_id: PeerId },
    OrSetAdd { key: String, element: Vec<u8>, tag: (PeerId, u64) },
    OrSetRemove { key: String, tag: (PeerId, u64) },
}
```

### 4.2 数据模型

#### SQLite 表结构

```sql
-- Agent状态
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    position_x INTEGER NOT NULL,
    position_y INTEGER NOT NULL,
    motivation_vector BLOB NOT NULL,  -- 序列化 [f32; 6]
    health INTEGER NOT NULL DEFAULT 100,
    max_health INTEGER NOT NULL DEFAULT 100,
    age INTEGER NOT NULL DEFAULT 0,
    personality_seed BLOB,  -- 序列化 PersonalitySeed
    is_alive BOOLEAN NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL
);

-- Agent背包
CREATE TABLE inventory (
    agent_id TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (agent_id, resource_type)
);

-- 记忆片段 (FTS5 全文索引)
CREATE TABLE memory_fragments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tick INTEGER NOT NULL,
    text_summary TEXT NOT NULL,
    emotion_tag TEXT NOT NULL,      -- JSON array: ["suspicious", "curious"]
    event_type TEXT NOT NULL,       -- trade/explore/attack/discover_legacy
    importance REAL NOT NULL DEFAULT 0.5,
    compression_level TEXT NOT NULL DEFAULT 'none',
    created_at INTEGER NOT NULL
);

-- FTS5 全文索引虚拟表
CREATE VIRTUAL TABLE memory_fts USING fts5(
    text_summary,
    emotion_tag,
    event_type,
    content='memory_fragments',
    content_rowid=id
);

-- FTS5 同步触发器
CREATE TRIGGER memory_fts_insert AFTER INSERT ON memory_fragments BEGIN
    INSERT INTO memory_fts(rowid, text_summary, emotion_tag, event_type)
    VALUES (new.id, new.text_summary, new.emotion_tag, new.event_type);
END;

CREATE TRIGGER memory_fts_delete AFTER DELETE ON memory_fragments BEGIN
    INSERT INTO memory_fts(memory_fts, rowid, text_summary, emotion_tag, event_type)
    VALUES('delete', old.id, old.text_summary, old.emotion_tag, old.event_type);
END;

CREATE TRIGGER memory_fts_update AFTER UPDATE ON memory_fragments BEGIN
    INSERT INTO memory_fts(memory_fts, rowid, text_summary, emotion_tag, event_type)
    VALUES('delete', old.id, old.text_summary, old.emotion_tag, old.event_type);
    INSERT INTO memory_fts(rowid, text_summary, emotion_tag, event_type)
    VALUES (new.id, new.text_summary, new.emotion_tag, new.event_type);
END;

-- 策略库索引
CREATE TABLE strategies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spark_type TEXT NOT NULL,
    success_rate REAL NOT NULL DEFAULT 1.0,
    use_count INTEGER NOT NULL DEFAULT 0,
    last_used_tick INTEGER NOT NULL,
    deprecated BOOLEAN NOT NULL DEFAULT 0,
    created_tick INTEGER NOT NULL,
    UNIQUE(spark_type)
);

-- 策略执行日志
CREATE TABLE strategy_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy_id INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    action_type TEXT NOT NULL,
    result TEXT NOT NULL,           -- success/fail/patch
    motivation_delta BLOB,          -- 序列化 [f32; 6]
    FOREIGN KEY (strategy_id) REFERENCES strategies(id)
);

-- 事件日志 (OR-Set)
CREATE TABLE event_log (
    id TEXT PRIMARY KEY,
    tick INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    actor_id TEXT,
    data TEXT NOT NULL,  -- JSON
    peer_id TEXT NOT NULL,
    tag_counter INTEGER NOT NULL,
    is_removed BOOLEAN NOT NULL DEFAULT 0
);

-- 世界地图单元格
CREATE TABLE map_cells (
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    terrain TEXT NOT NULL DEFAULT 'plains',
    structure_type TEXT,
    structure_owner TEXT,
    resource_type TEXT,
    resource_current INTEGER DEFAULT 0,
    resource_max INTEGER DEFAULT 0,
    region_id INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (x, y)
);

-- 遗迹
CREATE TABLE legacies (
    id TEXT PRIMARY KEY,
    position_x INTEGER NOT NULL,
    position_y INTEGER NOT NULL,
    legacy_type TEXT NOT NULL,
    original_agent_id TEXT NOT NULL,
    items TEXT NOT NULL,  -- JSON
    echo_log TEXT,  -- 回响日志
    created_tick INTEGER NOT NULL
);

-- 社会关系
CREATE TABLE relations (
    agent_id TEXT NOT NULL,
    other_id TEXT NOT NULL,
    trust REAL NOT NULL DEFAULT 0.5,
    relation_type TEXT NOT NULL DEFAULT 'neutral',  -- neutral/ally/enemy
    last_interaction_tick INTEGER,
    PRIMARY KEY (agent_id, other_id)
);
```

### 4.3 核心算法

#### Tick循环

```rust
async fn tick_loop(world: &mut World, llm: &dyn LlmProvider, network: &Network, sync: &Sync, tx: &Sender<WorldSnapshot>) {
    loop {
        let tick = world.tick();

        // 1. 感知 + Spark
        let sparks: Vec<(AgentId, Spark)> = world.agents()
            .map(|a| (a.id, a.compute_spark(&world)))
            .collect();

        // 2. 并行思考（各Agent独立）
        let mut actions = Vec::new();
        for (agent_id, spark) in sparks {
            let agent = world.get_agent(agent_id);
            let perception = agent.perceive(&world);

            // 2a. 硬约束过滤
            let safe_actions = rule_engine::filter(&perception, &agent.inventory, &world);

            // 2b. 构建Prompt
            let prompt = build_prompt(&agent, &spark, &perception, &world);

            // 2c. LLM生成
            let candidates = llm.generate(LlmRequest { prompt, .. }).await;

            // 2d. 规则校验 + 动机选择
            let action = match candidates {
                Ok(resp) => {
                    let validated = rule_engine::validate(resp.parsed_action, &safe_actions);
                    select_best(validated, &agent.motivation)
                }
                Err(_) => rule_engine::fallback(&agent, &perception),
            };
            actions.push((agent_id, action));
        }

        // 3. 执行动作
        let mut effects = Vec::new();
        for (agent_id, action) in actions {
            let effect = world.apply_action(agent_id, &action);
            effects.push(effect);

            // 生成CRDT操作
            let ops = sync.generate_ops(&effect);
            network.gossip_broadcast(ops).await;
        }

        // 4. 合并远端CRDT
        while let Ok(remote_op) = network.try_recv_crdt() {
            sync.merge(remote_op);
        }
        world.reconcile(sync.state());

        // 5. Echo：更新记忆和动机
        for effect in &effects {
            let agent = world.get_agent_mut(effect.agent_id);
            agent.update_memory(effect);
            agent.evolve_motivation(effect);
            agent.age += 1;

            // 死亡检查
            if agent.health <= 0 || agent.age >= agent.max_age {
                let legacy = agent.create_legacy(&world);
                world.deposit_legacy(legacy);
            }
        }

        // 6. 环境压力tick
        world.pressure_tick();

        // 7. 发送快照至Godot
        let snapshot = world.snapshot();
        tx.send(snapshot).ok();

        // 8. 持久化
        world.persist().await;

        tokio::time::sleep(Duration::from_secs(world.tick_interval())).await;
    }
}
```

#### JSON兼容解析

```rust
fn parse_action_json(raw: &str) -> Result<Action, ParseError> {
    // Layer 1: 直接解析
    if let Ok(action) = serde_json::from_str::<Action>(raw) {
        return Ok(action);
    }

    // Layer 2: 提取 {} 块
    if let Some(json_block) = extract_first_json_block(raw) {
        if let Ok(action) = serde_json::from_str::<Action>(json_block) {
            return Ok(action);
        }
    }

    // Layer 3: 修复常见错误
    let fixed = fix_common_json_errors(raw);
    if let Some(json_block) = extract_first_json_block(&fixed) {
        if let Ok(action) = serde_json::from_str::<Action>(json_block) {
            return Ok(action);
        }
    }

    Err(ParseError::InvalidJson)
}
```

### 4.4 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| LLM API超时 (>10s) | 取消请求，尝试降级链中下一个Provider |
| LLM API限流 (429) | Retry-After后重试，最多2次，仍失败则降级 |
| 本地GGUF OOM | 释放模型，回退至API Provider |
| JSON解析全部失败 | 降级为规则引擎兜底（安全动作） |
| P2P节点全断 | 继续本地tick，缓存CRDT操作，重连后合并 |
| CRDT签名不匹配 | 拒绝操作，记录警告日志 |
| Merkle根不一致 | 触发差异区域全量同步 |
| Agent背包溢出 | 拒绝采集，Prompt中加入背包满提示 |
| 世界文件损坏 | 从WorldSeed重建 + CRDT重放 |

### 4.5 Godot客户端设计

#### 场景树

```
Main (Node)
├── SimulationBridge (Rust GDExtension Node, autoload)
├── Camera2D (Camera2D, 可拖拽/缩放)
├── WorldView (Node2D)
│   ├── TileMapLayer (地形渲染, 256×256 chunks按需加载)
│   ├── Structures (Node2D, 建筑/遗迹Sprite)
│   └── Agents (Node2D, Agent Sprite2D+Label动态管理)
├── RightPanel (Control)
│   ├── AgentDetail (PanelContainer)
│   │   ├── NameLabel (Label)
│   │   ├── MotivationRadar (自定义Control, CanvasItem绘制雷达图)
│   │   ├── StatusLabel (Label: 当前动作/健康/年龄)
│   │   └── GuidePanel (VBoxContainer)
│   │       ├── MotivationSliders (6×HSlider + Label)
│   │       └── PreferenceButtons (HBoxContainer: 建议探索/交易/建造)
│   └── WorldInfo (PanelContainer)
│       ├── TickLabel (Label)
│       └── PressureList (ItemList: 活跃环境压力)
├── NarrativeFeed (PanelContainer, 底部)
│   └── RichTextLabel (叙事流, 自动滚动, 颜色编码)
└── TopBar (HBoxContainer)
    ├── TickCounter (Label)
    ├── AgentCount (Label)
    └── SpeedControl (OptionButton: 5s/10s/20s tick)
```

#### SimulationBridge关键实现

```rust
#[derive(GodotClass)]
#[class(base=Node)]
struct SimulationBridge {
    base: Base<Node>,
    rx: Option<Receiver<WorldSnapshot>>,
    cmd_tx: Option<Sender<SimCommand>>,
    agents_parent: OnReady<Gd<Node2D>>,
    tilemap: OnReady<Gd<TileMapLayer>>,
    narrative: OnReady<Gd<RichTextLabel>>,
}

#[godot_api]
impl INode for SimulationBridge {
    fn ready(&mut self) {
        // 启动Tokio运行时 + tick循环
        let (snap_tx, snap_rx) = mpsc::channel(100);
        let (cmd_tx, cmd_rx) = mpsc::channel(50);
        self.rx = Some(snap_rx);
        self.cmd_tx = Some(cmd_tx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                run_simulation(snap_tx, cmd_rx).await;
            });
        });
    }

    fn physics_process(&mut self, _delta: f64) {
        if let Some(rx) = &self.rx {
            while let Ok(snapshot) = rx.try_recv() {
                self.update_visuals(&snapshot);
            }
        }
    }
}

#[godot_api]
impl SimulationBridge {
    #[func]
    fn adjust_motivation(&mut self, agent_id: GString, dimension: i32, value: f32) {
        if let Some(tx) = &self.cmd_tx {
            tx.try_send(SimCommand::AdjustMotivation {
                agent_id: agent_id.to_string(),
                dimension: dimension as usize,
                value,
            }).ok();
        }
    }
}
```

## 5. 技术决策

### 决策1：全栈Rust

- **选型方案**：Rust统一实现模拟核心+网络+客户端桥接
- **选择理由**：rust-libp2p生产级P2P、无GC确定性模拟、内存安全、godot-rust GDExtension原生集成、编译至多平台
- **备选方案**：Python原型+Rust重写 / Go+Unity
- **放弃原因**：Python无生产级libp2p、GC不适合模拟确定性、Go移动端差、Unity授权与设计红线冲突

### 决策2：Godot 4.6 + godot-rust

- **选型方案**：Godot 4.6引擎 + gdext v0.5 GDExtension绑定
- **选择理由**：MIT开源符合设计红线、GDExtension原生Rust集成、桌面导出成熟、2D能力满足MVP、3D可增量升级
- **备选方案**：Unity+C FFI / Bevy纯Rust / 内嵌Web Server+浏览器
- **放弃原因**：Unity Runtime Fee风险、Bevy移动端不可用、内嵌Web不利于3D演进

### 决策3：自实现CRDT

- **选型方案**：自实现LWW-Register + G-Counter + OR-Set
- **选择理由**：精确匹配Agentora数据模型、避免y-py的JS运行时绑定、代码量可控
- **备选方案**：y-py (Yjs Python绑定) / crdt-rs库
- **放弃原因**：y-py非Rust、crdt-rs实现不完整

### 决策4：mistralrs作为本地推理

- **选型方案**：mistralrs v0.8.x 作为本地GGUF推理后端
- **选择理由**：纯Rust、支持GGUF、活跃维护(2026-04更新)、内置结构化输出、tokio兼容
- **备选方案**：llama-cpp-rs (llama.cpp绑定) / candle (HuggingFace纯Rust)
- **放弃原因**：llama-cpp-rs停止更新(2024-04)、candle不支持GGUF

### 决策5：mpsc Channel线程桥接

- **选型方案**：std::sync::mpsc Channel连接Tokio模拟线程与Godot主线程
- **选择理由**：Godot引擎类型非Send/Sync、Channel是唯一安全的跨线程方式、单向数据流清晰
- **备选方案**：godot-tokio库 / 共享内存+锁
- **放弃原因**：godot-tokio额外依赖、共享内存违反Godot线程安全约束

### 决策6：256×256大地图+区域划分

- **选型方案**：256×256网格起步，16×16区域划分，TileMapLayer按区域chunk按需渲染
- **选择理由**：大地图保证可玩性和探索空间、区域划分支持P2P兴趣过滤、按需渲染控制性能
- **备选方案**：32×32小地图 / 无限程序生成
- **放弃原因**：32×32太小无法支撑多Agent探索、程序生成增加MVP复杂度

### 决策7：三层记忆架构（借鉴Hermes）

- **选型方案**：ChronicleStore(Markdown冻结快照) + ChronicleDB(SQLite+FTS5) + StrategyHub(策略库)
- **选择理由**：
  - 借鉴Hermes Agent的成熟架构设计
  - 冻结快照保持Prompt稳定，避免prefix cache失效
  - FTS5全文索引足够满足标签/关键词检索，无向量索引开销
  - 策略库支持自我改进闭环
- **备选方案**：FAISS向量索引 + 单层记忆
- **放弃原因**：向量索引增加端侧计算负担、embedding模型内存占用、检索复杂度高

### 决策8：FTS5替代向量索引

- **选型方案**：SQLite FTS5 全文索引作为长期记忆检索方案
- **选择理由**：
  - SQLite内置，零额外依赖
  - emotion_tag/event_type等标签字段天然适合FTS5
  - 端侧性能轻量，适合手机运行
  - 查询语法简单，易于实现
- **备选方案**：FAISS向量索引 / hnsw-rs
- **放弃原因**：需要embedding模型增加内存、计算密集不适合端侧、语义相似度对Agentora场景非必需

### 决策9：策略库自我改进（借鉴Hermes Skills）

- **选型方案**：策略库使用Markdown+YAML frontmatter，支持create/patch/decay闭环
- **选择理由**：
  - 借鉴Hermes Skills的自我改进机制
  - 成功决策自动创建策略，失败立即patch
  - 衰减机制防止策略膨胀成为负担
  - progressive disclosure控制Prompt token消耗
- **备选方案**：硬编码策略 / 无策略库
- **放弃原因**：硬编码策略无法适应涌现、无策略库导致决策质量无法演化提升

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| 2B模型JSON遵循率低 | 高 | 多层兼容解析 + 规则引擎兜底 + Prompt工程迭代优化 |
| godot-rust v0.5 breaking changes | 中 | 锁定依赖版本，不轻易升级 |
| rust-libp2p GossipSub性能 | 中 | 压力测试，预留WebSocket Relay降级通道 |
| 256×256地图Godot渲染性能 | 中 | TileMap按区域chunk按需加载，仅渲染视口内区域 |
| mistralrs本地推理MVP不可用 | 中 | 优先API模式开发，本地推理作为Step 5加入 |
| 多节点CRDT合并延迟 | 低 | Tick间隔5-10秒容忍秒级同步延迟 |
| Agent涌现行为不足 | 中 | 调优动机向量初始分布、Prompt设计、压力池频率 |
| 策略库膨胀 | 中 | 衰减机制每50tick *=0.95，deprecated策略自动清理 |
| FTS5检索性能 | 低 | 记忆片段数量有限（<500条），FTS5内置足够高效 |
| 记忆围栏被LLM忽略 | 低 | 明确的系统注提示 + Prompt模板优化测试验证 |

## 7. 迁移方案

### 7.1 部署步骤

1. 用户下载对应平台的桌面包（.exe / .app / AppImage）
2. 解压/安装后双击运行
3. 首次启动生成PeerId密钥 + 加载默认WorldSeed
4. Godot界面自动打开，世界开始运行
5. 多节点：修改WorldSeed.toml填入种子节点地址后启动

### 7.2 回滚方案

- 每个Step完成后git tag标记
- 出现重大问题时回退到上一个tag版本
- SQLite数据库版本化，支持降级迁移

## 8. 待定事项

- [ ] 具体选用哪个GGUF模型（Qwen3.5-1.7B / Gemma-4-2B 等），等模型调研完成后确定
- [ ] Prompt模板具体设计，需在Step 1中迭代验证
- [ ] 区域chunk的Godot TileMapLayer加载策略（静态全加载 vs 按需加载），需性能测试后决定
- [ ] 交易系统的并发安全（两Agent同时交易同一资源的冲突处理）
- [ ] Agent命名规则和视觉标识（颜色/形状区分）
- [ ] FTS5查询模板与Spark类型的映射规则（需Prompt工程迭代）
- [ ] 策略库与动机向量联动的具体权重系数（success_rate → motivation_delta 的影响幅度）
- [ ] ChronicleStore char_limit 的最优值（1800/500 是否合理，需实测调整）
- [ ] 记忆安全扫描的威胁模式库是否需要扩展（针对Agentora特定场景）

