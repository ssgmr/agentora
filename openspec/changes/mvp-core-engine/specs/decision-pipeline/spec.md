# 功能规格说明：决策管道

## ADDED Requirements

### Requirement: 五阶段决策管道

系统 SHALL 对每个Agent每tick执行五阶段决策管道：硬约束过滤→上下文构建→LLM生成→规则校验→动机加权选择。

#### Scenario: 完整决策流程

- **WHEN** 一个tick开始
- **THEN** 系统 SHALL 按顺序执行：1)硬约束过滤 2)上下文构建 3)LLM生成 4)规则校验 5)动机加权选择
- **AND** 每阶段 SHALL 将结果传递给下一阶段

### Requirement: 硬约束过滤

系统 SHALL 在LLM生成前过滤掉物理上不可能的动作：超出移动范围、资源不足、目标不存在等。

#### Scenario: 资源不足过滤

- **WHEN** Agent尝试建造但材料不足
- **THEN** 建造动作 SHALL 被硬约束过滤排除

#### Scenario: 移动范围过滤

- **WHEN** Agent尝试移动到距离>1格的目标
- **THEN** 该移动动作 SHALL 被过滤排除

#### Scenario: 目标不存在过滤

- **WHEN** Agent尝试与不存在的Agent交互
- **THEN** 该交互动作 SHALL 被过滤排除

### Requirement: 上下文构建

系统 SHALL 将Agent的感知信息组装为LLM Prompt，总token数 SHALL 不超过2500。包含：动机向量和当前Spark、压缩后记忆（≤1800 tokens）、视野内Agent和社会关系、世界区域摘要。

#### Scenario: Prompt大小控制

- **WHEN** Agent记忆内容过长
- **THEN** 系统 SHALL 执行记忆压缩，将Prompt总token数控制在2500以内

#### Scenario: 视野范围限定

- **WHEN** Agent周围有多个其他Agent
- **THEN** 系统 SHALL 仅包含视野半径（5格）内的Agent信息到Prompt中

### Requirement: LLM生成候选动作

系统 SHALL 将构建的Prompt发送给LLM，请求生成结构化JSON格式的候选动作列表。每个候选动作包含：reasoning（思考过程）、action（动作类型）、target（目标）、params（参数）、motivation_delta（自评估动机变化）。

#### Scenario: 成功生成候选

- **WHEN** LLM正常返回JSON
- **THEN** 系统 SHALL 解析为候选动作列表
- **AND** 每个候选 SHALL 包含 action/target/params 字段

#### Scenario: JSON解析失败降级

- **WHEN** LLM返回的文本无法解析为合法JSON
- **THEN** 系统 SHALL 尝试提取第一个{...}块重试
- **AND** 若仍失败，SHALL 修复常见错误（尾逗号/单引号）重试
- **AND** 若全部失败，SHALL 降级为规则引擎兜底决策

### Requirement: 规则校验

系统 SHALL 对LLM生成的候选动作执行规则校验：类型合法性、参数范围、动作前置条件满足。

#### Scenario: 合法动作通过

- **WHEN** 候选动作类型已知且参数合法
- **THEN** 动作 SHALL 通过规则校验

#### Scenario: 未知动作类型拒绝

- **WHEN** 候选动作的action字段不在已知动作列表中
- **THEN** 系统 SHALL 拒绝该候选

#### Scenario: 交易参数不合法

- **WHEN** 交易提议中offer或want包含Agent不具备的资源
- **THEN** 系统 SHALL 拒绝该候选

### Requirement: 动机加权选择

系统 SHALL 从通过校验的候选动作中，基于动机向量加权选择最终执行动作。权重计算：`score = dot_product(action.motivation_alignment, agent.motivation_vector)`。

#### Scenario: 唯一候选直接选择

- **WHEN** 仅有一个候选通过校验
- **THEN** 该候选 SHALL 成为最终动作

#### Scenario: 多候选加权选择

- **WHEN** 有多个候选通过校验
- **THEN** 系统 SHALL 计算每个候选与动机向量的对齐度得分
- **AND** 选择得分最高的候选（加少量随机性，temperature=0.1）

#### Scenario: 无候选通过校验

- **WHEN** 所有候选均未通过规则校验
- **THEN** 系统 SHALL 执行规则引擎的默认安全动作（原地等待或向最近资源移动）