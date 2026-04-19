# 决策管道 — 移除动机系统

## MODIFIED Requirements

### Requirement: 决策管道阶段

系统 SHALL 对每个 Agent 执行简化的决策管道：上下文构建 → LLM 生成 → 规则校验 → 选择最终动作。移除原有的五阶段管道中的"硬约束过滤"和"动机加权选择"阶段。

#### Scenario: 完整决策流程

- **WHEN** 一个 tick 开始
- **THEN** 系统 SHALL 按顺序执行：1) 上下文构建 2) LLM 生成 3) 规则校验 4) 选择最终动作
- **AND** 规则校验失败时，SHALL 不执行动作，记录错误反馈供 LLM 下回合修正
- **AND** LLM 不可用时，SHALL 降级到规则引擎兜底

#### Scenario: LLM 输出为最终动作

- **WHEN** LLM 返回合法动作并通过规则校验
- **THEN** 该动作 SHALL 直接成为最终执行动作
- **AND** 系统 SHALL 不再执行加权或随机选择
- **AND** 系统 SHALL 不再计算点积或 softmax

#### Scenario: 多候选简化选择

- **WHEN** LLM 返回多个候选动作（未来扩展）
- **THEN** 系统 SHALL 按顺序选择第一个通过校验的候选
- **AND** 或 SHALL 按 LLM 提供的 confidence 分数选择最高分

### Requirement: 上下文构建器

系统 SHALL 将 Agent 的感知信息组装为 LLM Prompt，不再包含动机向量和 Spark 信息。Prompt SHALL 包含：System Prompt（角色设定）+ 当前状态 + 感知摘要 + 记忆摘要 + 策略参考。

#### Scenario: Prompt 组成结构

- **WHEN** 构建决策 Prompt
- **THEN** Prompt SHALL 包含：
  - System Prompt（角色设定/性格描述）
  - 当前状态（health/satiety/hydration/inventory）
  - 感知摘要（附近资源/Agent/建筑）
  - 记忆摘要（短期记忆 + 编年史）
  - 策略参考（成功策略回顾）
- **AND** Prompt SHALL 不再包含动机向量表格
- **AND** Prompt SHALL 不再包含 Spark 信息
- **AND** Prompt SHALL 不再包含 satisfaction 数组

#### Scenario: Prompt 大小控制

- **WHEN** 记忆内容过长导致总 token > 2500
- **THEN** 系统 SHALL 执行截断：先截断策略提示，再截断记忆摘要，最后截断感知摘要
- **AND** 角色设定和当前状态 SHALL 始终保留

### Requirement: LLM 候选生成

系统 SHALL 将构建的 Prompt 发送给 LLM，请求生成结构化 JSON 格式的动作。动作 SHALL 不再包含 motivation_delta 字段。

#### Scenario: 成功生成候选

- **WHEN** LLM 正常返回 JSON
- **THEN** 系统 SHALL 解析为单个动作
- **AND** 动作 SHALL 包含：reasoning, action_type, target, params
- **AND** 动作 SHALL 不再包含 motivation_delta 字段

### Requirement: 规则校验器

系统 SHALL 对 LLM 生成的候选动作执行规则校验。校验通过后直接执行，不再进入动机加权选择阶段。

#### Scenario: 合法动作通过

- **WHEN** 候选动作类型已知且参数合法
- **THEN** 动作 SHALL 通过规则校验
- **AND** 动作 SHALL 直接成为最终执行动作

#### Scenario: 无候选通过校验

- **WHEN** LLM 返回的动作未通过规则校验
- **THEN** 系统 SHALL 不执行任何动作
- **AND** 系统 SHALL 记录校验失败原因到 last_action_result
- **AND** 下次决策 Prompt SHALL 包含此错误反馈

### Requirement: ActionCandidate 结构体

系统 SHALL 简化 ActionCandidate 结构体，移除 motivation_delta 和 source 字段。

#### Scenario: 候选动作结构

- **WHEN** 定义 ActionCandidate
- **THEN** 结构体 SHALL 包含：
  - reasoning: String（决策理由）
  - action_type: ActionType（动作类型）
  - target: Option<String>（目标）
  - params: HashMap<String, Value>（参数）
- **AND** 结构体 SHALL 不再包含 motivation_delta
- **AND** 结构体 SHALL 不再包含 source

### Requirement: LLM 失败兜底

系统 SHALL 在 LLM Provider 不可用（未配置/超时/失败）时，使用规则引擎提供合理的默认动作。

#### Scenario: LLM 不可用时的生存兜底

- **WHEN** LLM Provider 不可用
- **AND** Agent 的 satiety 或 hydration 低于 30
- **THEN** 系统 SHALL 优先选择进食/饮水动作
- **AND** 若背包没有食物/水源，SHALL 选择向最近资源移动
- **AND** 若无资源可寻，SHALL 选择 Wait

#### Scenario: LLM 不可用时的默认兜底

- **WHEN** LLM Provider 不可用
- **AND** Agent 状态正常
- **THEN** 系统 SHALL 选择 Wait 作为默认动作
- **AND** 系统 SHALL 记录降级事件到日志

## REMOVED Requirements

### Requirement: 动机加权选择器

**原因**：动机系统已移除，不再需要基于动机向量的加权选择。LLM 直接输出的动作即为最终决策。
**迁移方案**：原加权选择逻辑替换为直接选择 LLM 输出的第一个通过校验的动作。

### Requirement: Spark 缺口计算

**原因**：Spark 系统已移除。LLM 直接从状态值理解 Agent 需求，不需要 Spark 提示"最需要什么"。
**迁移方案**：原 Spark 触发的决策场景由 LLM 自主判断状态值替代。
