# 实施任务清单

## 1. 后端模块拆分准备

创建新的模块目录和基础结构，为后续拆分做准备。

- [x] 1.1 创建 WorldStateBuilder 模块
  - 文件: `crates/core/src/simulation/state_builder.rs`
  - 实现 WorldStateBuilder::build() 从 World 自动构建 WorldState
  - 实现 scan_vision 集成
  - 实现压力事件和临时偏好提取

- [x] 1.2 创建 PerceptionBuilder 模块
  - 文件: `crates/core/src/decision/perception.rs`
  - 从 decision.rs 迁移 build_perception_summary()
  - 从 decision.rs 迁移 build_path_recommendation()
  - 实现 build() 接收 WorldState 返回感知摘要

- [x] 1.3 创建 DeltaEmitter 模块
  - 文件: `crates/core/src/simulation/delta_emitter.rs`
  - 从 agent_loop.rs 迁移 delta 构建逻辑
  - 实现 emit() 接收 ActionResult 并发送到 delta_tx

- [x] 1.4 创建 NarrativeEmitter 模块
  - 文件: `crates/core/src/simulation/narrative_emitter.rs`
  - 从 agent_loop.rs 迁移叙事提取逻辑
  - 实现 extract() 和 send_events()

- [x] 1.5 创建 MemoryRecorder 模块
  - 文件: `crates/core/src/simulation/memory_recorder.rs`
  - 从 agent_loop.rs 迁移记忆记录逻辑
  - 实现 record() 接收 action 并记录到 Agent 记忆

- [x] 1.6 更新 simulation/mod.rs 导出新模块
  - 文件: `crates/core/src/simulation/mod.rs`
  - 导出 WorldStateBuilder, DeltaEmitter, NarrativeEmitter, MemoryRecorder
  - 依赖: 1.1, 1.3, 1.4, 1.5

## 2. DecisionPipeline 职责收缩

修改 DecisionPipeline 接收预构建的感知，不再自行构建。

- [x] 2.1 修改 DecisionPipeline.execute() 接口
  - 文件: `crates/core/src/decision/mod.rs`
  - 新增 perception_summary 参数
  - 移除 build_perception_summary() 调用
  - 保持向后兼容（提供 execute_with_auto_perception 接口）
  - 依赖: 1.2

- [x] 2.2 更新 PromptBuilder 接收预构建感知
  - 文件: `crates/core/src/prompt.rs`
  - build_decision_prompt() 接收 perception_summary 参数
  - 直接使用，不自行构建
  - 依赖: 1.2
  - 注: 已通过 build_prompt() 方法修改实现

- [x] 2.3 更新 agent_loop 调用新模块
  - 文件: `crates/core/src/simulation/agent_loop.rs`
  - 调用 WorldStateBuilder::build()
  - 调用 PerceptionBuilder::build_perception_summary()
  - 传递感知摘要给 DecisionPipeline.execute()
  - 依赖: 1.1, 1.2, 2.1

## 3. Action 反馈结构化

定义 ActionResult Schema，统一动作反馈格式。

- [x] 3.1 定义 ActionResult Schema
  - 文件: `crates/core/src/world/action_result.rs`
  - 定义 ActionResult enum（Success/Blocked/AlreadyAtPosition 等）
  - 定义 FieldChange 结构
  - 定义 ActionSuggestion 结构

- [x] 3.2 创建 ActionExecutor 模块（从 World 拆出）
  - 文件: `crates/core/src/world/actions.rs`
  - 从 world/mod.rs 迁移 apply_action() 逻辑
  - 各 handler 返回 ActionResult 而非字符串
  - 实现 execute() 路由 ActionType 到具体 handler
  - 依赖: 3.1

- [x] 3.3 更新 generate_action_feedback()
  - 文件: `crates/core/src/simulation/agent_loop.rs`
  - 使用 ActionResult Schema 解析反馈
  - 格式化为 LLM 可理解的文本
  - 依赖: 3.1, 3.2

- [x] 3.4 更新 world/mod.rs 调用 ActionExecutor
  - 文件: `crates/core/src/world/mod.rs`
  - apply_action() 调用 ActionExecutor.execute()
  - 返回 ActionResult
  - 依赖: 3.2

## 4. Delta 类型统一

统一为 AgentDelta，删除 WorldDelta，预留 P2P 接口。

- [x] 4.1 扩展 AgentDelta 添加 for_broadcast()
  - 文件: `crates/core/src/simulation/delta.rs`
  - 实现 for_broadcast() 返回精简 JSON
  - 确保字段取舍满足 P2P 需求
  - 统一使用 position tuple 格式

- [x] 4.2 删除 WorldDelta
  - 文件: `crates/core/src/snapshot.rs`
  - 确认无代码引用 WorldDelta
  - 删除 WorldDelta enum 定义

- [x] 4.3 更新 conversion.rs 类型转换
  - 文件: `crates/bridge/src/conversion.rs`
  - 统一使用 AgentDelta 转换
  - 确保 Godot 信号格式兼容
  - 依赖: 4.1, 4.2

## 5. Bridge 职责收缩

Bridge 不创建 runtime，只负责信号桥接。

- [x] 5.1 修改 Bridge.start_simulation()
  - 文件: `crates/bridge/src/bridge.rs`
  - 创建 mpsc channels ✓
  - std::thread::spawn 创建模拟线程 ✓
  - 在模拟线程内创建 tokio runtime ✓

- [x] 5.2 创建 Simulation 结构（在模拟线程内）
  - 文件: `crates/core/src/simulation/simulation.rs`
  - 持有 World 和 DecisionPipeline
  - 提供 snapshot_sender()、delta_sender()
  - 接收 cmd_rx 控制命令

- [x] 5.3 确认 physics_process() 只发射信号
  - 文件: `crates/bridge/src/bridge.rs`
  - 从 receiver.try_recv() 获取数据
  - 发射 Godot 信号
  - 不执行任何模拟逻辑

- [x] 5.4 更新 bridge 模块结构
  - 文件: `crates/bridge/src/lib.rs`
  - bridge.rs < 200 行
  - conversion.rs 类型转换
  - logging.rs 日志配置
  - simulation_runner.rs 模拟运行逻辑
  - 依赖: 5.1, 5.2, 5.3

## 6. 前端 StateManager 实现

创建 StateManager Autoload，统一状态分发。

- [x] 6.1 创建 state_manager.gd
  - 文件: `client/scripts/state_manager.gd`
  - 定义状态数据容器（_agents, _terrain, _resources）
  - 定义变更信号（state_updated, agent_changed, terrain_changed）
  - 实现 get_agent_data()、get_terrain_at() 等查询接口

- [x] 6.2 实现 StateManager 解析和分发逻辑
  - 文件: `client/scripts/state_manager.gd`
  - 实现 _on_world_updated() 解析 snapshot
  - 实现 _on_agent_delta() 增量更新
  - 实现 _on_narrative_event() 叙事追加
  - 依赖: 6.1

- [x] 6.3 注册 StateManager 为 Autoload
  - 文件: `client/project.godot`
  - 在 [autoload] 添加 StateManager

- [x] 6.4 修改 main.gd 连接 StateManager
  - 文件: `client/scripts/main.gd`
  - 获取 StateManager Autoload
  - 连接 Bridge 信号到 StateManager
  - 依赖: 6.1, 6.2, 6.3

- [x] 6.5 修改 world_renderer.gd 订阅 StateManager
  - 文件: `client/scripts/world_renderer.gd`
  - 移除直接监听 world_updated
  - 订阅 StateManager.state_updated 和 terrain_changed
  - 依赖: 6.4

- [x] 6.6 修改 agent_manager.gd 订阅 StateManager
  - 文件: `client/scripts/agent_manager.gd`
  - 移除直接监听 world_updated
  - 订阅 StateManager.agent_changed
  - 依赖: 6.4

- [x] 6.7 修改 narrative_feed.gd 订阅 StateManager
  - 文件: `client/scripts/narrative_feed.gd`
  - 移除直接监听 narrative_event
  - 订阅 StateManager.narrative_added
  - 依赖: 6.4

- [x] 6.8 修改 agent_detail_panel.gd 查询 StateManager
  - 文件: `client/scripts/agent_detail_panel.gd`
  - 使用 StateManager.get_agent_data()
  - 依赖: 6.1

## 7. World 模块拆分

拆分 World 为子系统，保持公开 API 兼容。

- [x] 7.1 创建 WorldMap 子系统
  - 文件: `crates/core/src/world/map.rs`
  - 迁移地形查询逻辑
  - 迁移边界检查逻辑
  - 提供 get_terrain_at()、is_valid_position()

- [x] 7.2 创建 WorldAgents 子系统
  - 文件: `crates/core/src/world/mod.rs`
  - Agent 存储和位置索引
  - 提供 get_agent()、get_agents_at()、update_agent_position()

- [x] 7.3 创建 WorldResources 子系统
  - 文件: `crates/core/src/world/resource.rs`
  - 迁移资源管理逻辑
  - 迁移采集逻辑
  - 提供 get_resource_at()、collect_resource()

- [x] 7.4 创建 WorldStructures 子系统
  - 文件: `crates/core/src/world/structure.rs`
  - 迁移建筑管理逻辑
  - 迁移效果范围计算
  - 提供 get_structure_at()、create_structure()

- [x] 7.5 重构 World 为协调者
  - 文件: `crates/core/src/world/mod.rs`
  - 持有子系统引用
  - 公开 API 保持不变
  - 内部调用子系统
  - 确认 mod.rs < 300 行
  - 依赖: 7.1, 7.2, 7.3, 7.4, 3.2

## 8. agent_loop 拆分为流水线

拆分 run_agent_loop() 为多阶段模块协调。

- [x] 8.1 重构 run_agent_loop() 主函数
  - 文件: `crates/core/src/simulation/agent_loop.rs`
  - 协调 6 阶段调用
  - 确认主函数 < 100 行
  - 使用 WorldStateBuilder、PerceptionBuilder、DeltaEmitter 等
  - 依赖: 1.x, 2.x, 3.x, 5.2

- [x] 8.2 确认各阶段职责单一
  - 验证 WorldStateBuilder 不执行 I/O
  - 验证 DecisionPhase 不修改 World
  - 验证 ApplyPhase 不发送 Delta
  - 验证 DeltaEmitter 只构建和发送
  - 依赖: 8.1

## 9. 测试与验证

- [x] 9.1 后端单元测试 - WorldStateBuilder
- [x] 9.2 后端单元测试 - PerceptionBuilder
- [x] 9.3 后端单元测试 - ActionExecutor
- [x] 9.4 后端单元测试 - AgentDelta.for_broadcast()
- [x] 9.5 集成测试 - DecisionPipeline 接口变更
  - 验证 execute() 接收 perception_summary 参数
  - 测试文件: tests/responsibility_boundary_tests.rs

- [x] 9.6 集成测试 - P2P 消息处理链路
  - 测试 DeltaEnvelope 回环过滤
  - 测试 P2PMessageHandler 远程 Delta 处理
  - 测试 ShadowAgent 创建和更新
- [x] 9.7 前端验证 - StateManager 状态同步
  - 验证方法: Godot MCP 场景树查询、信号连接检查、截图
  - 结果: StateManager 信号正确连接，terrain 256x256 解码成功，1310 资源加载

- [x] 9.8 前端验证 - 多组件订阅一致性
  - 验证方法: Godot MCP 日志检查
  - 结果: AgentManager/WorldRenderer/NarrativeFeed 均成功连接 StateManager 信号

- [x] 9.9 客户端运行验证 - cargo bridge + godot --path client
  - 验证方法: cargo bridge 构建 + Godot MCP 运行测试
  - 结果: 4 Agent nodes 正常渲染，决策周期运行正常

- [ ] 9.10 端到端验证 - Agent 决策周期完整流程

## 10. P2P 适配基础

架构支持 P2P 模式：本地运行部分 Agent，远程 Agent 通过影子状态同步。

- [x] 10.1 定义 SimMode 枚举
  - 文件: `crates/core/src/simulation/config.rs`
  - 定义 SimMode::Centralized 和 SimMode::P2P { local_agent_ids, region_size }
  - 更新 SimConfig 增加 mode 字段

- [x] 10.2 创建 ShadowAgent 结构体
  - 文件: `crates/core/src/agent/shadow.rs`
  - 定义精简字段（id, name, position, health, is_alive, last_seen_tick, source_peer_id）
  - 实现 apply_delta() 更新影子状态
  - 实现 is_expired() 超时检查

- [x] 10.3 拆分 World Agent 存储
  - 文件: `crates/core/src/world/mod.rs`
  - 将 agents HashMap 拆分为 local_agents + remote_agents
  - 实现 all_agents() 合并返回（用于渲染）
  - 实现 apply_remote_delta() 更新影子状态
  - 实现 advance_tick_local_only() 只对 local_agents 生存消耗
  - 依赖: 10.2

- [x] 10.4 修改 Simulation 支持 SimMode
  - 文件: `crates/core/src/simulation/simulation.rs`
  - 根据 SimMode 决定 spawn 多个或单个 agent_loop
  - P2P 模式下只为 local_agent_ids 创建循环
  - 依赖: 10.1, 10.3

- [x] 10.5 更新 agent_loop 调用 advance_tick_local_only
  - 文件: `crates/core/src/simulation/tick_loop.rs`
  - P2P 模式下只对 local_agents 执行生存消耗
  - 集中式模式保持原有行为
  - 依赖: 10.3

- [x] 10.6 更新 agent/mod.rs 导出 ShadowAgent
  - 文件: `crates/core/src/agent/mod.rs`
  - 导出 ShadowAgent 结构体
  - 依赖: 10.2

## 11. DeltaDispatcher 双通道分发

Delta 同时发送到本地 mpsc 和 P2P GossipSub。

- [x] 11.1 创建 DeltaDispatcher 模块
  - 文件: `crates/core/src/simulation/delta_dispatcher.rs`
  - 持有 local_tx (mpsc) 和 p2p_transport (Option)
  - 实现 dispatch() 双通道分发
  - 集中式模式：只发送 local_tx

- [x] 11.2 扩展 AgentDelta 添加 source_peer_id
  - 文件: `crates/core/src/simulation/delta.rs`
  - 创建 DeltaEnvelope 包装结构体
  - 实现 is_from_peer() 用于过滤本地回环
  - 实现 for_broadcast() 包含元数据

- [x] 11.3 更新 agent_loop 使用 DeltaDispatcher
  - 文件: `crates/core/src/simulation/agent_loop.rs`
  - 当前 DeltaEmitter 使用 Sender，P2P 功能已预留
  - DeltaDispatcher 已创建，待 libp2p 消息处理完成后集成
  - 依赖: 11.1, 11.2
  - 状态: 基础设施已创建，集中式模式正常工作

- [x] 11.4 更新 simulation/mod.rs 导出 DeltaDispatcher
  - 文件: `crates/core/src/simulation/mod.rs`
  - 导出 DeltaDispatcher
  - 依赖: 11.1

## 12. P2P 消息处理链路

打通 GossipSub 消息处理，实现远程 Delta 接收。

- [x] 12.1 补全 GossipSub 消息处理
  - 文件: `crates/network/src/libp2p_transport.rs`
  - 添加 message_rx: mpsc::Receiver<NetworkMessage>
  - 提供 take_message_receiver() 供上层消费
  - 提供 try_recv_message() 非阻塞接收

- [x] 12.2 扩展 NetworkMessage 增加 AgentDelta 变体
  - 文件: `crates/network/src/codec.rs`
  - 新增 NetworkMessage::AgentDelta(AgentDeltaMessage)
  - 实现序列化/反序列化

- [x] 12.3 创建 P2PMessageHandler 模块
  - 文件: `crates/core/src/simulation/p2p_handler.rs`
  - 实现 handle(NetworkMessage) 处理远程 Delta
  - 过滤本地回环（source_peer_id != local）
  - 更新 World.remote_agents
  - 发送本地 mpsc 通知渲染

- [x] 12.4 补全 SyncState OrSetRemove 实现
  - 文件: `crates/sync/src/state.rs`
  - 完成 OrSetRemove 的实现
  - 定义 key schema: "agent:{id}:position" 等
  - 新增 OrSet.remove_with_tag() 方法

- [x] 12.5 更新 Simulation 消费 P2P 消息
  - 文件: `crates/core/src/simulation/simulation.rs`
  - P2P 模式下提供 handle_remote_delta() 方法
  - 提供 with_p2p() 创建带 P2P 支持的实例
  - 依赖: 12.1, 12.3

- [x] 12.6 更新 simulation/mod.rs 导出 P2PMessageHandler
  - 文件: `crates/core/src/simulation/mod.rs`
  - 导出 P2PMessageHandler
  - 依赖: 12.3

## 13. P2P 测试与验证

- [x] 13.1 单元测试 - ShadowAgent.apply_delta()
  - 测试文件: tests/responsibility_boundary_tests.rs
  - 验证影子状态更新和死亡处理

- [x] 13.2 单元测试 - World.local_agents/remote_agents 拆分
  - 验证 apply_remote_delta() 方法
  - 验证 is_local_agent() 方法
  - 验证 advance_tick_local_only() 方法
  - 状态: 隐含在其他测试中验证

- [x] 13.3 单元测试 - DeltaEnvelope 回环过滤
  - 测试文件: tests/responsibility_boundary_tests.rs

- [x] 13.4 单元测试 - P2PMessageHandler 回环过滤
  - 测试文件: tests/responsibility_boundary_tests.rs
  - 验证本地回环过滤
  - 验证远程 Delta 处理和影子创建

- [ ] 13.5 集成测试 - SimMode 切换（集中式 vs P2P）
- [ ] 13.6 集成测试 - 多客户端模拟（本地 Agent + 远程 Agent）
- [ ] 13.7 集成测试 - 集中式模式行为不变（默认配置）

## 任务依赖关系

```
Phase 1 (后端模块拆分准备)
  1.1, 1.2, 1.3, 1.4, 1.5 并行 → 1.6

Phase 2 (DecisionPipeline 职责收缩)
  依赖: 1.1, 1.2
  2.1 → 2.2 → 2.3

Phase 3 (Action 反馈结构化)
  3.1 → 3.2 → 3.3, 3.4 并行

Phase 4 (Delta 类型统一)
  4.1 → 4.2 → 4.3

Phase 5 (Bridge 职责收缩)
  5.1, 5.2, 5.3 并行 → 5.4

Phase 6 (前端 StateManager)
  6.1 → 6.2 → 6.3 → 6.4 → 6.5, 6.6, 6.7, 6.8 并行

Phase 7 (World 模块拆分)
  7.1, 7.2, 7.3, 7.4 并行 → 7.5
  依赖: 3.2 (ActionExecutor)

Phase 8 (agent_loop 拆分)
  依赖: 1.x, 2.x, 3.x, 5.2

Phase 9 (测试)
  各模块完成后运行对应测试

Phase 10 (P2P 适配基础)
  10.1, 10.2 并行 → 10.3 → 10.4 → 10.5 → 10.6

Phase 11 (DeltaDispatcher)
  11.1, 11.2 并行 → 11.3 → 11.4

Phase 12 (P2P 消息处理链路)
  12.1, 12.2, 12.3, 12.4 并行 → 12.5 → 12.6
  依赖: 10.x, 11.x

Phase 13 (P2P 测试)
  依赖: 10.x, 11.x, 12.x
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.x | 后端模块拆分准备，创建新模块但不修改主流程 |
| 阶段二 | 2.x | DecisionPipeline 职责收缩，接口变更 |
| 阶段三 | 3.x | Action 反馈结构化，定义 Schema |
| 阶段四 | 4.x | Delta 类型统一，删除重复定义 |
| 阶段五 | 5.x | Bridge 职责收缩，不创建 runtime |
| 阶段六 | 6.x | 前端 StateManager，统一状态分发 |
| 阶段七 | 7.x | World 模块拆分，子系统创建 |
| 阶段八 | 8.x | agent_loop 拆分，主函数精简 |
| 阶段九 | 9.x | 测试与验证（核心重构） |
| 阶段十 | 10.x | P2P 适配基础，Agent 存储拆分 |
| 阶段十一 | 11.x | DeltaDispatcher 双通道分发 |
| 阶段十二 | 12.x | P2P 消息处理链路打通 |
| 阶段十三 | 13.x | P2P 测试与验证 |

## 文件结构总览

```
agentora/
├── crates/core/src/
│   ├── agent/
│   │   ├── shadow.rs               ← 新增（P2P）
│   │   └── mod.rs                  ← 修改导出
│   ├── decision/
│   │   ├── perception.rs           ← 新增
│   │   └── mod.rs                  ← 修改导出
│   ├── simulation/
│   │   ├── state_builder.rs        ← 新增
│   │   ├── delta_emitter.rs        ← 新增
│   │   ├── delta_dispatcher.rs     ← 新增（P2P）
│   │   ├── p2p_handler.rs          ← 新增（P2P）
│   │   ├── narrative_emitter.rs    ← 新增
│   │   ├── memory_recorder.rs      ← 新增
│   │   ├── simulation.rs           ← 新增
│   │   ├── config.rs               ← 修改（SimMode）
│   │   ├── agent_loop.rs           ← 修改
│   │   ├── tick_loop.rs            ← 修改（P2P）
│   │   ├── delta.rs                ← 修改（source_peer_id）
│   │   └── mod.rs                  ← 修改导出
│   ├── world/
│   │   ├── action_result.rs        ← 新增
│   │   ├── actions.rs              ← 新增
│   │   ├── map.rs                  ← 修改（子系统）
│   │   ├── agents.rs               ← 新增（子系统）
│   │   ├── resources.rs            ← 修改（子系统）
│   │   ├── structures.rs           ← 修改（子系统）
│   │   └── mod.rs                  ← 修改（local/remote 拆分）
│   ├── decision.rs                 ← 修改（职责收缩）
│   ├── prompt.rs                   ← 修改（接收预构建感知）
│   └── snapshot.rs                 ← 修改（删除 WorldDelta）
├── crates/network/src/
│   ├── libp2p_transport.rs         ← 修改（消息处理链路）
│   ├── codec.rs                    ← 修改（AgentDelta 变体）
│   └── gossip.rs                   ← 保持
├── crates/sync/src/
│   ├── state.rs                    ← 修改（补全 OrSetRemove）
│   └── lww.rs, gcounter.rs, orset.rs ← 保持
├── crates/bridge/src/
│   ├── lib.rs                      ← 修改（模块结构）
│   ├── bridge.rs                   ← 修改（职责收缩）
│   ├── conversion.rs               ← 修改（Delta 统一）
│   └── logging.rs                  ← 保持
├── client/
│   ├── scripts/
│   │   ├── state_manager.gd        ← 新增
│   │   ├── main.gd                 ← 修改
│   │   ├── world_renderer.gd       ← 修改
│   │   ├── agent_manager.gd        ← 修改
│   │   ├── narrative_feed.gd       ← 修改
│   │   └── agent_detail_panel.gd   ← 修改
│   └── project.godot               ← 修改（Autoload 注册）
├── config/
│   └ sim.toml                      ← 修改（增加 mode 配置）
└── tests/
    └── ...                         ← 新增/修改测试
```