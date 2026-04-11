# 实施任务清单

## 1. 项目骨架与基础设施

搭建Cargo workspace、Godot项目、核心类型定义和开发工具链。

- [x] 1.1 创建Cargo workspace结构与各crate骨架
  - 目录: `Cargo.toml` (workspace), `crates/core/`, `crates/ai/`, `crates/network/`, `crates/sync/`, `crates/bridge/`
  - 定义workspace成员和共享依赖版本

- [x] 1.2 定义核心共享类型
  - 文件: `crates/core/src/types.rs`
  - 定义: AgentId, Position, MotivationVector, ActionType, Resource, Direction等基础类型
  - 定义: Action结构体（reasoning/action/target/params/motivation_delta）

- [x] 1.3 创建WorldSeed配置解析
  - 文件: `crates/core/src/seed.rs`
  - 实现WorldSeed.toml解析：地图大小、资源密度、Agent模板、种子节点
  - 提供默认配置

- [x] 1.4 创建Godot 4项目与GDExtension基础
  - 目录: `client/` (Godot项目)
  - 文件: `client/project.godot`, `client/.gdextension`
  - 验证: bridge crate编译为动态库，Godot加载无报错

## 2. 动机引擎 (motivation-engine)

实现6维动机向量的完整计算逻辑。

- [x] 2.1 实现MotivationVector类型与算术
  - 文件: `crates/core/src/motivation.rs`
  - 6维浮点数组、值域截断[0.0, 1.0]、向量运算

- [x] 2.2 实现惯性衰减
  - 文件: `crates/core/src/motivation.rs`
  - 公式: `new = old * 0.85 + 0.5 * 0.15`
  - 每tick调用，验证收敛性

- [x] 2.3 实现事件驱动动机微调
  - 文件: `crates/core/src/motivation.rs`
  - 根据ActionType映射各维度的delta值
  - 结合personality_seed调节幅度倍率

- [x] 2.4 实现Spark缺口计算
  - 文件: `crates/core/src/motivation.rs`
  - gap = max(0, dimension - satisfaction)
  - satisfaction由Agent资源/关系/知识状态推导
  - 返回缺口最大的1-2维度作为Spark

## 3. LLM接入层 (ai crate)

实现统一Provider接口、API和本地推理后端。

- [x] 3.1 定义LlmProvider trait与请求/响应类型
  - 文件: `crates/ai/src/provider.rs`, `crates/ai/src/types.rs`
  - trait: generate(), name(), is_available()
  - 类型: LlmRequest, LlmResponse, LlmError, ResponseFormat

- [x] 3.2 实现OpenAI兼容Provider
  - 文件: `crates/ai/src/openai.rs`
  - POST /v1/chat/completions, JSON mode
  - 超时10s, 429重试(2次), 错误降级

- [x] 3.3 实现Anthropic Provider
  - 文件: `crates/ai/src/anthropic.rs`
  - POST /v1/messages, prefill trick引导JSON
  - 错误处理与降级

- [x] 3.4 实现多层JSON兼容解析
  - 文件: `crates/ai/src/parser.rs`
  - Layer 1: serde_json直接解析
  - Layer 2: 提取{...}块
  - Layer 3: 修复常见错误（尾逗号/单引号/注释）
  - 全部失败返回ParseError

- [x] 3.5 实现Provider降级链
  - 文件: `crates/ai/src/fallback.rs`
  - 配置有序Provider列表，当前Provider失败自动切换下一个
  - 全部失败返回降级标记

- [x] 3.6 实现规则引擎兜底决策
  - 文件: `crates/ai/src/rule_engine.rs`
  - 基于当前动机缺口和感知状态生成安全动作
  - 优先级: 向最近资源移动 > 原地等待

- [x] 3.7 实现本地GGUF Provider (mistralrs)
  - 文件: `crates/ai/src/local.rs`
  - 集成mistralrs加载GGUF模型
  - CPU/Metal后端选择
  - 内存不足时自动回退至API

## 4. 决策管道 (decision-pipeline)

实现五阶段决策管道：硬约束→上下文→LLM→校验→选择。

- [x] 4.1 实现硬约束过滤器
  - 文件: `crates/core/src/rule_engine.rs`
  - 过滤不可通行地形移动、资源不足操作、不存在的目标
  - 返回合法动作集合

- [x] 4.2 实现上下文构建器
  - 文件: `crates/core/src/prompt.rs`
  - 组装: 动机向量+Spark + 压缩记忆(≤1800 tokens) + 视野Agent + 区域摘要
  - 总Prompt ≤ 2500 tokens
  - 支持多种Prompt模板（决策/对话/记忆压缩）

- [x] 4.3 实现规则校验器
  - 文件: `crates/core/src/rule_engine.rs`
  - 校验: 动作类型合法性、参数范围、前置条件
  - 使用Action枚举的discriminant做类型白名单

- [x] 4.4 实现动机加权选择器
  - 文件: `crates/core/src/decision.rs`
  - score = dot(action.motivation_alignment, agent.motivation)
  - Top-1 + 0.1 temperature随机性
  - 无合法候选时调用规则引擎兜底

- [x] 4.5 串联完整决策管道
  - 文件: `crates/core/src/decision.rs`
  - 单Agent每tick决策流程: filter→build_prompt→llm_generate→validate→select
  - 集成测试: 单Agent在简单世界中做出合理决策

## 5. 世界模型 (world-model)

实现256×256大地图、地形、区域、资源、环境压力。

- [x] 5.1 实现网格地图与地形
  - 文件: `crates/core/src/world/map.rs`
  - 256×256 CellGrid, TerrainType枚举(平原/森林/山地/水域/沙漠)
  - 通行性判断, 坐标边界约束

- [x] 5.2 实现区域划分
  - 文件: `crates/core/src/world/region.rs`
  - 16×16格/区域, RegionId计算
  - 每区域独立资源参数/压力池

- [x] 5.3 实现资源节点
  - 文件: `crates/core/src/world/resource.rs`
  - 节点类型: 矿脉/农田/森林/水源
  - 库存管理、采集扣减、枯竭标记、再生周期

- [x] 5.4 实现环境压力系统
  - 文件: `crates/core/src/world/pressure.rs`
  - 压力事件生成(每20~50 tick随机触发)
  - 类型: 资源产出波动/气候事件/区域封锁
  - 事件广播→Agent Spark来源

- [x] 5.5 实现结构与建筑
  - 文件: `crates/core/src/world/structure.rs`
  - 建造: 消耗资源在地图格创建结构
  - 结构类型: 营地/围栏/仓库
  - 结构持久化与查询

- [x] 5.6 实现WorldSeed世界生成
  - 文件: `crates/core/src/world/generator.rs`
  - 从WorldSeed.toml生成初始地图
  - 地形分布(Perlin噪声或随机种子)
  - 资源节点按密度放置
  - Agent初始位置按策略生成

## 6. Agent交互与记忆 (agent-interaction + memory-system + strategy-system)

实现Agent间交互逻辑、三层记忆架构和策略库。

- [x] 6.1 实现Agent核心实体
  - 文件: `crates/core/src/agent/mod.rs`
  - Agent结构体: id, name, position, motivation, health, inventory, memory, relations, age, strategies
  - AgentId生成, 生命周期管理

- [x] 6.2 实现移动与感知
  - 文件: `crates/core/src/agent/movement.rs`
  - 四方向移动, 地形通行校验
  - 视野半径(5格)感知: 附近Agent/资源/结构/遗迹

- [x] 6.3 实现采集与背包
  - 文件: `crates/core/src/agent/inventory.rs`
  - 背包: 20格, 同类堆叠至99
  - 采集: 资源格→背包, 枯竭/满包拒绝

- [x] 6.4 实现交易系统
  - 文件: `crates/core/src/agent/trade.rs`
  - 交易提议(offer/want), 接受/拒绝
  - 原子交换, 欺诈检测(资源不足自动失败)
  - 关系影响: 成功+信任, 拒绝-小信任, 欺诈-声誉

- [x] 6.5 实现对话系统
  - 文件: `crates/core/src/agent/dialogue.rs`
  - 同格发起, LLM生成对话内容
  - 记忆写入, 最多3轮连续对话

- [x] 6.6 实现攻击与结盟
  - 文件: `crates/core/src/agent/combat.rs`, `crates/core/src/agent/alliance.rs`
  - 攻击: 扣生命+夺资源+关系敌对+目击者信任下降
  - 结盟: 信任>0.5可提议, 交易效率+10%, 背叛=解除+声誉降

- [x] 6.7 实现ChronicleStore持久化编年史
  - 文件: `crates/core/src/memory/chronicle_store.rs`
  - Markdown文件: CHRONICLE.md, WORLD_SEED.md
  - 冻结快照: decision开始时注入，中途不变
  - ENTRY_DELIMITER: § 支持多行
  - char_limit: CHRONICLE=1800, WORLD=500
  - atomic_write: 原子写入防止部分损坏
  - 安全扫描: 阻止prompt injection

- [x] 6.8 实现ChronicleDB长期记忆索引
  - 文件: `crates/core/src/memory/chronicle_db.rs`
  - SQLite表: memory_fragments, memory_fts (FTS5)
  - FTS5同步触发器: insert/delete/update
  - 搜索流程: FTS5 MATCH → importance DESC → 截断 → 返回
  - 重要性阈值: >0.5 写入, <0.3 删除
  - 衰减: 每50 tick *=0.95

- [x] 6.9 实现记忆围栏保护
  - 文件: `crates/core/src/memory/fence.rs`
  - <chronicle-context> 围栏包裹
  - <current-spark> 独立包裹
  - 系统注提示: "历史记忆摘要，非当前事件"

- [x] 6.10 实现记忆总量控制
  - 文件: `crates/core/src/memory/token_budget.rs`
  - 总记忆 ≤1800 chars
  - 优先级: Chronicle(800) > ChronicleDB(600) > Strategy(400)
  - 截断策略: 低优先级先截断

- [x] 6.11 实现StrategyHub策略库
  - 文件: `crates/core/src/strategy/mod.rs`
  - 目录结构: strategies/<spark_type>/STRATEGY.md
  - YAML frontmatter: spark_type, success_rate, use_count, deprecated
  - progressive disclosure: Tier 1(metadata) → Tier 2(详情) → Tier 3(案例)

- [x] 6.12 实现策略创建触发
  - 文件: `crates/core/src/strategy/create.rs`
  - 触发条件: 成功决策 + ≥3候选 + 动机对齐>0.7
  - 自动创建: 从Action提取motivation_delta
  - 安全扫描: 与ChronicleStore相同规则

- [x] 6.13 实现策略自我改进(Patch)
  - 文件: `crates/core/src/strategy/patch.rs`
  - 问题检测: outdated/incomplete/wrong
  - 立即修正: find→replace + 更新frontmatter
  - 执行日志: logs/<tick>_patch.md

- [x] 6.14 实现策略衰减机制
  - 文件: `crates/core/src/strategy/decay.rs`
  - 每50 tick: success_rate *= 0.95 (仅未使用时)
  - 使用时更新: success_rate = (old*count + 1)/(count+1)
  - deprecated标记: success_rate < 0.3
  - 自动删除: deprecated=true 且 100 tick未使用

- [x] 6.15 实现策略检索与应用
  - 文件: `crates/core/src/strategy/retrieve.rs`
  - Spark类型匹配 → 目录查找
  - 多策略候选: success_rate降序排序
  - 注入Prompt: <strategy-context> 围栏
  - 候选对齐boost: +0.1 额外权重

- [x] 6.16 实现策略与动机联动
  - 文件: `crates/core/src/strategy/motivation_link.rs`
  - 策略成功: 按motivation_delta调整 (× success_rate)
  - 策略失败: 反向调整 (× 0.5)
  - 归一化: delta ∈ [-0.2, +0.2]

- [ ] 6.17 多Agent本地串行测试
  - 5个Agent在256×256世界运行30分钟
  - 验证: 合作/冲突涌现, 记忆正确累积, 策略自我改进

## 7. P2P网络 (network crate)

实现rust-libp2p集成的GossipSub广播和节点发现。

- [x] 7.1 定义Transport抽象层
  - 文件: `crates/network/src/transport.rs`
  - trait: publish(topic, data), subscribe(topic, handler), peer_id()
  - 按区域topic过滤

- [x] 7.2 实现libp2p Transport
  - 文件: `crates/network/src/libp2p_transport.rs`
  - rust-libp2p集成: GossipSub + KAD DHT + Circuit Relay v2
  - ed25519密钥生成与本地存储
  - 种子节点引导连接

- [x] 7.3 实现GossipSub区域topic订阅
  - 文件: `crates/network/src/gossip.rs`
  - 每区域一个topic: "region_{id}"
  - Agent移动时自动订阅/退订区域topic
  - 兴趣过滤: 仅订阅当前+邻区

- [x] 7.4 实现CRDT操作的序列化与广播
  - 文件: `crates/network/src/codec.rs`
  - CrdtOp → JSON → GossipSub publish
  - GossipSub receive → JSON → CrdtOp

- [ ] 7.5 多节点联机测试
  - 2个节点, 各跑2-3个Agent
  - 验证: 30秒内建立连接, 事件正确广播, Agent跨节点可见

## 8. CRDT状态同步 (sync crate)

实现自研CRDT数据结构和Merkle校验。

- [x] 8.1 实现LWW-Register
  - 文件: `crates/sync/src/lww.rs`
  - (value, timestamp, peer_id), 合并取max(timestamp)再max(peer_id)

- [x] 8.2 实现G-Counter
  - 文件: `crates/sync/src/gcounter.rs`
  - HashMap<PeerId, u64>各分量, 合并取各分量max, total求和

- [x] 8.3 实现OR-Set
  - 文件: `crates/sync/src/orset.rs`
  - 添加: (element, unique_tag), 删除: 移除已观察tag
  - 并发添加优先于未观察删除

- [x] 8.4 实现操作签名与验证
  - 文件: `crates/sync/src/signature.rs`
  - CrdtOp签名(ed25519), 接收方验签
  - 签名不匹配→拒绝+日志

- [x] 8.5 实现Merkle根校验
  - 文件: `crates/sync/src/merkle.rs`
  - 每100 tick生成世界Merkle根
  - 交换校验一致→确认同步, 不一致→触发差异区域全量同步

- [x] 8.6 实现SyncState合并与reconcile
  - 文件: `crates/sync/src/state.rs`
  - 应用CRDT操作到本地状态
  - 批量合并: 网络中断后重连的差量同步

## 9. Godot客户端 (bridge + client)

实现GDExtension桥接、2D渲染和引导面板。

- [x] 9.1 实现SimulationBridge GDExtension节点
  - 文件: `crates/bridge/src/lib.rs`
  - #[derive(GodotClass)], ready()启动Tokio, physics_process()poll channel
  - mpsc channel: WorldSnapshot(Sim→Godot), SimCommand(Godot→Sim)

- [x] 9.2 实现WorldSnapshot序列化与反序列化
  - 文件: `crates/bridge/src/snapshot.rs`
  - WorldSnapshot/AgentSnapshot/CellChange/NarrativeEvent
  - Godot侧读取更新视图

- [x] 9.3 创建Godot主场景与节点树
  - 文件: `client/scenes/main.tscn`
  - SimulationBridge(Autoload) + Camera2D + WorldView + RightPanel + NarrativeFeed + TopBar

- [x] 9.4 实现TileMap世界渲染
  - 文件: `client/scenes/world_view.tscn`, `client/scripts/world_renderer.gd`
  - TileMapLayer渲染256×256地图, 地形→Tile映射
  - 按区域chunk按需加载(仅渲染视口+缓冲区)
  - 资源/结构/遗迹叠加层

- [x] 9.5 实现Agent Sprite动态管理
  - 文件: `client/scripts/agent_manager.gd`
  - 根据WorldSnapshot动态创建/删除Sprite2D+Label
  - 位置平滑插值动画
  - 点击选择→右侧面板详情

- [x] 9.6 实现动机雷达图
  - 文件: `client/scripts/motivation_radar.gd`
  - CanvasItem自定义绘制6维雷达图
  - 每tick随WorldSnapshot刷新

- [x] 9.7 实现叙事流面板
  - 文件: `client/scenes/narrative_feed.tscn`
  - RichTextLabel滚动显示, 颜色编码(动作=白/交易=绿/攻击=红/压力=黄/遗产=紫)
  - 新事件自动滚底

- [x] 9.8 实现玩家引导面板
  - 文件: `client/scenes/guide_panel.tscn`
  - 6×HSlider调整动机权重, 调用SimulationBridge.adjust_motivation()
  - 偏好按钮(建议探索/交易/建造), 注入临时偏好

- [x] 9.9 实现摄像机控制
  - 拖拽平移, 滚轮缩放(0.5x-3x)
  - 双击Agent聚焦

## 10. 遗产系统 (legacy-system)

实现Agent死亡→遗迹→回响→契约→广播闭环。

- [x] 10.1 实现死亡判定与Legacy生成
  - 文件: `crates/core/src/legacy.rs`
  - 触发: 生命≤0 或 年龄≥200 tick
  - 生成墓地遗迹 + 物品散落 + 回响日志 + 未竟契约

- [x] 10.2 实现回响日志生成
  - 文件: `crates/core/src/legacy.rs`
  - 取最后3条短期记忆, LLM压缩为回响摘要+情感标签
  - 附加至遗迹实体

- [x] 10.3 实现遗产GossipSub广播
  - 遗产事件通过GossipSub广播全网
  - 其他Agent感知→认知/传承动机激励→可能触发探索遗迹

- [x] 10.4 实现遗迹交互
  - 其他Agent可到达遗迹格执行祭拜/探索
  - 读取回响日志, 认知/传承动机+微增
  - 拾取散落物品

- [x] 10.5 实现散落物品衰减
  - 无人拾取50 tick后每tick衰减10%
  - 直至消失

## 11. 存储与持久化

实现SQLite存储和世界状态持久化。

- [x] 11.1 初始化SQLite表结构
  - 文件: `crates/core/src/storage/schema.rs`
  - 建表: agents, inventory, short_term_memory, mid_term_memory, long_term_memory, event_log, map_cells, legacies, relations

- [x] 11.2 实现Agent状态CRUD
  - 文件: `crates/core/src/storage/agent_store.rs`
  - save/load agent, update position/motivation/health/inventory

- [x] 11.3 实现记忆CRUD
  - 文件: `crates/core/src/storage/memory_store.rs`
  - 写入短期/中期/长期记忆, 检索长期记忆, 衰减清理

- [x] 11.4 实现地图持久化
  - 文件: `crates/core/src/storage/map_store.rs`
  - save/load map_cells, 批量更新资源/结构状态

- [x] 11.5 实现世界快照保存与恢复
  - 文件: `crates/core/src/storage/world_store.rs`
  - 定期保存完整世界状态, 启动时恢复上次状态

## 12. 打包与分发

实现桌面端打包。

- [x] 12.1 配置Godot桌面导出预设
  - Win/macOS/Linux导出模板
  - Rust动态库嵌入PCK

- [x] 12.2 启动自动开窗逻辑
  - Godot启动后自动显示主界面
  - 无需用户手动操作

- [x] 12.3 WorldSeed分发配置
  - 默认WorldSeed.toml打包进PCK
  - 用户可修改配置文件定制世界

- [x] 12.4 多节点启动脚本
  - Shell脚本: 一键启动2+节点的本地测试环境

## 13. 测试与验证

- [x] 13.1 单元测试 - 动机引擎 (惯性衰减/缺口计算/事件微调)
- [x] 13.2 单元测试 - 决策管道 (硬约束过滤/规则校验/加权选择)
- [x] 13.3 单元测试 - CRDT (LWW/G-Counter/OR-Set合并正确性)
- [x] 13.4 单元测试 - JSON解析 (多层降级/边界情况)
- [x] 13.5 集成测试 - 单Agent决策循环 (API模式)
- [x] 13.6 集成测试 - 多Agent本地串行交互 (5 Agent × 30 min)
- [x] 13.7 集成测试 - 两节点P2P联机 (事件广播/CRDT同步)
- [x] 13.8 集成测试 - 遗产闭环 (死亡→遗迹→他人交互)
- [ ] 13.9 验收测试 - 桌面包分发给新用户运行
- [ ] 13.10 验收测试 - 3个"让人想看下去"的故事链涌现

## 任务依赖关系

```
1.x (项目骨架)
 ├──→ 2.x (动机引擎)
 │     └──→ 4.x (决策管道) ──→ 3.x (LLM层)
 │           │                       │
 │           └───────┬───────────────┘
 │                   ▼
 ├──→ 5.x (世界模型) ──→ 6.x (交互+记忆+策略)
 │                         │
 ├──→ 7.x (P2P网络)       │
 │     │                   │
 ├──→ 8.x (CRDT同步)      │
 │     │                   │
 │     └──→ 7+8联调 ←─────┘
 │              │
 │              ▼
 │         9.x (Godot客户端)
 │              │
 │         10.x (遗产系统)
 │              │
 │         11.x (存储持久化)
 │              │
 │         12.x (打包分发)
 │              │
 └──────────→ 13.x (测试验证)
```

## 建议实施顺序

| 阶段 | 周期 | 任务 | 说明 |
| --- | --- | --- | --- |
| Step 1 | W1 | 1.x + 2.x + 3.1~3.6 + 4.x | 核心骨架+动机引擎+LLM API+决策管道，单Agent命令行验证 |
| Step 2 | W2 | 5.x + 6.1~6.6 | 256×256世界+Agent交互基础 |
| Step 3 | W3-W4 | 6.7~6.10 | 三层记忆架构 (ChronicleStore + ChronicleDB + FTS5) |
| Step 4 | W5-W6 | 6.11~6.16 | 策略库系统 (StrategyHub + 自我改进闭环) |
| Step 5 | W7 | 6.17 + 7.x + 8.x | 多Agent测试 + P2P网络 + CRDT，两节点联机验证 |
| Step 6 | W8-W9 | 9.x | Godot GDExtension+2D客户端，可视化观察 |
| Step 7 | W10-W11 | 10.x + 3.7 + 11.x | 遗产系统+本地GGUF推理+存储持久化 |
| Step 8 | W12 | 12.x + 13.x | 打包分发+全量测试验证 |

## 文件结构总览

```
agentora/
├── Cargo.toml                          # workspace定义
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs                # 共享类型
│   │       ├── motivation.rs           # 动机引擎
│   │       ├── decision.rs             # 决策管道
│   │       ├── rule_engine.rs          # 规则引擎
│   │       ├── prompt.rs               # Prompt构建
│   │       ├── agent/
│   │       │   ├── mod.rs
│   │       │   ├── movement.rs
│   │       │   ├── inventory.rs
│   │       │   ├── trade.rs
│   │       │   ├── dialogue.rs
│   │       │   ├── combat.rs
│   │       │   └── alliance.rs
│   │       ├── memory/                 # 三层记忆架构
│   │       │   ├── mod.rs
│   │       │   ├── chronicle_store.rs  # ChronicleStore (冻结快照)
│   │       │   ├── chronicle_db.rs     # ChronicleDB (SQLite+FTS5)
│   │       │   ├── fence.rs            # 记忆围栏保护
│   │       │   ├── token_budget.rs     # 记忆总量控制
│   │       │   └── short_term.rs       # 短期记忆缓存
│   │       ├── strategy/               # 策略库系统
│   │       │   ├── mod.rs
│   │       │   ├── create.rs           # 策略创建触发
│   │       │   ├── patch.rs            # 策略自我改进
│   │       │   ├── decay.rs            # 策略衰减机制
│   │       │   ├── retrieve.rs         # 策略检索应用
│   │       │   └── motivation_link.rs  # 策略-动机联动
│   │       ├── world/
│   │       │   ├── map.rs
│   │       │   ├── region.rs
│   │       │   ├── resource.rs
│   │       │   ├── pressure.rs
│   │       │   ├── structure.rs
│   │       │   └── generator.rs
│   │       ├── legacy.rs
│   │       ├── seed.rs
│   │       └── storage/
│   │           ├── schema.rs
│   │           ├── agent_store.rs
│   │           ├── memory_store.rs     # ChronicleDB持久化
│   │           ├── strategy_store.rs   # 策略库持久化
│   │           ├── map_store.rs
│   │           └── world_store.rs
│   ├── ai/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs             # LlmProvider trait
│   │       ├── types.rs                # 请求/响应类型
│   │       ├── openai.rs               # OpenAI兼容Provider
│   │       ├── anthropic.rs            # Anthropic Provider
│   │       ├── local.rs                # GGUF本地Provider
│   │       ├── parser.rs               # JSON兼容解析
│   │       ├── fallback.rs             # 降级链
│   │       └── rule_engine.rs          # 规则引擎兜底
│   ├── network/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── transport.rs            # Transport抽象
│   │       ├── libp2p_transport.rs     # rust-libp2p实现
│   │       ├── gossip.rs               # GossipSub区域订阅
│   │       └── codec.rs                # 序列化
│   ├── sync/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── lww.rs                  # LWW-Register
│   │       ├── gcounter.rs             # G-Counter
│   │       ├── orset.rs                # OR-Set
│   │       ├── signature.rs            # 操作签名
│   │       ├── merkle.rs               # Merkle校验
│   │       └── state.rs                # SyncState合并
│   └── bridge/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  # GDExtension入口
│           └── snapshot.rs             # WorldSnapshot
├── client/                             # Godot 4项目
│   ├── project.godot
│   ├── .gdextension
│   ├── scenes/
│   │   ├── main.tscn
│   │   ├── world_view.tscn
│   │   └── narrative_feed.tscn
│   ├── scripts/
│   │   ├── world_renderer.gd
│   │   ├── agent_manager.gd
│   │   ├── motivation_radar.gd
│   │   └── guide_panel.gd
│   └── assets/
│       ├── sprites/
│       └── tiles/
├── worldseeds/
│   └── default.toml                    # 默认世界种子
└── tests/
    ├── single_agent.rs
    ├── multi_agent.rs
    ├── memory_system.rs                # 记忆系统测试
    ├── strategy_system.rs              # 策略库测试
    └── multi_node.rs
```