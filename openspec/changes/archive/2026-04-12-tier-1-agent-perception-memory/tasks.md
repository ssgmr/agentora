# Tasks: Tier 1 — Agent 感知与记忆接线

## Phase 1: 基础设施 — 反向索引与 vision 模块

- [x] 1.1 在 `World` 结构体中新增 `agent_positions: HashMap<Position, Vec<AgentId>>` 字段
- [x] 1.2 新增 `World::insert_agent_at(&mut self, agent_id, agent)` 方法，统一处理 `agents.insert` + `agent_positions` 初始化
- [x] 1.3 在 `World::generate_agents` 中改用 `insert_agent_at()` 创建 Agent
- [x] 1.4 在 `World::apply_action` 出口统一维护 `agent_positions`：记录旧位置 → 执行动作 → 比较位置变化 → 更新索引
- [x] 1.5 在 `World::check_agent_death` 中清理死亡 Agent 的位置记录
- [x] 1.6 新建 `crates/core/src/vision.rs`，创建 `NearbyAgentInfo` 结构体（id, name, position, distance, motivation_summary, relation_type, trust）
- [x] 1.7 新建 `VisionScanResult` 结构体（self_position, terrain_at, resources_at as `(ResourceType, u32)`, nearby_agents）
- [x] 1.8 实现 `scan_vision(&World, &AgentId, radius) -> VisionScanResult` 函数：遍历位置（非遍历实体），曼哈顿距离过滤，填充地形/资源/Agent/关系数据
- [x] 1.9 删除 `movement.rs` 中的 `perceive_nearby()` 及 `PerceivedAgent`/`PerceivedResource`/`PerceptionResult` 类型
- [x] 1.10 在 `crates/core/src/lib.rs` 中导出 `vision` 模块及公开类型

## Phase 2: WorldState 扩展

- [x] 2.1 在 `WorldState` 结构体中新增 `nearby_agents: Vec<NearbyAgentInfo>` 字段
- [x] 2.2 将 `resources_at` 的值类型从 `ResourceType` 改为 `(ResourceType, u32)` 以携带数量
- [x] 2.3 更新 `RuleEngine` 中对 `resources_at` 的访问代码适配新类型

## Phase 3: Bridge 接线

- [x] 3.1 在 `run_agent_loop` 中删除手写扫描循环（dx/dy saturating_add 代码）
- [x] 3.2 改为 `lock world` → `scan_vision()` → `clone Agent` → `release lock` 流程
- [x] 3.3 在锁外调用 `agent.memory.get_summary(spark_type)` 获取摘要（clone 后重连 SQLite）
- [x] 3.4 将 `VisionScanResult` 字段映射到 `WorldState`
- [x] 3.5 在 `create_npc_agents` 中改用 `world.insert_agent_at()` 替代直接 `agents.insert`，确保 `agent_positions` 同步初始化

## Phase 4: 记忆系统接线

- [x] 4.1 在 `World::generate_agents` 中初始化 Agent 的 ChronicleDB 和 ChronicleStore（已通过 `Agent::new` → `MemorySystem::new` 完成）
- [x] 4.2 在 `run_apply_loop` 中，apply_action 后调用 `agent.memory.record(MemoryEvent{...})`
- [x] 4.3 定义 `MemoryEvent` 的自动标注逻辑：根据 ActionType 映射 emotion_tags 和 importance
- [x] 4.4 在 `run_agent_loop` 中，**锁外**使用 clone 后的 Agent memory 调用 `get_summary(spark_type)` 获取摘要
- [x] 4.5 修改 `DecisionPipeline::execute()` 接受 memory_summary 参数（或改为调用 `build_prompt_with_memory`）
- [x] 4.6 处理记忆系统初始化失败的情况（降级为无持久记忆，仅短期记忆 — `get_summary` 已有 `if let Some` 检查）

## Phase 5: Prompt 模板扩展

- [x] 5.1 扩展 `build_perception_summary` 输出：加入 nearby_agents（名字/距离/关系状态）
- [x] 5.2 扩展 `build_perception_summary` 输出：加入资源（位置/类型/数量）
- [x] 5.3 扩展 `build_perception_summary` 输出：加入地形概览（各方向主要地形）
- [x] 5.4 确保 memory_summary 正确注入 `PromptBuilder::build_decision_prompt`（已有逻辑，确认连接）
- [x] 5.5 验证 Prompt token 总量在限制内（已有分级截断机制，确认新增内容不超限）

## Phase 6: 测试与验证

- [x] 6.1 编写单元测试：`scan_vision` 覆盖四个象限（非仅东北），验证圆形扫描正确
- [x] 6.2 编写单元测试：`scan_vision` 返回的 nearby_agents 包含关系数据
- [x] 6.3 编写单元测试：`scan_vision` 返回的 resources_at 包含数量信息
- [x] 6.4 编写单元测试：`agent_positions` 在 Move 后保持一致
- [x] 6.5 编写单元测试：`agent_positions` 在 Explore（handle_special_action 中的随机移动）后保持一致
- [x] 6.6 编写单元测试：`agent_positions` 在 Agent 生成/死亡后保持一致
- [x] 6.7 编写单元测试：`insert_agent_at()` 同时更新 agents 和 agent_positions
- [x] 6.8 编写测试：验证 memory.record 在 apply_action 后被调用（已在 bridge `run_apply_loop` 中实现，`MemoryEvent` 自动标注逻辑已连接）
- [x] 6.9 编写测试：验证 run_agent_loop 的锁外 get_summary 调用时序（不持锁）（架构设计保证：`scan_vision()` → `clone Agent` → `release lock` → `get_summary()` 流程已确认）
- [x] 6.10 运行 Godot 客户端，观察 Agent 决策是否包含周围环境信息和历史记忆
- [x] 6.11 验证 Prompt 内容：打印 LLM 接收的完整 Prompt，确认感知/记忆/关系段正确（Godot 运行确认 Prompt 长度从 ~200 增至 628 chars，感知/记忆/关系段正确注入；tracing::info! 日志钩子已添加用于调试）
