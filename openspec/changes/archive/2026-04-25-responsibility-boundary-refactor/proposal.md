# 需求说明书

## 背景概述

Agentora 项目经过 MVP 开发和多轮迭代后，代码量快速增长，但模块职责边界出现模糊和重叠问题。当前架构存在以下核心问题：

1. **World 模块职责过载**：World 结构体承担了地图管理、Agent管理、资源管理、建筑管理、Tick推进、动作执行、叙事事件、交易处理、压力系统、遗产系统、里程碑检查等 12+ 个职责，apply_action() 单方法超过 110 行，是典型的"上帝对象"。

2. **decision.rs 与 prompt.rs 边界模糊**：build_perception_summary() 和 build_path_recommendation() 位于 decision.rs（共 700+ 行），但应该属于 PromptBuilder 或独立的感知构建模块。

3. **WorldState 与 World 数据重复**：WorldState 作为 World 的快照需要手动构建，字段重复，每次决策都要完整构建，维护成本高。

4. **agent_loop.rs 职责链过长**：单 async fn 承担获取状态、构建感知、决策、应用动作、记录记忆、提取叙事、构建 Delta、发送事件等 10+ 个职责。

5. **Action 反馈逻辑分散**：ActionResult 格式各 handler 自定义字符串，generate_action_feedback() 在 world/mod.rs 解析，反馈格式无统一 Schema。

6. **Bridge 与 Simulation 边界不清晰**：Bridge 创建 tokio runtime，Simulation 内部也用 tokio；命令通道在两者间传递；命名混乱。

7. **前端信号处理冗余**：三个组件独立监听 world_updated，各自维护状态字典；没有统一的状态管理器；_map_size 在多个组件中重复。

8. **Delta 类型重复定义**：simulation/delta.rs 的 AgentDelta 与 snapshot.rs 的 WorldDelta 定义几乎相同的变体，字段命名不一致（position vs x/y），需考虑未来 P2P 模式的统一需求。

本次变更为架构重构，不涉及新功能开发，但需要保证现有功能正常运行。

## 变更目标

- **目标1**：拆分 World 模块，将上帝对象分解为多个专职子系统，每个子系统职责单一、边界清晰
- **目标2**：明确 decision.rs 与 prompt.rs 职责边界，将感知构建逻辑移至 PromptBuilder
- **目标3**：统一 WorldState 构建方式，消除与 World 的数据重复，提供统一的状态视图
- **目标4**：拆分 agent_loop.rs，将决策循环分解为独立的构建器、执行器、发射器
- **目标5**：统一 Action 反馈生成，建立标准化的反馈 Schema 和生成流程
- **目标6**：明确 Bridge 与 Simulation 编排边界，Bridge 只负责前端-后端桥接
- **目标7**：统一前端信号处理，建立单一状态管理器，消除冗余监听
- **目标8**：统一 Delta 类型定义，为未来 P2P 去中心化模式预留扩展接口

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `world-subsystems` | World 拆分后的子系统模块：WorldMap、WorldAgents、WorldResources、WorldStructures、ActionExecutor |
| `perception-builder` | 独立的感知构建模块，从 WorldState 构建 Prompt 所需的感知摘要 |
| `state-view` | 统一的状态视图模块，提供 World 到 WorldState 的标准化转换 |
| `action-feedback-schema` | Action 反馈的标准化 Schema，定义 SuccessDetail 和 BlockedReason 的结构化格式 |
| `frontend-state-manager` | 前端统一状态管理器（GDScript Autoload），接管所有 snapshot/delta 状态分发 |
| `unified-delta` | 统一的 Delta 类型定义，支持本地渲染和 P2P 网络广播两种模式 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `decision-pipeline` | 决策管道职责收缩，感知构建逻辑移至 PerceptionBuilder |
| `prompt-builder` | Prompt 构建器职责扩展，接管感知摘要和路径推荐构建 |
| `simulation` | Simulation 编排层职责明确，不再承担前端桥接逻辑 |
| `bridge` | Bridge 职责收缩为纯前端桥接，删除 tokio runtime 创建逻辑 |
| `agent-loop` | Agent 循环拆分为多阶段流水线，每个阶段职责单一 |

## 影响范围

- **代码模块**：
  - `crates/core/src/world/mod.rs` → 拆分为 5+ 子模块
  - `crates/core/src/decision.rs` → 职责收缩
  - `crates/core/src/prompt.rs` → 职责扩展
  - `crates/core/src/simulation/agent_loop.rs` → 拆分为多模块
  - `crates/core/src/simulation/delta.rs` → 统一定义
  - `crates/core/src/snapshot.rs` → 删除 WorldDelta，使用统一 Delta
  - `crates/bridge/src/bridge.rs` → 职责收缩
  - `client/scripts/*.gd` → 引入 StateManager Autoload

- **API接口**：无新增 API，但内部模块接口重构

- **依赖组件**：无新增外部依赖

- **关联系统**：为未来 P2P 网络模块预留 Delta 广播接口

## 验收标准

- [ ] World 模块拆分后，mod.rs 行数 < 300，各子系统职责单一且有独立文件
- [ ] decision.rs 行数 < 500，build_perception_summary() 和 build_path_recommendation() 移至 PromptBuilder
- [ ] WorldState 从 World 自动构建，无需 agent_loop.rs 手动组装 80+ 行代码
- [ ] agent_loop.rs 拆分为 WorldStateBuilder、ActionApplier、DeltaEmitter 三个阶段，单函数 < 100 行
- [ ] Action 反馈使用结构化 Schema（而非字符串），generate_action_feedback() 使用 Schema 解析
- [ ] Bridge 不创建 tokio runtime，只调用 Simulation API 和发射 Godot 信号
- [ ] 前端有单一 StateManager Autoload，world_updated 只分发一次，各组件订阅 StateManager
- [ ] Delta 类型统一为单一 AgentDelta，删除 snapshot.rs 中的 WorldDelta
- [ ] 所有单元测试通过（cargo test）
- [ ] 客户端正常运行，功能无退化