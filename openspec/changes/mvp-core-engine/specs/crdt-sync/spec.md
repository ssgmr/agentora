# 功能规格说明：CRDT状态同步

## ADDED Requirements

### Requirement: LWW-Register

系统 SHALL 实现Last-Writer-Wins Register CRDT，用于同步Agent状态（位置/生命/动机向量）和地图结构（建筑/遗迹）。并发写入以较大时间戳为准，时间戳相同以PeerId字典序为准。

#### Scenario: Agent状态同步

- **WHEN** Agentα在节点A移动到新位置
- **THEN** 节点A SHALL 生成LWW-Register操作（新位置+时间戳）广播
- **AND** 节点B收到后 SHALL 以时间戳决定是否更新本地副本

#### Scenario: 并发写入冲突

- **WHEN** 两个节点同时修改同一Agent状态（不可能发生，因每Agent仅一个节点控制）
- **THEN** 系统 SHALL 以较大时间戳的写入为准

### Requirement: G-Counter

系统 SHALL 实现Grow-Only Counter CRDT，用于同步资源采集量等只增不减的数值。每个节点维护自己的计数器分量，合并时取各分量最大值。

#### Scenario: 资源采集计数同步

- **WHEN** 节点A的Agent采集了3单位铁矿
- **THEN** 节点A SHALL 递增本地G-Counter分量
- **AND** 广播后节点B合并时 SHALL 取max(本地, 远程)各分量

#### Scenario: 最终一致性

- **WHEN** 两个节点各自独立递增计数器后交换状态
- **THEN** 合并后双方 SHALL 看到相同的总计数

### Requirement: OR-Set

系统 SHALL 实现Observed-Remove Set CRDT，用于同步事件日志和叙事版本。支持添加和删除，删除仅影响已观察的元素，不阻止后续重新添加。

#### Scenario: 事件日志同步

- **WHEN** Agent动作产生新事件
- **THEN** 系统 SHALL 向OR-Set添加事件（含唯一ID和标签）
- **AND** 广播后所有节点 SHALL 合并事件

#### Scenario: 删除旧事件

- **WHEN** 事件超过保留期限
- **THEN** 系统 SHALL 删除该事件（OR-Set语义）
- **AND** 删除操作 SHALL 同步至其他节点

#### Scenario: 并发添加和删除

- **WHEN** 节点A添加事件X的同时节点B删除事件X
- **THEN** 合并后事件X SHALL 存在（添加优先于未观察到的删除）

### Requirement: 操作签名与验证

系统 SHALL 对每个CRDT操作附带节点签名，接收方验证签名后才合并。防止未授权的篡改。

#### Scenario: 正常签名验证

- **WHEN** 接收到CRDT操作
- **THEN** 系统 SHALL 验证操作签名与PeerId匹配
- **AND** 验证通过后执行合并

#### Scenario: 签名不匹配

- **WHEN** 接收到签名不匹配的CRDT操作
- **THEN** 系统 SHALL 拒绝该操作并记录警告

### Requirement: 状态快照与Merkle校验

系统 SHALL 每100个tick生成世界状态的Merkle根，用于快速校验节点间状态一致性。差异超阈值时触发全量同步。

#### Scenario: Merkle根一致

- **WHEN** 两个节点交换Merkle根且一致
- **THEN** 双方 SHALL 确认状态已同步

#### Scenario: Merkle根不一致

- **WHEN** 两个节点Merkle根不一致
- **THEN** 系统 SHALL 触发差异区域的全量状态交换