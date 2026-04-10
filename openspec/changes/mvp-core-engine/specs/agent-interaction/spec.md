# 功能规格说明：Agent交互系统

## ADDED Requirements

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

#### Scenario: 交易欺诈（资源不足）

- **WHEN** 发起方声称offer的资源实际背包中不足
- **THEN** 交易 SHALL 自动失败
- **AND** 发起方声誉 SHALL 下降

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