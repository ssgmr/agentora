## Why

策略系统当前各个模块（create/patch/decay/retrieve）已有框架实现，但未形成完整的自我改进闭环：策略创建后未保存到文件系统，Patch 修正未实际执行，衰减机制未集成到 tick 循环，策略检索结果未注入决策 Prompt。这导致 Agent 无法从历史成功决策中学习，无法验证"策略自我改进"的设计假设。

## What Changes

- **新增** 策略文件持久化（Markdown + YAML frontmatter 格式）
- **新增** 策略创建触发集成（成功决策后自动创建策略文件）
- **新增** 策略 Patch 执行工具（find/replace 实际修改文件内容）
- **新增** 策略衰减 tick 集成（每 50 tick 自动衰减）
- **新增** 策略检索注入 Prompt（`<strategy-context>`围栏）
- **新增** 策略与动机联动（策略成功/失败调整动机向量）

## Capabilities

### New Capabilities

- `strategy-persistence`: 策略文件持久化，Markdown+YAML frontmatter 格式，目录结构管理
- `strategy-create-trigger`: 策略创建触发集成，成功决策后自动创建策略文件
- `strategy-patch-executor`: 策略 Patch 执行工具，实际修改文件内容并记录日志
- `strategy-decay-tick`: 策略衰减 tick 集成，每 50 tick 自动衰减未使用策略
- `strategy-retrieve-inject`: 策略检索注入 Prompt，`<strategy-context>` 围栏保护
- `strategy-motivation-link`: 策略与动机联动，策略成功/失败调整动机向量

### Modified Capabilities

无

## Impact

- **affected crates**: `core` (strategy 模块), `decision` (Prompt 构建)
- **dependencies**: `serde_yaml` (YAML frontmatter 解析), `std::fs` (文件操作)
- **breaking changes**: 无，当前策略系统为框架实现
- **integration points**: DecisionPipeline 需调用策略检索；World::apply_action 需触发策略创建/更新
