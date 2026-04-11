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

# 构建GDExtension动态库（bridge crate）
cargo build --release -p agentora-bridge

# 启动多节点测试环境
bash scripts/start_multi_node.sh

# Godot客户端打包（需先编译bridge）
godot --path client --export-release "Windows Desktop" agentora_windows.exe
```

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

### 依赖关系
- `core` → `ai`, `sync` (决策需要LLM、状态需要CRDT类型)
- `network` → `sync` (广播CRDT操作)
- `bridge` → `core` (桥接核心引擎到Godot)
- 根包 → 所有crates (集成测试)

### 核心组件

**动机引擎** (`crates/core/src/motivation.rs`)
- 6维向量：生存/社交/认知/表达/权力/传承
- 惯性衰减公式：`new = old * 0.85 + 0.5 * 0.15`
- 缺口计算 → Spark触发决策

**决策管道** (`crates/core/src/decision.rs`)
- 火花生成 → 硬约束过滤 → Prompt构建 → LLM调用 → 规则校验 → 动机加权选择

**三层记忆** (`crates/core/src/memory/`)
- ChronicleStore：冻结快照（Markdown注入Prompt）
- ChronicleDB：SQLite + FTS5全文索引
- TokenBudget：总量控制 ≤1800 chars

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
- 设计红线：规则开源可审计，平台抽成≤15%，死亡不重置进度，不售卖数值优势
- 经济双轨：尘(Dust)软货币 + 星(Star)硬货币，基尼系数<0.6