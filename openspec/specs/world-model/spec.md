# 世界模型

## Purpose

定义256×256+大规模网格地图、地形系统、区域划分、资源节点、环境压力事件和可建造结构，构成模拟世界的物理基础。

## Requirements

### Requirement: 大规模网格地图

系统 SHALL 支持最小256×256的网格地图，每格为基本空间单元。地图SHALL可通过WorldSeed配置尺寸，理论支持任意大小。

#### Scenario: 默认地图创建

- **WHEN** 未指定地图大小时创建世界
- **THEN** 系统 SHALL 创建256×256网格地图

#### Scenario: 自定义地图大小

- **WHEN** WorldSeed指定地图大小为512×512
- **THEN** 系统 SHALL 按指定大小创建地图

#### Scenario: 地图边界约束

- **WHEN** Agent尝试移动到坐标 < 0 或 >= 地图大小
- **THEN** 系统 SHALL 阻止移动，Agent保持在地图内

### Requirement: 地形类型

系统 SHALL 支持以下地形类型：平原（可通行）、森林（可通行，采集木材）、山地（不可通行）、水域（不可通行）、沙漠（可通行，无资源）。MVP至少实现4种地形。

#### Scenario: 地形通行性

- **WHEN** Agent在平原格尝试向山地格移动
- **THEN** 系统 SHALL 阻止移动，因山地不可通行

#### Scenario: 森林资源采集

- **WHEN** Agent在森林格执行采集动作
- **THEN** 系统 SHALL 允许采集木材资源

### Requirement: 区域划分

系统 SHALL 将地图划分为区域（Region），每16×16格为一个区域，共16×16个区域（256×256地图）。每个区域有独立的资源参数、压力池、叙事版本。

#### Scenario: 区域参数独立性

- **WHEN** 北方矿区资源产出下降50%
- **THEN** 仅北方矿区所在区域受影响，其他区域资源产出不变

#### Scenario: 区域感知

- **WHEN** Agent在区域边界附近
- **THEN** 系统 SHALL 将邻区的基本信息纳入Agent感知范围

### Requirement: 资源节点与再生

系统 SHALL 在地图上放置资源节点（矿脉、农田、森林、水源），每个资源节点有产量上限和再生周期。资源被采集后按周期恢复。

#### Scenario: 资源采集

- **WHEN** Agent在资源格执行采集
- **THEN** 系统 SHALL 从节点库存中扣除资源量
- **AND** Agent背包 SHALL 增加对应资源

#### Scenario: 资源枯竭

- **WHEN** 资源节点库存降为0
- **THEN** 该节点 SHALL 进入枯竭状态，不可采集
- **AND** 系统 SHALL 在再生周期到达后恢复部分库存

#### Scenario: 资源再生

- **WHEN** 资源节点枯竭后经过再生周期（如10个tick）
- **THEN** 节点库存 SHALL 恢复至最大值的30%~50%

### Requirement: 环境压力系统

系统 SHALL 动态生成环境压力事件影响区域参数：资源产出波动、气候变化（影响农田/水源产出）、区域封锁等。压力事件通过GossipSub广播，成为Agent的Spark来源。

#### Scenario: 资源产出波动

- **WHEN** 系统随机触发"矿脉衰减"压力事件
- **THEN** 目标区域的矿脉产出 SHALL 下降20%~50%
- **AND** 事件 SHALL 广播至该区域所有Agent

#### Scenario: 气候事件

- **WHEN** 系统触发"干旱"气候事件
- **THEN** 目标区域的农田和水源产出 SHALL 下降
- **AND** 事件持续5~15个tick后自动解除

#### Scenario: 压力生成频率

- **WHEN** 世界运行中
- **THEN** 系统 SHALL 每20~50个tick随机生成一个环境压力事件
- **AND** 事件 SHALL 优先选择已长时间无压力的区域

### Requirement: 结构与建筑

系统 SHALL 支持Agent消耗资源建造结构（营地、围栏、仓库），结构占据地图格子，可被其他Agent发现和交互。

#### Scenario: 建造营地

- **WHEN** Agent拥有足够材料（如木材5）并执行建造动作
- **THEN** 系统 SHALL 在Agent当前位置创建营地结构
- **AND** 从Agent背包扣除材料

#### Scenario: 发现他人建筑

- **WHEN** Agent进入有建筑的格子
- **THEN** 系统 SHALL 将建筑信息纳入Agent感知
