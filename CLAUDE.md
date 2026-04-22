# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Agentora 智纪** — 端侧多模态大模型AI智能体驱动的去中心化数文明模拟器。无中心服务器、无预设剧本、支持断网运行的持久化模拟沙盒（每个玩家只有一个Agent，玩家只做部分引导，Agent自主演化）。


## Commands

```bash
# 构建全部crates
cargo build

# 构建release版本（用于Godot打包）
cargo build --release

# 运行全部单元测试
cargo test

# 运行单个测试文件
cargo test --test decision_tests
cargo test --test crdt_tests
cargo test --test json_parse_tests
cargo test --test strategy_tests
cargo test --test memory_tests
cargo test --test legacy_tests
cargo test --test agent_tests

# 运行单个测试用例
cargo test --test decision_tests -- --test specific_test_name

# 运行集成测试（需要LLM服务）
cargo test --test single_agent
cargo test --test multi_agent
cargo test --test multi_node
cargo test --test llm_local_test

# 构建GDExtension动态库（bridge crate）并复制到client/bin/
cargo build -p agentora-bridge
cargo bridge                      # 别名，等同于上面
cargo bridge-release              # release模式别名
bash scripts/build-bridge.sh            # 推荐：自动编译+复制
bash scripts/build-bridge.sh --release  # release模式
scripts/build-bridge.bat                # Windows批处理版本

# 启动多节点测试环境
bash scripts/start_multi_node.sh

# 运行Godot客户端（本地调试，需确保godot命令在PATH中）
godot --path client
bash scripts/run_client.sh        # Linux/WSL别名
scripts\run_client.bat             # Windows批处理

# Godot客户端打包（需先编译bridge）
godot --path client --export-release "Windows Desktop" agentora_windows.exe
```

### Cargo别名 (`.cargo/config.toml`)
- `cargo bridge` → `cargo build -p agentora-bridge`
- `cargo bridge-release` → `cargo build -p agentora-bridge --release`

## Architecture

### Crate结构
```
crates/
├── core/      # 核心引擎：决策、世界、Agent、记忆、策略、存储
├── ai/        # LLM接入层：Provider trait、OpenAI/Anthropic/本地、降级链、JSON解析
├── network/   # P2P网络：libp2p、GossipSub、区域订阅、Codec
├── sync/      # CRDT同步：LWW、G-Counter、OR-Set、签名、Merkle
├── bridge/    # Godot GDExtension：SimulationBridge、WorldSnapshot序列化
```

### AI Crate (`crates/ai/src/`)
- **`LlmProvider` trait** — 统一接口，所有Provider实现此trait
- **`OpenAiProvider`** — OpenAI兼容API（localhost:1234，兼容LM Studio等）
- **`AnthropicProvider`** — Anthropic兼容端点（备用）
- **`LocalProvider`** — 本地GGUF推理（feature门控，当前禁用）
- **`FallbackChain`** — 降级链：主Provider失败时自动切换备用
- **`RetryProvider`** — 速率限制重试（解析Retry-After header）
- **`parse_action_json`** — 从LLM响应中提取JSON并解析为Action
- **`LlmConfig`** — 从`config/llm.toml`加载配置，含记忆系统子配置
- **`ResponseFormat`** — 支持JSON结构化响应

### Sync Crate (`crates/sync/src/`)
- **`LwwRegister`** — Last-Writer-Wins注册表（时间戳排序）
- **`GCounter`** — 增长计数器（PNDT-safe）
- **`OrSet`** — Observed-Remove Set（支持并发添加/删除）
- **`CrdtOp`** — 操作编码/解码（Codec序列化）
- **`SyncState`** — 同步状态管理
- **签名验证** — ed25519-dalek操作签名
- **Merkle校验** — SHA-256哈希树验证数据完整性

### Network Crate (`crates/network/src/`)
- **`Libp2pTransport`** — 主传输层，封装Swarm
- **`GossipManager`** — GossipSub广播管理
- **混合传输策略** — TCP直连 + DCUtR打洞 + Relay穿透
- **NAT穿透** — AutoNAT探测 + DCUtR Hole Punching
- **KAD DHT** — 节点发现 + 区域订阅
- **区域订阅** — 基于空间位置的Topic订阅

### Godot客户端 (`client/`)
Godot 4 GDScript客户端，负责渲染和交互：
- `main.gd` — 主场景入口
- `bridge_accessor.gd` — 桥接访问器：与SimulationBridge GDExtension节点交互
- `state_manager.gd` — 状态管理器：管理游戏状态
- `agent_manager.gd` — Agent生命周期和状态管理
- `world_renderer.gd` — TileMap世界渲染（地图尺寸从后端snapshot获取）
- `camera_controller.gd` — 相机控制（边界从后端snapshot动态设置）
- `narrative_feed.gd` — 叙事流面板
- `agent_detail_panel.gd` — Agent详情面板
- `milestone_panel.gd` — 文明里程碑面板

**重要：客户端不硬编码配置值**
- `_map_size` 从 `snapshot.terrain_width` 获取
- 相机边界通过 `set_map_bounds(width, height, tile_size)` 设置
- 所有配置来自后端 Core，客户端只负责渲染


#### Godot可执行文件
- 使用 `godot` 命令（需安装Godot并添加到PATH）
- macOS: 创建符号链接 `ln -sf /Applications/Godot.app/Contents/MacOS/Godot ~/.local/bin/godot`
- Windows: 将Godot安装目录添加到系统PATH环境变量
- 运行命令：`godot --path client`
- Godot需要编辑器模式（而非运行模式）来触发重新导入资源。

#### UI布局验证：自动截图（优先使用godot MCP）
外部屏幕截图工具（nircmd、snipping tool、Windows.Graphics.Capture等）无法捕获Godot窗口（Vulkan渲染器绕过GDI）。**必须使用Godot内部截图**。

**关键陷阱：`.godot/imported/` 缓存损坏**
- Godot 导入 PNG 后会在 `.godot/imported/` 生成 `.ctex` 缓存文件
- 缓存损坏时会导致 `Failed loading resource` 错误，即使 PNG 文件本身是合法的
- **解决**：`rm -rf client/.godot/imported/` 删除缓存，然后打开 Godot 编辑器让它重新导入
- 验证 PNG 是否合法：`file client/assets/textures/*.png` 应显示 `PNG image data`

**步骤：**
1. 创建 `client/scripts/auto_screenshot.gd`（作为Autoload加载）：
```gdscript
extends Node
func _ready():
    print("[AutoScreenshot] Auto screenshot loaded, waiting 15s...")
    await get_tree().create_timer(15.0).timeout
    print("[AutoScreenshot] Taking screenshot now...")
    var viewport = get_viewport()
    var img = viewport.get_texture().get_image()
    img.save_png("D:/work/code/rust/agentora/screenshot_godot.png")
    print("[AutoScreenshot] Saved screenshot_godot.png")
    get_tree().quit()
```
2. 在 `client/project.godot` 的 `[autoload]` 段下添加：
```ini
AutoScreenshot="*res://scripts/auto_screenshot.gd"
```
3. 运行 Godot：`godot --path client`

4. 用 `Read` 工具查看 `screenshot_godot.png`

**重要：**
- 验证时不要用 **headless 模式**，Vulkan 渲染需要窗口
- 验证完成后注释 `[autoload]` 下的 AutoScreenshot 配置(使用;注释)，避免每次运行都截图退出
- 截图路径必须使用项目根目录的绝对路径（如 `/Users/geminrong/work/code/python/agentora/screenshot_godot.png`），不能用 `user://`（无法从外部读取）
- 首次运行，GDExtension 需要在 Godot 编辑器中打开一次，让它扫描并注册 .gdextension 文件。直接 --path . 运行时可能还没有正确注册类。

#### Godot UI布局经验总结
- **CanvasLayer + Control锚定**：Control节点在CanvasLayer下必须设置 `layout_mode = 2` 才能生效
- **anchor_right=1.0 + anchor_left=1.0** 会导致宽度为0，使用 `anchor_right=0.0 + offset_left` 替代
- **VBoxContainer子节点**：需要正确设置 `size_flags_vertical`（0=不扩展, 3=填充扩展）避免内容溢出
- **NarrativeFeed vs RightPanel**：NarrativeFeed的 `anchor_right` 应设为 `0.0`（不覆盖右侧），或设置 `offset_right = 930.0` 避开RightPanel区域
- **@onready时序问题**：某些节点路径在 `_ready()` 的@onready阶段可能还未就绪，改用 `get_node_or_null()` 延迟获取
- **SimulationBridge 节点路径**（场景树根节点为 Main）：
  - `SimulationBridge` 是 Main 的直接子节点，不是 WorldView 的子节点
  - 场景树结构：`Main → [SimulationBridge, WorldView, Camera2D, UI]`
  - 在 `world_renderer.gd`（挂载于 WorldView）中引用：`get_node("../SimulationBridge")`
  - 在 `agent_manager.gd`（挂载于 WorldView/Agents）中引用：`get_node("../../SimulationBridge")`
  - **反复踩坑**：WorldView 脚本中用 `../../SimulationBridge` 会找不到节点，必须用 `../SimulationBridge`（向上一级而非两级）
- **GDScript 缩进**：严禁混用空格和制表符，GDScript 解析器会直接报错 `Mixed use of tabs and spaces for indentation`。Godot 编辑器默认使用制表符，手动编辑时注意保持一致

### OpenSpec变更管理 (`openspec/`)
项目使用OpenSpec工作流管理功能开发：
- `openspec/changes/archive/` — 已归档的变更（MVP核心引擎、决策管道、记忆系统、策略系统、LLM Provider、P2P网络、Legacy系统等）
- `openspec/specs/` — 当前活跃规范（decision-pipeline、configurable-memory、token-budget、strategy-*、legacy-system等）
- `openspec/config.yaml` — OpenSpec配置
- 使用`/opsx:new`创建新变更，`/opsx:apply`实施，`/opsx:archive`归档

### Core Crate顶层模块 (`crates/core/src/`)
- **`types.rs`** — 核心类型：AgentId, Position, Direction, ResourceType, TerrainType, StructureType, ActionType(13种), Action, PersonalitySeed, PeerId
- **`decision/mod.rs`** — DecisionPipeline：接收预构建感知 → LLM调用 → 规则校验 → 执行（`execute`方法），保留`execute_with_auto_perception`向后兼容
- **`decision/perception.rs`** — PerceptionBuilder：从WorldState构建感知摘要和路径推荐（已从decision.rs抽离）
- **`rule_engine.rs`** — 规则引擎：硬约束过滤 + 动作校验 + LLM失败兜底
- **`prompt.rs`** — PromptBuilder：分级截断（策略→记忆→感知），最大2500 tokens，中文感知估算
- **`narrative.rs`** — 叙事构建器：NarrativeBuilder, EventType, action_type_display
- **`seed.rs`** — WorldSeed配置加载
- **`snapshot.rs`** — WorldSnapshot/AgentSnapshot/CellChange/NarrativeEvent/LegacyEvent/PressureSnapshot（Godot序列化）

### Simulation模块 (`crates/core/src/simulation/`)
模拟编排层，管理 Agent 决策循环、世界时间推进、快照生成：
- **`mod.rs`** — 模块导出，重导出 Simulation, SimConfig, AgentDelta
- **`simulation.rs`** — Simulation结构体：封装World + DecisionPipeline，管理agent_handles/tick_handle/snapshot_handle，提供start/pause/resume API
- **`config.rs`** — SimConfig, SimMode 配置加载
- **`delta.rs`** — AgentDelta增量事件枚举（14种变体：AgentMoved/AgentDied/AgentSpawned/StructureCreated/TradeCompleted等）
- **`agent_loop.rs`** — Agent决策循环6阶段流水线：WorldState构建 → 感知 → 决策 → 应用 → Delta发送 → 叙事发送
- **`tick_loop.rs`** — 世界时间推进（advance_tick、生存消耗）
- **`snapshot_loop.rs`** — 定期快照生成（5秒完整状态兜底）
- **`npc.rs`** — NPC Agent创建（规则引擎快速决策）
- **`state_builder.rs`** — WorldStateBuilder：从World自动构建WorldState供决策使用
- **`delta_emitter.rs`** — DeltaEmitter：构建和发送delta事件
- **`narrative_emitter.rs`** — NarrativeEmitter：提取和发送叙事事件
- **`memory_recorder.rs`** — MemoryRecorder：记录动作到Agent记忆
- **`p2p_handler.rs`** — P2PMessageHandler：处理P2P网络消息

### Agent模块 (`crates/core/src/agent/`)
Agent只负责自身状态变更，World负责协调/校验/叙事。返回详细元组供World生成反馈。
- **`mod.rs`** — Agent实体（信任关系/临时偏好/frozen_inventory/pending_trade_id）
- **`movement.rs`** — `move_to(target) -> (bool, old_pos, new_pos)` 单步移动
- **`survival.rs`** — `eat_food()/drink_water() -> (success, delta, before, after, remaining)`
- **`social.rs`** — `talk_with()/receive_talk()` 对话与信任建立
- **`combat.rs`** — `receive_attack(damage, attacker_id)/initiate_attack(target_id)` 分段借用
- **`trade.rs`** — `freeze_resources/complete_trade_send/cancel_trade/give_resources/receive_resources`
- **`inventory.rs`** — 物品栏管理（收集/消耗），全局InventoryConfig单例
- **`alliance.rs`** — 联盟系统（信任值阈值0.5）
- **`shadow.rs`** — ShadowAgent：P2P远程Agent精简结构，支持apply_delta和过期检测

### World模块 (`crates/core/src/world/`)
- **`mod.rs`** — World主结构（256x256地图、区域、资源、建筑、压力池、Agent管理）
- **`map.rs`** — CellGrid地形网格
- **`region.rs`** — 区域划分和管理
- **`resource.rs`** — 资源节点分布和刷新
- **`pressure.rs`** — 环境压力事件池（资源波动/气候事件/区域封锁）
- **`structure.rs`** — 建筑结构（Camp, Fence, Warehouse）
- **`generator.rs`** — 世界生成器（基于WorldSeed）
- **`actions.rs`** — 动作执行路由（apply_action分发）
- **`snapshot.rs`** — 世界快照生成
- **`feedback.rs`** — 动作反馈生成（物理/社会/记忆/系统四层）
- **`tick.rs`** — 生存消耗、建筑效果、压力等tick处理
- **`milestones.rs`** — 文明里程碑
- **`legacy.rs`** — 遗产系统（Agent死亡/遗产沉淀）
- **`vision.rs`** — 视野扫描（scan_vision, calculate_direction）
- **`types.rs`** — 世界类型（MilestoneType, TradeStatus等）
- **`action_result.rs`** — 动作结果结构（ActionResultSchema, FieldChange, ActionSuggestion）

### Storage层 (`crates/core/src/storage/`)
- **`mod.rs`** — StorageManager（SQLite连接管理）
- **`schema.rs`** — SQLite表结构定义
- **`world_store.rs`** — 世界状态持久化
- **`agent_store.rs`** — Agent状态持久化
- **`memory_store.rs`** — 记忆存储封装
- **`strategy_store.rs`** — 策略存储（创建/查询/更新/衰减）
- **`map_store.rs`** — 地图数据持久化

### Memory模块 (`crates/core/src/memory/`)
- **`mod.rs`** — MemorySystem：ShortTerm + ChronicleDB + ChronicleStore + TokenBudget
- **`chronicle_store.rs`** — 冻结快照（Markdown文件注入Prompt）
- **`chronicle_db.rs`** — SQLite + FTS5全文索引
- **`short_term.rs`** — 短期记忆队列
- **`token_budget.rs`** — 总量控制（可配置预算）
- **`fence.rs`** — 记忆围栏/隔离

### Strategy模块 (`crates/core/src/strategy/`)
- **`mod.rs`** — StrategyHub（YAML frontmatter STRATEGY.md文件加载）
- **`create.rs`** — 策略自我创建
- **`patch.rs`** — 策略改进/Patch
- **`decay.rs`** — 衰减清理
- **`retrieve.rs`** — Spark类型匹配检索

### 依赖关系
- `core` → `ai`, `sync` (决策需要LLM、状态需要CRDT类型)
- `network` → `sync` (广播CRDT操作)
- `bridge` → `core`, `ai` (桥接核心引擎和LLM到Godot)
- 根包 → 所有crates (集成测试)

### Bridge架构 (`crates/bridge/src/`)
GDExtension桥接Rust核心引擎到Godot客户端（拆分为4个模块）：
- **`lib.rs`** — 入口：定义AgentoraExtension，重导出SimulationBridge, SimCommand
- **`bridge.rs`** — SimulationBridge GDExtension节点：实现INode(ready, physics_process)，通过mpsc通道接收snapshot/delta/narrative并emit_signal给GDScript，提供start_simulation/pause/toggle_pause/inject_preference/get_agent_data等[func] API
- **`conversion.rs`** — 类型转换：delta_to_dict, agent_to_dict, snapshot_to_dict（Rust结构体→GDScript Dictionary/Variant）
- **`logging.rs`** — LogConfig：从config/log.toml加载配置，init_logging初始化console+file双层日志
- **`simulation_runner.rs`** — 模拟线程运行器：在独立OS thread创建tokio runtime，运行Simulation并处理SimCommand命令循环

**数据流**：`SimulationBridge.ready()` → spawn OS thread → `run_simulation_with_api()` → tokio runtime → `Simulation::start()` → mpsc channels → `physics_process` try_recv → emit_signal到Godot

**重要注意事项**：
- 产物为`cdylib`动态库（macOS: `.dylib`, Linux: `.so`, Windows: `.dll`），每次构建后需复制到`client/bin/`
- **线程安全**：`godot_print!`等godot-rust API只能在主线程调用，异步task中会panic，应改用`eprintln!`
- **WorldSeed加载**：必须从`worldseeds/default.toml`通过`WorldSeed::load()`加载，配置变更后需重新编译bridge

**决策管道** (`crates/core/src/decision/mod.rs`)
- 接收预构建感知 → LLM调用 → 规则校验 → 执行
- PerceptionBuilder独立于DecisionPipeline，在`decision/perception.rs`中实现

**规则引擎** (`crates/core/src/rule_engine.rs`)
- `RuleEngine`：硬约束过滤 + 动作校验 + LLM失败兜底
- `WorldState`：世界状态快照（位置/库存/地形/资源）
- 保证LLM不可用时的系统鲁棒性

**三层记忆** (`crates/core/src/memory/`)
- `ChronicleStore`：冻结快照（Markdown文件注入Prompt）
- `ChronicleDB`：SQLite + FTS5全文索引
- `TokenBudget`：总量控制 ≤1800 chars（可配置：total=1800, chronicle=800, db=600, strategy=400）
- `ShortTerm`：短期记忆队列
- `Fence`：记忆围栏/隔离

**策略库** (`crates/core/src/strategy/`)
- 自我创建、Patch改进、衰减清理
- Spark类型匹配检索
- 基于YAML frontmatter的STRATEGY.md文件加载

**Prompt构建** (`crates/core/src/prompt.rs`)
- 分级截断策略：策略 → 记忆 → 感知（优先保留核心决策上下文）
- 最大2500 tokens，中文感知估算

### 世界模型四层架构
1. **公理层** — 物理常数/稀缺性/死亡规则，硬编码+签名配置，高门槛DAO投票修改
2. **事件层** — 所有交互原始日志，CRDT日志流+Merkle根，P2P Gossip广播
3. **叙事层** — LoreGraph知识图谱，多版本并存，声誉加权传播
4. **正典层** — 共识签名记录，质押投票后正典化，直接改变世界参数

### 核心循环：Spark → Act → Echo → Legacy
- Spark: 环境压力/社会压力/内部压力感知
- Act: 状态评估 → LLM决策 → 规则校验 → 策略生成（合作/探索/创造/规避/竞争）
- Echo: 世界反馈（物理/社会/记忆/系统四层）
- Legacy: 死亡沉淀为遗产，广播至P2P网络成为新Spark

### 项目结构
```
agentora/
├── Cargo.toml                  # Workspace定义
├── crates/
│   ├── core/                   # 决策管道 + 世界模型 + Agent交互 + 记忆 + 策略 + 存储
│   ├── ai/                     # LLM接入层：Provider trait、OpenAI/Anthropic/本地推理、降级链
│   ├── network/                # P2P网络：libp2p、GossipSub、区域订阅
│   ├── sync/                   # CRDT同步：LWW、G-Counter、OR-Set、签名、Merkle
│   └── bridge/                 # Godot GDExtension：SimulationBridge、WorldSnapshot
├── client/                     # Godot 4客户端：TileMap渲染、Agent Sprite、叙事流面板、Agent详情、文明里程碑
├── config/                     # 配置文件：llm.toml, sim.toml, log.toml
├── worldseeds/                 # 世界种子：default.toml
├── tests/                      # 测试：单元测试 + 集成测试
├── scripts/                    # 脚本：多节点启动、打包说明
├── openspec/                   # OpenSpec变更管理：proposal、specs、tasks
└── src/                        # 根包（集成测试入口）
```

### 技术栈
- **端侧模型**: Qwen3.5-2B (主力) / Qwen3.5-35B-A3B MoE (复杂决策) / Gemma 4-2B
- **推理框架**: MLC LLM (主) + MLX (Apple) + llama.cpp (兜底)
- **P2P网络**: libp2p + WebRTC/QUIC + GossipSub
- **状态同步**: Yjs/Automerge CRDT
- **存储**: SQLite (热/温) + FAISS-lite (向量索引) + IPFS/Arweave (冷数据)

### 测试结构 (`tests/`)
- **单元测试**：`decision_tests`, `crdt_tests`, `json_parse_tests`, `strategy_tests`, `memory_tests`, `legacy_tests`, `agent_tests`
- **集成测试**（需要LLM服务运行）：`single_agent`, `multi_agent`, `multi_node`, `llm_local_test`
- 集成测试通过`tempfile`创建临时数据库，测试后自动清理

### 关键性能指标
- 骁龙8Gen3决策延迟 < 150ms，首token < 100ms
- 决策prompt严格控制在 2.5K tokens内，记忆预算max 1800 chars（可配置）

## Configuration

**LLM配置** (`config/llm.toml`)
- 主Provider默认使用OpenAI兼容端点（localhost:1234，兼容LM Studio等）
- 支持Anthropic兼容端点作为备用
- 决策max_tokens=500，temperature=0.7
- 默认模型：`gemma-4-e2b-it`
- 记忆系统配置在 `[memory]` section，包含预算/存储/检索/容量/Prompt约束等参数
  - `total_budget=1800` chars, `chronicle_budget=800`, `db_budget=600`, `strategy_budget=400`
  - 检索层：`importance_threshold=0.5`, `search_limit=5`, `snippet_max_chars=200`
  - 短期记忆容量：`short_term_capacity=5`

**模拟配置** (`config/sim.toml`)
- `initial_agent_count=1`, `npc_count=0`（实际玩家只有一个Agent，NPC为开发测试用）
- `npc_decision_interval_secs=5`, `player_decision_interval_secs=2`

**日志配置** (`config/log.toml`)
- 日志目录：`../logs`（相对于Godot工作目录）
- 全局日志级别：`debug`
- 支持控制台和文件双输出，文件日志支持按日/小时轮转
- 可按模块设置不同日志级别（`[log.targets]`）

**世界种子** (`worldseeds/default.toml`)
- 地图256×256，区域16×16
- 地形比例：plains 0.5 / forest 0.25 / mountain 0.1 / water 0.1 / desert 0.05
- 初始Agent=5，资源密度=0.02
- 生成策略：scattered（分散）
- 压力池配置：资源波动0.3、气候事件概率0.1、封锁持续10-30tick

## Conventions

- 代码注释和文档使用中文
- 许可证：MIT License
- 改动后需主动运行客户端，通过日志文件信息（logs目录）、截图等方式验证、修复问题
- 客户端测试、调试、验证首选 Godot MCP 工具（场景树查询、属性读写、输入模拟、截图等），而非手动操作，调试后记得关闭godot客户端
- Agent的LLM决策和规则引擎是两套独立的决策系统，本系统的目标是给Agent充分的自主性，规则引擎只是用来兜底和测试的
- 应该给Agent充分的自主性，少在代码里做动作执行兜底，Agent决策、行动的结果，应该让LLM都感知到，才能更好优化下一步动作
- 6维动机向量：生存/社交/认知/表达/权力/传承这是指导玩法设计的理念，但不是实现的具体手段