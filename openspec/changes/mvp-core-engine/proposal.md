# 需求说明书

## 背景概述

Agentora（智纪）定位为端侧多模态大模型AI智能体驱动的去中心化数字文明模拟器，核心愿景是让AI Agent在无预设剧本的共享世界中基于自主动机决策，涌现出合作、冲突与文明演进。项目目前处于方案设计阶段，设计文档（`Agentora方案初稿.md`）已定义了四层世界模型、Spark→Act→Echo→Legacy核心循环、6维动机引擎等核心概念，但尚无可运行的代码实现。

MVP阶段的核心任务是验证最关键的假设：**AI Agent基于6维动机向量的自主决策，在多Agent共享世界中能否涌现出合作、冲突和世界演进？** 这一验证必须支持多节点联机（联机是核心体验的一部分而非可后续追加），且需要可分发给用户在桌面端运行。

## 变更目标

- 验证6维动机引擎驱动下的Agent自主决策是否产生有趣、个性化、有涌现性的行为
- 验证多Agent在同一共享世界中能否自然涌现合作、冲突与社会结构
- 验证Tick-Based脉冲决策模式（5-10秒/周期）下端侧LLM推理的可行性
- 实现多节点P2P联机，通过CRDT保证状态最终一致
- 实现Agent死亡→遗产沉淀→成为他人Spark的闭环
- 交付可分发的桌面应用（Windows/macOS/Linux），用户可独立运行观察世界
- 为后续3D升级、移动端适配、经济系统建立可演进的Rust+Godot架构基础

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `motivation-engine` | 6维动机向量引擎：生存与资源、社会与关系、认知与好奇、表达与创造、权力与影响、意义与传承。支持惯性衰减(α=0.85)和事件驱动微调，每tick计算动机缺口生成Spark |
| `decision-pipeline` | 决策管道：硬约束过滤→上下文构建(≤2.5K tokens)→LLM生成候选→规则校验→动机加权选择。输出结构化JSON Action |
| `world-model` | 世界模型：256×256网格大地图，多区域划分，资源节点（矿/农田/森林/水源）带再生周期，环境压力系统，气候/资源枯竭等动态事件 |
| `agent-interaction` | Agent交互系统：移动、采集、交易（提议/接受/拒绝）、对话（LLM生成）、攻击、建造、结盟、遗产（死亡→遗迹+记忆散落+遗嘱广播） |
| `memory-system` | 记忆系统：短期记忆(最近5条完整文本) + 中期记忆(LLM压缩摘要) + 长期记忆(关键事件向量索引)，总量控制在1800 tokens内 |
| `llm-bridge` | LLM接入层：统一Provider trait，支持OpenAI兼容API、Anthropic API、本地GGUF推理三种后端，结构化JSON输出+多层兼容降级+规则引擎兜底 |
| `p2p-network` | P2P网络层：rust-libp2p集成，GossipSub事件广播、KAD DHT节点发现、Circuit Relay v2 NAT穿透，Transport抽象支持后续换WebRTC |
| `crdt-sync` | CRDT状态同步：自实现LWW-Register(Agent状态/地图结构)、G-Counter(资源采集量)、OR-Set(事件日志/叙事版本)，Merkle根校验 |
| `godot-client` | Godot 4客户端：GDExtension bridge crate + mpsc Channel线程桥接，2D TileMap世界渲染、Agent Sprite动态创建、动机雷达图(CanvasItem自绘)、叙事流(RichTextLabel)、玩家引导面板(HSlider动机调整) |
| `legacy-system` | 遗产系统：Agent死亡→生成遗迹实体(墓冢/废墟) + 物品散落 + 记忆压缩为回响日志 + 关系网转为未竟契约→GossipSub广播→他人可交互 |
| `world-seed` | 世界种子配置：WorldSeed.toml定义初始地图大小、资源分布、区域参数、压力池配置、Agent初始动机模板，支持从文件加载世界 |

### 修改功能

无（全新项目，首版实现）

## 影响范围

- **代码模块**：全新Cargo workspace（core/network/sync/ai/bridge）+ Godot 4客户端项目
- **API接口**：无外部API，内部crate间通过Rust trait和channel通信
- **依赖组件**：rust-libp2p、godot-rust(gdext v0.5)、tokio、axum(仅调试用HTTP)、rusqlite、serde_json、llama-cpp-rs或mistralrs(本地推理)
- **关联系统**：LLM API服务（OpenAI/Anthropic）为外部依赖，需用户提供API Key或本地模型GGUF文件

## 验收标准

- [ ] 5个Agent在256×256世界中持续运行30分钟，产生合作/冲突的涌现行为
- [ ] 多节点联机（≥2节点），CRDT状态最终一致，无死锁
- [ ] Agent决策延迟在API模式下≤5秒/tick，本地推理模式下≤8秒/tick
- [ ] Agent死亡产生遗产，其他Agent可发现并交互遗迹
- [ ] 玩家可通过Godot客户端调整Agent动机权重并观察行为变化
- [ ] 桌面端可打包为单文件分发（Win/Mac/Linux），双击即可运行
- [ ] 至少产生3个"让人想看下去"的Agent决策故事链