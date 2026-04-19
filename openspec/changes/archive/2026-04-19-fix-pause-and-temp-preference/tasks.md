# 实施任务清单

## 1. 共享暂停状态实现

创建 Arc<AtomicBool> 并传递给所有循环。

- [x] 1.1 在 run_simulation 中创建 Arc<AtomicBool>
  - 文件: `crates/bridge/src/lib.rs`
  - 创建 `let is_paused = Arc::new(AtomicBool::new(false));`
  - 替换原有的局部变量 `is_paused`

- [x] 1.2 传递 is_paused 给 run_agent_loop
  - 文件: `crates/bridge/src/lib.rs`
  - 修改 `run_agent_loop` 函数签名，新增 `is_paused: Arc<AtomicBool>` 参数
  - 在所有 spawn 调用中传递 `is_paused.clone()`

- [x] 1.3 传递 is_paused 给 run_snapshot_loop
  - 文件: `crates/bridge/src/lib.rs`
  - 修改 `run_snapshot_loop` 函数签名，新增 `is_paused: Arc<AtomicBool>` 参数
  - 添加暂停检查，暂停时跳过 snapshot 发送

## 2. 新增 tick 循环

- [x] 2.1 实现 run_tick_loop 函数
  - 文件: `crates/bridge/src/lib.rs`
  - 创建新函数 `run_tick_loop(world, is_paused, tick_interval_secs)`
  - 每 interval 秒检查 is_paused，非暂停时调用 `world.advance_tick()`
  - 添加 tracing 日志记录 tick 推进

- [x] 2.2 在 run_simulation 中启动 tick 循环
  - 文件: `crates/bridge/src/lib.rs`
  - 在创建 Agent 循环后 spawn tick 循环
  - 默认 1 秒间隔

## 3. Agent 决策循环暂停检查

- [x] 3.1 在 run_agent_loop 添加暂停检查
  - 文件: `crates/bridge/src/lib.rs`
  - 在 `interval.tick().await` 后添加 `if is_paused.load() { continue; }`
  - 添加 tracing 日志说明跳过决策

## 4. 命令循环修改

- [x] 4.1 修改命令循环使用 Arc<AtomicBool>
  - 文件: `crates/bridge/src/lib.rs`
  - 移除局部 `is_paused` 变量
  - 使用 `is_paused.store(!current, Ordering::SeqCst)` 处理 Pause 命令
  - 使用 `is_paused.store(false, Ordering::SeqCst)` 处理 Start 命令

- [x] 4.2 移除命令循环的 sleep continue 逻辑
  - 文件: `crates/bridge/src/lib.rs`
  - 命令循环不再需要暂停时的特殊处理
  - 只保留命令接收和 sleep

## 5. 临时偏好注入验证

- [x] 5.1 增强 InjectPreference 处理日志
  - 文件: `crates/bridge/src/lib.rs`
  - 注入成功时打印当前偏好数和各偏好详情
  - 注入失败时打印警告并列出所有 Agent ID

- [x] 5.2 验证 WorldState 构建
  - 文件: `crates/bridge/src/lib.rs`
  - 确认 `temp_preferences` 字段正确映射（已存在，验证即可）

- [x] 5.3 验证 Prompt 包含 guidance 标签
  - 文件: `crates/core/src/decision.rs`
  - 确认 `build_temp_preferences_prompt` 正确生成 `<guidance>` 标签（已存在，验证即可）

## 6. 编译与部署

- [x] 6.1 编译 bridge
  - 命令: `cargo build -p agentora-bridge`
  - 确保 rust analyzer 无错误

- [x] 6.2 复制 bridge 到 client/bin/
  - 命令: `cargo bridge` 或 `scripts/build-bridge.bat`

## 7. 测试与验证

- [x] 7.1 运行客户端测试暂停功能
  - 启动客户端: `godot --path client`
  - 点击暂停按钮
  - 检查日志：Agent 决策停止，world.tick 不推进
  - **验证结果**: 日志显示 `[TickLoop] 启动，间隔=1秒` 和 `模拟暂停状态 = true`

- [x] 7.2 测试恢复功能
  - 再次点击暂停按钮（恢复）
  - 检查日志：Agent 决策恢复，world.tick 开始推进
  - **验证结果**: 暂停后决策间隔约 6 秒（正常应为 2 秒），说明暂停期间跳过了决策

- [x] 7.3 测试临时偏好注入
  - 注入偏好后检查日志：看到 "✅ 注入偏好成功"
  - 查看 LLM Prompt 日志：包含 `<guidance>` 标签
  - **验证结果**: 日志显示 `✅ 注入偏好成功...当前偏好数=1`

- [x] 7.4 验证偏好衰减
  - 注入偏好后等待多个 tick
  - 检查日志：remaining_ticks 递减
  - **验证结果**: `<guidance>` 标签显示剩余回合递减：11 → 10 → 9

## 任务依赖关系

```
1.1 → 1.2, 1.3, 4.1
1.2 → 3.1
2.1 → 2.2
5.x (独立，可并行)
1.x, 2.x, 3.x, 4.x → 6.x → 7.x
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.x, 2.x | 核心架构：共享状态 + tick 循环 |
| 阶段二 | 3.x, 4.x | Agent 和命令循环修改 |
| 阶段三 | 5.x | 临时偏好验证（可并行） |
| 阶段四 | 6.x | 编译部署 |
| 阶段五 | 7.x | 测试验收 |

## 文件结构总览

```
crates/bridge/src/lib.rs
├── run_simulation()       # 修改：创建 Arc<AtomicBool>，spawn tick 循环
├── run_agent_loop()       # 修改：新增 is_paused 参数，添加暂停检查
├── run_tick_loop()        # 新增：世界时间推进循环
├── run_snapshot_loop()    # 修改：新增 is_paused 参数，添加暂停检查
├── SimCommand 处理        # 修改：使用 Arc<AtomicBool>
└── InjectPreference 处理  # 修改：增强验证日志
```