# 功能规格说明：决策管道完整实现

## ADDED Requirements

### Requirement: 五阶段决策管道

系统 SHALL 对每个 Agent 每 tick 执行五阶段决策管道：硬约束过滤 → 上下文构建 → LLM 生成 → 规则校验 → 动机加权选择。

#### Scenario: 完整决策流程

- **WHEN** 一个 tick 开始
- **THEN** 系统 SHALL 按顺序执行：1) 硬约束过滤 2) 上下文构建 3)LLM 生成 4) 规则校验 5) 动机加权选择
- **AND** 每阶段 SHALL 将结果传递给下一阶段
- **AND** 任一阶段失败 SHALL 降级到规则引擎兜底

### Requirement: 硬约束过滤器

系统 SHALL 在 LLM 生成前过滤掉物理上不可能的动作：超出移动范围、资源不足、目标不存在等。

#### Scenario: 资源不足过滤

- **WHEN** Agent 尝试建造但材料不足
- **THEN** 建造动作 SHALL 被硬约束过滤排除
- **AND** 建造动作 SHALL 不在候选列表中

#### Scenario: 移动范围过滤

- **WHEN** Agent 尝试移动到距离>1 格的目标
- **THEN** 该移动动作 SHALL 被过滤排除

#### Scenario: 目标不存在过滤

- **WHEN** Agent 尝试与不存在的 Agent 交互
- **THEN** 该交互动作 SHALL 被过滤排除

#### Scenario: 地形通行性过滤

- **WHEN** Agent 尝试移动到不可通行地形（水域/山脉）
- **THEN** 该移动动作 SHALL 被过滤排除

### Requirement: 上下文构建器

系统 SHALL 将 Agent 的感知信息组装为 LLM Prompt，总 token 数 SHALL 不超过 2500。

#### Scenario: Prompt 组成结构

- **WHEN** 构建决策 Prompt
- **THEN** Prompt SHALL 包含：动机向量 + Spark + 记忆摘要 + 感知摘要 + 策略提示
- **AND** 各部分 SHALL 使用围栏标签包裹（`<chronicle-context>`, `<current-spark>`, `<strategy-context>`）

#### Scenario: Prompt 大小控制

- **WHEN** 记忆内容过长导致总 token>2500
- **THEN** 系统 SHALL 执行截断：先截断策略提示，再截断记忆摘要
- **AND** 动机向量和 Spark SHALL 始终保留

#### Scenario: 视野范围限定

- **WHEN** Agent 周围有多个其他 Agent
- **THEN** Prompt SHALL 仅包含视野半径（5 格）内的 Agent 信息
- **AND** 超出视野的 Agent SHALL 不出现在上下文中

### Requirement: LLM 候选生成

系统 SHALL 将构建的 Prompt 发送给 LLM，请求生成结构化 JSON 格式的候选动作列表。

#### Scenario: 成功生成候选

- **WHEN** LLM 正常返回 JSON
- **THEN** 系统 SHALL 解析为候选动作列表
- **AND** 每个候选 SHALL 包含：reasoning, action_type, target, params, motivation_delta

#### Scenario: JSON 解析失败降级

- **WHEN** LLM 返回的文本无法解析为合法 JSON
- **THEN** 系统 SHALL 尝试 Layer 2：提取第一个{...}块
- **AND** 若 Layer 2 失败，SHALL 尝试 Layer 3：修复常见错误（尾逗号/单引号/注释）
- **AND** 若全部失败，SHALL 降级为规则引擎兜底决策

#### Scenario: Provider 降级链

- **WHEN** 主 Provider（OpenAI）调用失败（超时/429/5xx）
- **THEN** 系统 SHALL 尝试备用 Provider（Anthropic）
- **AND** 若备用 Provider 也失败，SHALL 尝试本地 GGUF（若可用）
- **AND** 全部失败 SHALL 降级为规则引擎兜底

### Requirement: 规则校验器

系统 SHALL 对 LLM 生成的候选动作执行规则校验：类型合法性、参数范围、动作前置条件满足。

#### Scenario: 合法动作通过

- **WHEN** 候选动作类型已知且参数合法
- **THEN** 动作 SHALL 通过规则校验
- **AND** 动作 SHALL 进入动机加权选择阶段

#### Scenario: 未知动作类型拒绝

- **WHEN** 候选动作的 action_type 字段不在已知动作枚举中
- **THEN** 系统 SHALL 拒绝该候选
- **AND** 该候选 SHALL 不进入动机加权选择

#### Scenario: 交易参数不合法

- **WHEN** 交易提议中 offer 或 want 包含 Agent 不具备的资源
- **THEN** 系统 SHALL 拒绝该候选

#### Scenario: 攻击目标无效

- **WHEN** 攻击动作的目标不存在或距离>1 格
- **THEN** 系统 SHALL 拒绝该候选

### Requirement: 动机加权选择器

系统 SHALL 从通过校验的候选动作中，基于动机向量加权选择最终执行动作。

#### Scenario: 唯一候选直接选择

- **WHEN** 仅有一个候选通过校验
- **THEN** 该候选 SHALL 成为最终动作
- **AND** 不执行加权计算

#### Scenario: 多候选加权选择

- **WHEN** 有 N 个候选（N>1）通过校验
- **THEN** 系统 SHALL 计算每个候选的得分：`score = dot_product(candidate.motivation_delta, agent.motivation)`
- **AND** 系统 SHALL 使用 softmax+temperature 选择最终动作
- **AND** temperature SHALL=0.1（保留少量随机性）

#### Scenario: 无候选通过校验

- **WHEN** 所有候选均未通过规则校验
- **THEN** 系统 SHALL 执行规则引擎的默认安全动作
- **AND** 默认动作优先级：向最近资源移动 > 原地等待

### Requirement: ActionCandidate 结构体

系统 SHALL 定义统一的 ActionCandidate 结构体，承载 LLM 生成和规则引擎兜底的候选动作。

#### Scenario: 候选动作结构

- **WHEN** 定义 ActionCandidate
- **THEN** 结构体 SHALL 包含：
  - reasoning: String（决策理由）
  - action_type: ActionType（动作类型）
  - target: Option<String>（目标）
  - params: HashMap<String, Value>（参数）
  - motivation_delta: [f32; 6]（自评动机变化）
  - source: CandidateSource（来源：LLM 或 RuleEngine）

#### Scenario: 候选来源标记

- **WHEN** 创建候选动作
- **THEN** 系统 SHALL 标记来源（LLM / RuleEngine）
- **AND** 来源信息 SHALL 用于统计和调试

## MODIFIED Requirements

### Requirement: 规则引擎完整校验

原需求"规则引擎：硬约束过滤、规则校验、兜底决策"扩展为完整的校验逻辑。

#### Scenario: 资源检查

- **WHEN** 校验建造动作
- **THEN** 系统 SHALL 检查 Agent 背包是否有足够的建造材料
- **AND** 材料不足 SHALL 拒绝该动作

#### Scenario: 范围检查

- **WHEN** 校验交互动作（交易/攻击/对话）
- **THEN** 系统 SHALL 检查目标与 Agent 的距离是否≤1 格
- **AND** 超出范围 SHALL 拒绝该动作

#### Scenario: 目标存在性检查

- **WHEN** 校验交互动作
- **THEN** 系统 SHALL 检查目标 Agent/资源是否存在
- **AND** 目标不存在 SHALL 拒绝该动作

## REMOVED Requirements

无
