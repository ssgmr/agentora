# 策略系统 — 移除动机关联

## MODIFIED Requirements

### Requirement: 策略库架构

系统 SHALL 为每个 Agent 维护决策策略库（StrategyHub），存储成功决策的可复用策略。策略库使用 Markdown + YAML frontmatter 格式。

STRATEGY.md YAML Frontmatter 必填字段：
- `spark_type`: 策略适用的 Spark 类型（改为 `state_pattern` 命名）
- `success_rate`: 成功率（0.0-1.0）
- `use_count`: 使用次数
- `last_used_tick`: 最后使用的 tick

#### Scenario: STRATEGY.md YAML Frontmatter

- **WHEN** 创建或更新 STRATEGY.md
- **THEN** 文件 SHALL 以 YAML frontmatter 开头（--- 包裹）
- **AND** frontmatter SHALL 包含必填字段：spark_type, success_rate, use_count, last_used_tick
- **AND** frontmatter SHALL 不再包含 motivation_delta 字段

### Requirement: 策略创建触发

系统 SHALL 在以下条件触发策略创建，将成功决策转化为可复用策略。

#### Scenario: 成功决策触发创建

- **WHEN** Agent 执行决策后反馈为"成功"
- **AND** 决策涉及 ≥3 个候选动作筛选（未来扩展场景）
- **THEN** 系统 SHALL 自动创建策略
- **AND** 策略名 SHALL 使用本次 Spark 类型（保留 spark_type 字段，因为策略按状态模式分类仍有意义）

#### Scenario: 策略的内容安全扫描

- **WHEN** 创建策略内容
- **THEN** 系统 SHALL 执行安全扫描（与 ChronicleStore 相同规则）
- **AND** 扫描威胁模式：prompt injection、role hijack、rule bypass、invisible unicode
- **AND** 检测到威胁 SHALL 拒绝创建

### Requirement: 策略衰减机制

系统 SHALL 对长期不适用或成功率下降的策略执行衰减。

#### Scenario: 策略成功/失败不再影响动机

- **WHEN** 策略执行成功
- **THEN** 系统 SHALL 更新 success_rate
- **AND** 系统 SHALL 不再修改 Agent 的动机向量
- **AND** 系统 SHALL 不再调用 motivation_link 模块

#### Scenario: 策略执行失败

- **WHEN** 策略执行失败
- **THEN** 系统 SHALL 更新 success_rate
- **AND** 系统 SHALL 不再反向调整动机向量

### Requirement: 策略检索与应用

系统 SHALL 在决策构建 Prompt 时检索匹配的策略。

#### Scenario: 策略内容注入 Prompt

- **WHEN** 策略匹配成功
- **THEN** 系统 SHALL 将策略内容注入 Prompt，用 `<strategy-context>` 标签包裹
- **AND** 不再计算候选与策略的动机对齐度
- **AND** 不再基于动机对齐度给予额外 boost

## REMOVED Requirements

### Requirement: 策略与动机向量联动

**原因**：动机系统已移除，策略执行结果不再需要影响动机向量。
**迁移方案**：
- 移除 `strategy/motivation_link.rs` 整个模块
- 移除 `Strategy` 结构体的 `motivation_delta` 字段
- 移除 `on_strategy_success()` 和 `on_strategy_failure()` 函数
- 移除策略创建时从 `Action.motivation_delta` 提取并记录到 frontmatter 的逻辑

### Requirement: 策略与候选动作对齐

**原因**：动机对齐度已不再计算。
**迁移方案**：移除策略检索时的动机对齐度计算和 +0.1 boost 逻辑。
