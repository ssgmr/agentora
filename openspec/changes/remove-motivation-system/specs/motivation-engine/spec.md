# 动机引擎 — 从运行时移除

## REMOVED Requirements

### Requirement: 6维动机向量定义

**原因**：六维动机是设计理念而非运行时变量。LLM 本身能直接理解状态值并做出决策，不需要动机向量作为中间层。
**迁移方案**：
- 移除 `MotivationVector` 结构体及其所有方法（decay/apply_delta/compute_gap 等）
- 移除 `Agent.motivation` 字段
- 移除 `Action.motivation_delta` 字段
- 移除 `ActionCandidate.motivation_delta` 字段
- 六维动机（生存与资源/社会与关系/认知与好奇/表达与创造/权力与影响/意义与传承）仅作为设计文档中的理念框架保留

### Requirement: 惯性衰减机制

**原因**：动机向量已移除，衰减机制不再需要。
**迁移方案**：移除 `World.advance_tick()` 中的 `agent.motivation.decay()` 调用。

### Requirement: 事件驱动动机微调

**原因**：动机向量已移除，不再需要事件驱动的微调机制。
**迁移方案**：
- 移除 `World.apply_action()` 中的动机 delta 应用逻辑
- 移除 `Action.motivation_delta` 的使用
- 动作对 Agent 状态的影响直接通过世界系统修改（如 Eat → satiety +30）

### Requirement: 动机缺口计算（Spark）

**原因**：Spark 系统已移除。LLM 直接从状态值感知需求，不需要缺口计算。
**迁移方案**：
- 移除 `Spark` 和 `SparkType` 结构体
- 移除 `Spark.from_gap()` 方法
- 决策管道不再需要 Spark 输入

### Requirement: 人格种子影响

**原因**：人格种子保留，但不再影响动机向量。人格种子将编译进角色配置 System Prompt。
**迁移方案**：
- `PersonalitySeed` 保留但仅作为 AgentProfile 的一部分
- 人格值不再作为运行时影响决策的变量
- 人格值在角色创建时描述为自然语言（如"你是一个好奇心强的人"）

### Requirement: 有效动机计算

**原因**：`effective_motivation()` 方法混合了状态值和动机值，是两套系统混乱的根源。
**迁移方案**：移除 `Agent.effective_motivation()` 方法。状态值对决策的影响改为直接出现在 Prompt 中。
