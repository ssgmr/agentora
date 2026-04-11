# 功能规格说明：遗产系统

## ADDED Requirements

### Requirement: Agent死亡触发

系统 SHALL 在Agent生命值降至0或年龄超过最大寿命时触发死亡流程。死亡不可逆转，Agent从活跃列表移除。

#### Scenario: 生命值降至零

- **WHEN** Agent遭受攻击或饥饿导致生命值 ≤ 0
- **THEN** 系统 SHALL 触发Legacy流程

#### Scenario: 自然死亡

- **WHEN** Agent年龄超过最大寿命（MVP: 200 tick）
- **THEN** 系统 SHALL 触发Legacy流程
- **AND** 生命值缓慢降为0

### Requirement: 遗迹生成

系统 SHALL 在Agent死亡位置生成遗迹实体（墓冢/废墟/营地残骸），遗迹永久存在直至被交互消耗。

#### Scenario: 墓冢生成

- **WHEN** Agent死亡
- **THEN** 死亡位置 SHALL 生成"墓冢"遗迹实体
- **AND** 遗迹 SHALL 在地图上可见

#### Scenario: 建筑变为废墟

- **WHEN** 拥有建筑的Agent死亡
- **THEN** 其建筑 SHALL 变为"废墟"遗迹
- **AND** 废墟中 SHALL 保留部分建筑材料

### Requirement: 物品散落

系统 SHALL 将死亡Agent背包中的物品散落至遗迹格子，其他Agent可拾取。

#### Scenario: 资源散落

- **WHEN** Agent死亡时背包有铁矿5、食物3
- **THEN** 遗迹格子 SHALL 包含铁矿5、食物3
- **AND** 任何到达该格的Agent可执行拾取

#### Scenario: 散落物品衰减

- **WHEN** 散落物品存在超过50个tick无人拾取
- **THEN** 物品数量 SHALL 每tick衰减10%

### Requirement: 回响日志

系统 SHALL 将Agent最后的3条短期记忆压缩为回响日志（多模态摘要+情感标签），附加于遗迹实体。

#### Scenario: 回响日志生成

- **WHEN** Agent死亡
- **THEN** 系统 SHALL 取最后3条短期记忆生成压缩摘要
- **AND** 摘要 SHALL 包含情感标签和核心事实

#### Scenario: 遗迹交互读取回响

- **WHEN** 其他Agent进入遗迹格并执行"祭拜/探索"动作
- **THEN** 系统 SHALL 展示回响日志内容
- **AND** 访问Agent的认知或传承动机 SHALL 微增

### Requirement: 未竟契约

系统 SHALL 将死亡Agent的关系网转为未竟契约，广播至社交圈。他人可通过履行契约获得信任和声望。

#### Scenario: 未竟契约广播

- **WHEN** Agent死亡时有未完成交易
- **THEN** 系统 SHALL 生成未竟契约并通过GossipSub广播
- **AND** 契约 SHALL 包含原定交易内容

#### Scenario: 履行契约

- **WHEN** 其他Agent向死亡Agent的盟友完成原定交易
- **THEN** 履行方 SHALL 获得声望奖励
- **AND** 死亡Agent盟友的信任度 SHALL 向履行方提升

### Requirement: 遗产广播

系统 SHALL 将遗产事件通过GossipSub广播至全网络，成为新Agent的Spark来源。遗产事件包含：死亡Agent的名字、遗迹位置、物品摘要、回响日志关键词。

#### Scenario: 全网广播

- **WHEN** Agent死亡遗产生成完成
- **THEN** 系统 SHALL 通过GossipSub广播遗产事件
- **AND** 所有节点 SHALL 收到事件

#### Scenario: 遗产成为Spark

- **WHEN** 其他Agent感知到遗产事件
- **THEN** Agent的认知/传承动机维度 SHALL 产生激励缺口
- **AND** 可能触发前往遗迹探索的Spark