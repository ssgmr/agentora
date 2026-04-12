# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Agentora 智纪** — 端侧多模态大模型AI智能体驱动的去中心化数字文明模拟器。无中心服务器、无预设剧本、支持断网运行的持久化模拟沙盒。

项目当前处于 **MVP内核开发阶段**

## Commands

```bash
# 构建全部crates
cargo build

# 构建release版本（用于Godot打包）
cargo build --release

# 运行全部单元测试
cargo test

# 运行单个测试文件
cargo test --test motivation_tests
cargo test --test decision_tests
cargo test --test crdt_tests
cargo test --test json_parse_tests

# 运行集成测试（需要LLM服务）
cargo test --test single_agent
cargo test --test multi_agent
cargo test --test multi_node

# 构建GDExtension动态库（bridge crate）并复制到client/bin/
cargo build -p agentora-bridge
cargo bridge                      # 别名，等同于上面
cargo bridge-release              # release模式别名
bash scripts/build-bridge.sh            # 推荐：自动编译+复制
bash scripts/build-bridge.sh --release  # release模式
scripts/build-bridge.bat                # Windows批处理版本

# 启动多节点测试环境
bash scripts/start_multi_node.sh

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
├── core/      # 核心引擎：动机、决策、世界、Agent、记忆、策略、存储
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
- **`SimpleActionType`** — 规则引擎动作类型（移动/交互/建造/社交）
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
- `simulation_bridge.gd` — 桥接GDScript与Rust GDExtension
- `agent_manager.gd` — Agent生命周期和状态管理
- `world_renderer.gd` — TileMap世界渲染
- `camera_controller.gd` — 相机控制
- `narrative_feed.gd` — 叙事流面板
- `motivation_radar.gd` — 6维动机雷达图
- `guide_panel.gd` — 引导面板

#### Godot可执行文件路径
- 路径：`D:/tool/Godot/Godot_v4.6.2-stable_win64.exe`（注意是文件，不是目录）
- 运行命令：`"D:/tool/Godot/Godot_v4.6.2-stable_win64.exe" --path client`

#### UI布局验证：自动截图
外部屏幕截图工具（nircmd、snipping tool、Windows.Graphics.Capture等）无法捕获Godot窗口（Vulkan渲染器绕过GDI）。**必须使用Godot内部截图**。

**步骤：**
1. 创建 `client/scripts/auto_screenshot.gd`（作为Autoload加载）：
```gdscript
extends Node
func _ready():
    await get_tree().create_timer(3.0).timeout
    var viewport = get_viewport()
    var img = viewport.get_texture().get_image()
    img.save_png("D:/work/code/rust/agentora/screenshot_godot.png")
    print("[AutoScreenshot] Saved screenshot_godot.png")
    get_tree().quit()
```
2. 在 `client/project.godot` 中添加Autoload：
```ini
[autoload]
AutoScreenshot="*res://scripts/auto_screenshot.gd"
```
3. 运行后自动截图到项目根目录并退出，用 `Read` 工具查看PNG

**重要：** 验证完成后删除 `auto_screenshot.gd` 和Autoload配置，避免每次运行都截图退出。

#### Godot UI布局经验总结
- **CanvasLayer + Control锚定**：Control节点在CanvasLayer下必须设置 `layout_mode = 2` 才能生效
- **anchor_right=1.0 + anchor_left=1.0** 会导致宽度为0，使用 `anchor_right=0.0 + offset_left` 替代
- **VBoxContainer子节点**：需要正确设置 `size_flags_vertical`（0=不扩展, 3=填充扩展）避免内容溢出
- **NarrativeFeed vs RightPanel**：NarrativeFeed的 `anchor_right` 应设为 `0.0`（不覆盖右侧），或设置 `offset_right = 930.0` 避开RightPanel区域
- **@onready时序问题**：某些节点路径在 `_ready()` 的@onready阶段可能还未就绪，改用 `get_node_or_null()` 延迟获取

### OpenSpec变更管理 (`openspec/`)
项目使用OpenSpec工作流管理功能开发：
- `openspec/changes/archive/` — 已归档的变更（MVP核心引擎、决策管道、记忆系统、策略系统、LLM Provider、P2P网络、Legacy系统等）
- `openspec/specs/` — 当前活跃规范（decision-pipeline、configurable-memory、token-budget、strategy-*、legacy-system等）
- `openspec/config.yaml` — OpenSpec配置
- 使用`/opsx:new`创建新变更，`/opsx:apply`实施，`/opsx:archive`归档

### 依赖关系
- `core` → `ai`, `sync` (决策需要LLM、状态需要CRDT类型)
- `network` → `sync` (广播CRDT操作)
- `bridge` → `core` (桥接核心引擎到Godot)
- 根包 → 所有crates (集成测试)

### Storage层 (`crates/core/src/storage/`)
- **`WorldStore`** — 世界状态持久化（SQLite）
- **`AgentStore`** — Agent状态持久化（SQLite）
- **`MemoryStore`** — 记忆存储封装
- **`StrategyStore`** — 策略存储（创建/查询/更新/衰减）
- **`MapStore`** — 地图数据持久化
- **`Schema`** — SQLite表结构定义

### Agent模块 (`crates/core/src/agent/`)
- **`movement.rs`** — 移动逻辑（边界检查、碰撞、地形影响）
- **`inventory.rs`** — 物品栏管理（收集/消耗/交易）
- **`trade.rs`** — Agent间交易协议
- **`dialogue.rs`** — 对话生成和传播
- **`combat.rs`** — 战斗系统
- **`alliance.rs`** — 联盟系统

### World模块 (`crates/core/src/world/`)
- **`map.rs`** — TileMap地形生成和查询
- **`region.rs`** — 区域划分和管理
- **`resource.rs`** — 资源分布和刷新
- **`pressure.rs`** — 环境压力系统（资源波动/气候事件/区域封锁）
- **`structure.rs`** — 建筑放置和管理
- **`generator.rs`** — 世界生成器（基于WorldSeed）

### Bridge架构 (`crates/bridge/src/`)
GDExtension桥接Rust核心引擎到Godot客户端：
- **`SimulationBridge`** — Godot Node扩展，通过mpsc通道与模拟线程通信
- **`SimCommand`** — 控制命令枚举（Start/Pause/SetTickInterval/AdjustMotivation/InjectPreference）
- **双线程模型** — Godot主线程渲染 + Rust后台模拟线程（内嵌Tokio运行时）
- **`WorldSnapshot`** — 每tick序列化世界状态发送至Godot
- **`agent_decision()`** — 简化决策循环（5维动机 + 规则引擎）
- 产物为`cdylib`动态库，复制到`client/bin/agentora_bridge.dll`

**动机引擎** (`crates/core/src/motivation.rs`)
- 6维向量：生存/社交/认知/表达/权力/传承
- 惯性衰减公式：`new = old * 0.85 + 0.5 * 0.15`
- 缺口计算 → Spark触发决策

**决策管道** (`crates/core/src/decision.rs`)
- 火花生成 → 硬约束过滤 → Prompt构建 → LLM调用 → 规则校验 → 动机加权选择

**三层记忆** (`crates/core/src/memory/`)
- `ChronicleStore`：冻结快照（Markdown文件注入Prompt）
- `ChronicleDB`：SQLite + FTS5全文索引
- `TokenBudget`：总量控制 ≤1800 chars（可配置）
- `ShortTerm`：短期记忆队列
- `Fence`：记忆围栏/隔离

**策略库** (`crates/core/src/strategy/`)
- 自我创建、Patch改进、衰减清理
- Spark类型匹配检索，动机联动反馈

### 世界模型四层架构
1. **公理层** — 物理常数/稀缺性/死亡规则，硬编码+签名配置，高门槛DAO投票修改
2. **事件层** — 所有交互原始日志，CRDT日志流+Merkle根，P2P Gossip广播
3. **叙事层** — LoreGraph知识图谱，多版本并存，声誉加权传播
4. **正典层** — 共识签名记录，质押投票后正典化，直接改变世界参数

### 核心循环：Spark → Act → Echo → Legacy
- Spark: 环境压力/社会压力/内部压力感知
- Act: 6维动机权重计算 → 策略生成（合作/探索/创造/规避/竞争）
- Echo: 世界反馈（物理/社会/记忆/系统四层）
- Legacy: 死亡沉淀为遗产，广播至P2P网络成为新Spark

### 项目结构
```
agentora/
├── Cargo.toml                  # Workspace定义
├── crates/
│   ├── core/                   # 动机引擎 + 决策管道 + 世界模型 + Agent交互 + 记忆 + 策略 + 存储
│   ├── ai/                     # LLM接入层：Provider trait、OpenAI/Anthropic/本地推理、降级链
│   ├── network/                # P2P网络：libp2p、GossipSub、区域订阅
│   ├── sync/                   # CRDT同步：LWW、G-Counter、OR-Set、签名、Merkle
│   └── bridge/                 # Godot GDExtension：SimulationBridge、WorldSnapshot
├── client/                     # Godot 4客户端：TileMap渲染、Agent Sprite、叙事流面板
├── config/                     # 配置文件：llm.toml
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
- **单元测试**：`motivation_tests`, `decision_tests`, `crdt_tests`, `json_parse_tests`, `strategy_tests`, `memory_tests`, `legacy_tests`
- **集成测试**（需要LLM服务运行）：`single_agent`, `multi_agent`, `multi_node`
- 集成测试通过`tempfile`创建临时数据库，测试后自动清理

### 核心规则引擎 (`crates/core/src/rule_engine.rs`)
- `RuleEngine`：硬约束过滤 + 动作校验 + 兜底决策
- `WorldState`：世界状态快照（位置/库存/地形/资源）
- LLM失败时提供规则引擎兜底动作，保证系统鲁棒性

### 关键性能指标
- 骁龙8Gen3决策延迟 < 150ms，首token < 100ms
- 决策prompt严格控制在 2.5K tokens内，记忆预算max 1800 chars（可配置）

## Configuration

**LLM配置** (`config/llm.toml`)
- 主Provider默认使用OpenAI兼容端点（localhost:1234，兼容LM Studio等）
- 支持Anthropic兼容端点作为备用
- 决策max_tokens=500，temperature=0.7
- 记忆系统配置在 `[memory]` section，包含预算/存储/检索/容量/Prompt约束等参数

**世界种子** (`worldseeds/default.toml`)
- 地图256×256，区域16×16
- 初始Agent=5，资源密度=0.15
- 动机模板：gatherer/trader/explorer/builder

## Conventions

- 代码注释和文档使用中文
- 许可证：MIT License
- 设计红线：规则开源可审计，平台抽成≤15%，死亡不重置进度，不售卖数值优势（盈利模式待定）
- 经济双轨：尘(Dust)软货币 + 星(Star)硬货币，基尼系数<0.6 （经济系统待定）