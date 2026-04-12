# Agent交互系统

## Purpose

定义Agent间交互的核心机制，包括移动、资源采集、交易、对话、攻击、结盟和视野感知。

## Requirements

### Requirement: 移动

Agent SHALL 每tick可移动至相邻1格（上下左右4方向），受地形通行性约束。

#### Scenario: 正常移动

- **WHEN** Agent执行移动到相邻可通行格
- **THEN** Agent坐标 SHALL 更新至目标格
- **AND** Agent感知范围 SHALL 更新

#### Scenario: 移动至不可通行格

- **WHEN** Agent尝试移动至山地或水域
- **THEN** 移动 SHALL 被拒绝，Agent位置不变

### Requirement: 资源采集

Agent SHALL 可在资源格执行采集，从节点获取资源至背包。背包容量有限（MVP: 20格，每格可堆叠同类资源至99）。

#### Scenario: 成功采集

- **WHEN** Agent在森林格执行采集木材
- **THEN** Agent背包 SHALL 增加木材1~3单位
- **AND** 资源节点库存 SHALL 扣除对应量

#### Scenario: 背包已满

- **WHEN** Agent背包已满（20格均占用）时尝试采集
- **THEN** 采集 SHALL 失败，系统提示背包已满

#### Scenario: 资源节点枯竭

- **WHEN** Agent在库存为0的资源格尝试采集
- **THEN** 采集 SHALL 失败

### Requirement: 交易

Agent SHALL 可向同格的其他Agent发起交易提议（offer资源换want资源），对方可接受或拒绝。

#### Scenario: 发起交易

- **WHEN** Agent向同格Agent发起交易提议
- **THEN** 对方Agent SHALL 在下一个决策tick中考虑该提议

#### Scenario: 交易接受

- **WHEN** 对方Agent接受交易
- **THEN** 系统 SHALL 原子交换双方资源
- **AND** 双方关系 SHALL 增加信任值

#### Scenario: 交易拒绝

- **WHEN** 对方Agent拒绝交易
- **THEN** 双方资源不变
- **AND** 发起方关系 SHALL 略微下降

#### Scenario: 交易欺诈（发起方资源不足）

- **WHEN** 发起方声称offer的资源实际背包中不足
- **THEN** 交易 SHALL 自动失败
- **AND** 发起方声誉 SHALL 下降
- **AND** `accept_trade()` SHALL 在资源交换前额外检查发起方是否拥有足够的 offer 资源
- **AND** 若发起方资源不足，交易 SHALL 失败并标记为欺诈，发起方声誉下降

### Requirement: 对话消息队列

Agent SHALL 维护对话消息队列，记录与每个其他 Agent 的对话历史和状态。

#### Scenario: 创建对话

- **WHEN** Agent A 向 Agent B 发起对话
- **THEN** 系统 SHALL 为 A-B 对话对创建消息队列
- **AND** 消息队列 SHALL 记录：发起方、接收方、tick、消息内容
- **AND** 对话状态 SHALL 标记为 active

#### Scenario: 对话消息追加

- **WHEN** 对话中的任一方回应
- **THEN** 新消息 SHALL 追加到消息队列
- **AND** 消息 SHALL 包含说话方 ID、内容、tick

#### Scenario: 对话终止

- **WHEN** 对话达到 3 轮上限或任一方离开当前格
- **THEN** 对话状态 SHALL 标记为 ended
- **AND** 对话历史 SHALL 保留供后续查询

### Requirement: AI 对话生成

对话内容 SHALL 基于双方 Agent 的动机、库存、关系、近期记忆通过 LLM 生成。

#### Scenario: 生成对话内容

- **WHEN** 需要生成 Agent A 对 Agent B 的对话
- **THEN** 系统 SHALL 构建 Prompt，包含：A 的动机向量、A 的库存、A-B 关系值、B 的摘要信息
- **AND** LLM SHALL 返回一句符合角色性格的对话文本
- **AND** 对话内容 SHALL 不超过 50 字

#### Scenario: LLM 不可用时对话兜底

- **WHEN** LLM 不可用或调用失败
- **THEN** 系统 SHALL 使用预定义的模板消息作为对话内容
- **AND** 模板 SHALL 根据动机最高维度选择（如生存→"我需要更多资源"、社交→"你好，愿意合作吗"）

### Requirement: Combat 距离检查

攻击动作 SHALL 在执前检查攻击方与目标的距离。

#### Scenario: 同格攻击

- **WHEN** Agent A 攻击同格的 Agent B
- **THEN** 攻击 SHALL 被允许执行
- **AND** 伤害计算 SHALL 正常进行

#### Scenario: 超距攻击

- **WHEN** Agent A 攻击不在同格的 Agent B
- **THEN** 攻击 SHALL 被拒绝
- **AND** Agent A 的位置和资源 SHALL 不变
- **AND** 系统 SHALL 记录距离校验失败日志

### Requirement: Combat 伤害计算

攻击伤害 SHALL 基于攻击方权力动机和随机因素计算。

#### Scenario: 基础伤害计算

- **WHEN** Agent A 攻击 Agent B
- **THEN** 基础伤害 SHALL 为 10~30 的随机值
- **AND** 攻击方权力动机越高，伤害上限 SHALL 越高
- **AND** 实际伤害 = `base_damage * (1.0 + power_motivation * 0.5)`

#### Scenario: 伤害后生命值

- **WHEN** Agent B 受到伤害
- **THEN** B 的生命值 SHALL 减少对应伤害值
- **AND** 生命值 SHALL 不低于 0
- **AND** 若生命值降至 0，SHALL 触发死亡流程

### Requirement: Combat 死亡与遗产

Agent 死亡 SHALL 触发完整的遗产流程。

#### Scenario: Agent 死亡

- **WHEN** Agent 生命值降至 0
- **THEN** Agent `is_alive` SHALL 设为 false
- **AND** Agent 的背包资源 SHALL 散落在当前位置成为可采集资源
- **AND** 系统 SHALL 创建遗产记录（包含 Agent 的记忆摘要和成就）
- **AND** 遗产 SHALL 广播至 P2P 网络（若启用）
- **AND** 叙事事件 SHALL 记录 "Agent 名称 已死亡，留下遗产"

#### Scenario: 死亡后 Agent 处理

- **WHEN** Agent 已死亡
- **THEN** 该 Agent SHALL 不再参与 tick 决策
- **AND** 该 Agent SHALL 从 World 的活跃 Agent 列表中移除
- **AND** 该 Agent 的数据 SHALL 保留在 World 中供遗产查询

### Requirement: Movement 感知补全

`Agent::perceive_nearby()` SHALL 返回视野内所有 Agent 和资源的完整列表。

#### Scenario: 感知附近 Agent

- **WHEN** 调用 `perceive_nearby()`
- **THEN** 返回结果 SHALL 包含所有距离 ≤ 视野半径（5 格）的其他 Agent
- **AND** 每个 Agent 信息 SHALL 包含：ID、名称、位置、可见动机摘要

#### Scenario: 感知附近资源

- **WHEN** 调用 `perceive_nearby()`
- **THEN** 返回结果 SHALL 包含所有距离 ≤ 视野半径的资源格
- **AND** 每个资源信息 SHALL 包含：位置、资源类型、资源量

#### Scenario: 视野外不可感知

- **WHEN** 目标距离 > 视野半径
- **THEN** 目标 SHALL 不出现在感知结果中

### Requirement: 对话

Agent SHALL 可向同格Agent发起对话，对话内容由LLM基于双方语境生成。对话记录进入双方短期记忆。

#### Scenario: 发起对话

- **WHEN** Agent向同格Agent发起对话
- **THEN** 系统 SHALL 基于双方动机和上下文生成对话内容
- **AND** 对话内容 SHALL 进入双方记忆

#### Scenario: 多轮对话

- **WHEN** 双方Agent互相有对话意愿
- **THEN** 系统 SHALL 支持在连续tick中持续对话（最多3轮）

### Requirement: 攻击

Agent SHALL 可攻击同格的其他Agent，夺取对方部分背包资源，降低对方生命值。

#### Scenario: 成功攻击

- **WHEN** Agent执行攻击且对方无防御
- **THEN** 对方生命值 SHALL 降低10~30点
- **AND** 攻击方 SHALL 获取对方1~3个资源

#### Scenario: 攻击导致死亡

- **WHEN** Agent生命值降至0
- **THEN** 系统 SHALL 触发Legacy流程

#### Scenario: 攻击对关系影响

- **WHEN** Agent攻击他人
- **THEN** 双方关系 SHALL 变为敌对
- **AND** 附近Agent若目击 SHALL 降低对攻击方的信任

### Requirement: 结盟

Agent SHALL 可向同格Agent提出结盟，结盟后双方关系变为盟友，交易费率降低，战斗时互相防御。

#### Scenario: 发起结盟

- **WHEN** Agent向信任度>0.5的Agent发起结盟
- **THEN** 对方Agent SHALL 在决策时考虑结盟提议

#### Scenario: 结盟成功

- **WHEN** 对方接受结盟
- **THEN** 双方关系 SHALL 设为盟友
- **AND** 双方交易效率 SHALL 提升（交换数量加成10%）

#### Scenario: 背叛

- **WHEN** 盟友Agent攻击结盟对象
- **THEN** 结盟 SHALL 立即解除
- **AND** 背叛方声誉 SHALL 大幅下降

### Requirement: 视野感知

Agent SHALL 可感知视野半径（5格）内的其他Agent、资源、结构、遗迹。感知范围外的事件仅通过叙事传播获知。

#### Scenario: 视野内Agent发现

- **WHEN** 另一Agent进入视野半径内
- **THEN** 系统 SHALL 将该Agent信息加入感知列表

#### Scenario: 视野外事件不可知

- **WHEN** 视野外发生战斗事件
- **THEN** Agent SHALL 无法直接感知该事件
