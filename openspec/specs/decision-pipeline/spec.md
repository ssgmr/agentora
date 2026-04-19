# Decision Pipeline

## Purpose

定义 Agent 的决策管道：上下文构建 → LLM 生成 → 规则校验 → 选择最终动作。

## Requirements

### Requirement: 决策管道阶段

系统 SHALL 对每个 Agent 每 tick 执行简化的决策管道：上下文构建 → LLM 生成 → 规则校验 → 选择最终动作。

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

系统 SHALL 将 Agent 的感知信息组装为 LLM Prompt，总 token 数 SHALL 不超过 2500。

#### Scenario: Prompt 组成结构

- **WHEN** 构建决策 Prompt
- **THEN** Prompt SHALL 包含：System Prompt（角色设定）+ 当前状态 + 感知摘要 + 记忆摘要 + 策略参考
- **AND** 各部分 SHALL 使用围栏标签包裹（`<chronicle-context>`, `<strategy-context>`）
- **AND** Prompt SHALL 包含 satiety/hydration 数值及状态标签（"饥饿中"/"口渴中"/"正常"）
- **AND** Prompt SHALL 不再包含动机向量表格
- **AND** Prompt SHALL 不再包含 Spark 信息
- **AND** Prompt SHALL 不再包含 satisfaction 数组

#### Scenario: Prompt 大小控制

- **WHEN** 记忆内容过长导致总 token > 2500
- **THEN** 系统 SHALL 执行截断：先截断策略提示，再截断记忆摘要，最后截断感知摘要
- **AND** 角色设定和当前状态 SHALL 始终保留

#### Scenario: 视野范围限定

- **WHEN** Agent 周围有多个其他 Agent
- **THEN** Prompt SHALL 仅包含视野半径（5 格）内的 Agent 信息
- **AND** 超出视野的 Agent SHALL 不出现在上下文中

#### Scenario: Prompt 注入生存状态

- **WHEN** Agent satiety ≤ 30
- **THEN** Prompt 包含"饱食度：{satiety}/100, 状态：饥饿中！需要寻找食物"

- **WHEN** Agent hydration ≤ 30
- **THEN** Prompt 包含"水分度：{hydration}/100, 状态：口渴中！需要寻找水源"

#### Scenario: Prompt 注入压力事件

- **WHEN** pressure_pool 中有干旱事件
- **THEN** Prompt 包含"当前世界事件：干旱来袭，水源产出减半"

### Requirement: LLM 候选生成

系统 SHALL 将构建的 Prompt 发送给 LLM，请求生成结构化 JSON 格式的动作。

#### Scenario: 成功生成候选

- **WHEN** LLM 正常返回 JSON
- **THEN** 系统 SHALL 解析为单个动作
- **AND** 动作 SHALL 包含：reasoning, action_type, target, params
- **AND** 动作 SHALL 不再包含 motivation_delta 字段

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

系统 SHALL 对 LLM 生成的候选动作执行规则校验。校验通过后直接执行，不再进入动机加权选择阶段。

#### Scenario: 合法动作通过

- **WHEN** 候选动作类型已知且参数合法
- **THEN** 动作 SHALL 通过规则校验
- **AND** 动作 SHALL 直接成为最终执行动作

#### Scenario: 未知动作类型拒绝

- **WHEN** 候选动作的 action_type 字段不在已知动作枚举中
- **THEN** 系统 SHALL 拒绝该候选

#### Scenario: 无候选通过校验

- **WHEN** 所有候选均未通过规则校验
- **THEN** 系统 SHALL 执行规则引擎的默认安全动作
- **AND** 默认动作优先级：向最近资源移动 > 原地等待

#### Scenario: 交易参数不合法

- **WHEN** 交易提议中 offer 或 want 包含 Agent 不具备的资源
- **THEN** 系统 SHALL 拒绝该候选

#### Scenario: 攻击目标无效

- **WHEN** 攻击动作的目标不存在或距离>1 格
- **THEN** 系统 SHALL 拒绝该候选

### Requirement: 规则引擎完整校验

系统 SHALL 实现完整的规则校验逻辑，包括资源检查、范围检查和目标存在性检查。

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
