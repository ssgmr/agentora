# 实施任务清单

## 1. 资源修复

清理损坏的导入缓存，确保纹理资源正确加载。这是最紧急的修复，因为当前启动就报错。

- [x] 1.1 清理 `.godot/imported/` 缓存并验证 Godot 重新导入后无报错
  - 操作: `rm -rf client/.godot/imported/`
  - 打开 Godot 编辑器触发重新导入
  - 验证启动日志无 `Failed loading resource` 错误

- [x] 1.2 运行 `svg_to_png.py` 确保所有 SVG 源文件正确导出为 PNG
  - 文件: `client/assets/svg_to_png.py`
  - 确保 `agent.svg` 被纳入导出列表（32x32）
  - 验证所有 PNG 文件存在且格式合法

- [x] 1.3 删除废弃的 `client/scripts/simulation_bridge.gd`
  - 场景 `main.tscn` 中的 `SimulationBridge` 节点使用 Rust GDExtension 类型，与该脚本无关

## 2. Rust Bridge 架构重构

将 World-driven 顺序 tick 改为 Agent 独立心跳 + 事件驱动推送。

- [x] 2.1 定义 `AgentDelta` 枚举类型
  - 文件: `crates/bridge/src/lib.rs`
  - 类型: `AgentMoved`, `AgentDied`, `AgentSpawned`
  - 包含 agent_id、位置、健康值、动机向量等字段

- [x] 2.2 改造 `SimulationBridge` 为双通道架构
  - 文件: `crates/bridge/src/lib.rs`
  - 新增 `delta_receiver: Option<Receiver<AgentDelta>>`
  - 新增 `agent_delta` 信号
  - `physics_process` 中优先处理 delta 事件，再处理 snapshot

- [x] 2.3 将 `World` 改造为 `Arc<Mutex<World>>` 支持并发访问
  - 文件: `crates/bridge/src/lib.rs`
  - 决策 task 只读 World 快照，Apply 循环独占 World

- [x] 2.4 实现 `run_agent_loop` — Agent 独立决策循环
  - 文件: `crates/bridge/src/lib.rs`
  - 每个 Agent 在独立的 `tokio::spawn` task 中运行
  - 决策完成后通过 channel 发送动作到 Apply 循环
  - 支持可配置的决策间隔（玩家 Agent 默认 2s，NPC 默认 1s）

- [x] 2.5 实现 Apply 循环 — 串行应用动作并发 delta
  - 文件: `crates/bridge/src/lib.rs`
  - 从 action channel 接收 `(AgentId, Action)`
  - 独占锁 apply_action 后构造 AgentDelta 发送至 delta 通道

- [x] 2.6 实现定期 snapshot 兜底循环
  - 文件: `crates/bridge/src/lib.rs`
  - 每 5 秒生成完整 WorldSnapshot 发送至 snapshot 通道
  - 不影响 delta 实时推送

- [x] 2.7 添加 NPC 快速决策支持
  - 文件: `crates/bridge/src/lib.rs`
  - NPC 跳过 LLM，直接用规则引擎决策
  - NPC 数量可配置（`NpcConfig` 结构体）

- [x] 2.8 重构主循环 `run_simulation_async`
  - 文件: `crates/bridge/src/lib.rs`
  - 替换现有 `for agent in agents` 顺序循环
  - 启动 Agent 独立 task + Apply 循环 + snapshot 兜底

## 3. Godot 客户端增量渲染

- [x] 3.1 修改 `agent_manager.gd` 支持 delta 增量更新
  - 文件: `client/scripts/agent_manager.gd`
  - 新增 `_on_agent_delta(delta_data: Dictionary)` 方法
  - 处理 `agent_moved`、`agent_died`、`agent_spawned` 事件类型
  - 收到 delta 后立即更新对应 sprite，不等待其他 Agent

- [x] 3.2 修改 `agent_manager.gd` 的 snapshot 处理为一致性校验
  - 文件: `client/scripts/agent_manager.gd`
  - `_on_world_updated` 改为只检查"缺失的 Agent 创建"和"幽灵 Agent 删除"
  - 不再做全量遍历更新

- [x] 3.3 连接 `agent_delta` 信号到 `agent_manager`
  - 文件: `client/scripts/agent_manager.gd` `_ready()` 方法
  - 新增 `bridge.agent_delta.connect(_on_agent_delta)`

- [x] 3.4 限制每帧 delta 处理数量防止卡顿
  - 文件: `client/scripts/agent_manager.gd`
  - `physics_process` 中每帧最多处理 100 个 delta，剩余留给下一帧

## 4. 测试与验证

- [x] 4.1 编译 GDExtension 并验证无编译错误
  - 命令: `cargo bridge`
  - 确认 `client/bin/agentora_bridge.dll` 更新

- [x] 4.2 Godot 启动验证
  - 确认无纹理加载错误
  - 确认 Agent sprite 正确显示
  - 确认 Agent 移动时实时更新（不等 snapshot）

- [x] 4.3 一致性校验验证
  - 删除 auto_screenshot 的 autoload 配置
  - 手动运行观察：snapshot 到达时不应重建已有的 Agent
  - 确认 snapshot 能修复丢失的 Agent（一致性兜底）

- [x] 4.4 NPC 验证
  - 配置 NPC 数量 > 0
  - 确认 NPC 快速决策不阻塞玩家 Agent
  - 确认 NPC 移动在 Godot 中实时可见

## 任务依赖关系

```
1.x (资源修复) ──独立，无依赖──▶ 可最先执行

2.x (Rust重构) ──顺序依赖──▶ 2.1→2.2→2.3→2.4→2.5→2.6→2.7→2.8

3.x (Godot渲染) ──依赖 2.x──▶ 等待 delta 通道就绪后开发

4.x (测试验证) ──依赖 2.x + 3.x──▶ 所有代码完成后验证
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.x | 修复纹理资源，确保 Godot 启动无报错（紧急修复） |
| 阶段二 | 2.1-2.3 | 定义数据类型 + 双通道 + World 并发改造（基础架构） |
| 阶段三 | 2.4-2.8 | Agent 独立循环 + Apply 循环 + 主循环重构（核心逻辑） |
| 阶段四 | 3.x | Godot 端 delta 增量渲染（客户端适配） |
| 阶段五 | 4.x | 编译 + 运行 + 一致性校验 + NPC 验证 |

## 文件结构总览

```
修改文件:
├── crates/bridge/src/lib.rs        # 核心重构：双通道 + Agent 独立循环
├── client/scripts/agent_manager.gd # 增量渲染 + 一致性校验
└── client/project.godot            # 临时 autoload 配置（验证后删除）

新增文件:
├── client/scripts/auto_screenshot.gd # 验证用（已存在，无需新建）

删除文件:
├── client/scripts/simulation_bridge.gd  # 废弃占位文件
└── client/scripts/auto_screenshot.gd    # 验证完成后删除 autoload 配置
```
