# 世界种子配置

## Purpose

定义WorldSeed.toml配置文件的结构和含义，用于初始化世界的地图、资源、Agent和网络参数。

## Requirements

### Requirement: WorldSeed配置文件

系统 SHALL 支持通过WorldSeed.toml文件定义世界的初始配置，包括地图大小、资源分布、区域参数、压力池参数、Agent初始配置。

#### Scenario: 加载WorldSeed

- **WHEN** 世界初始化
- **THEN** 系统 SHALL 从WorldSeed.toml读取配置
- **AND** 按配置生成初始世界状态

#### Scenario: 配置缺失时使用默认值

- **WHEN** WorldSeed.toml中某些字段缺失
- **THEN** 系统 SHALL 使用合理的默认值

### Requirement: 地图与资源配置

WorldSeed SHALL 支持配置地图尺寸、地形分布比例、资源节点密度和类型。

#### Scenario: 自定义地图大小

- **WHEN** WorldSeed中配置map_size = [512, 512]
- **THEN** 系统 SHALL 创建512×512地图

#### Scenario: 资源密度配置

- **WHEN** WorldSeed中配置resource_density = 0.15
- **THEN** 系统 SHALL 按每15%的可用格放置资源节点

### Requirement: Agent初始配置

WorldSeed SHALL 支持配置初始Agent数量、动机向量模板、人格种子分布、初始位置策略（随机/聚类/分散）。

#### Scenario: 指定初始Agent数

- **WHEN** WorldSeed中配置initial_agents = 10
- **THEN** 系统 SHALL 创建10个初始Agent

#### Scenario: 动机模板

- **WHEN** WorldSeed中定义了"商人"动机模板 [0.5, 0.8, 0.4, 0.3, 0.7, 0.3]
- **THEN** 使用该模板创建的Agent SHALL 具有高社交和高权力动机

#### Scenario: 位置策略

- **WHEN** WorldSeed配置spawn_strategy = "clustered"
- **THEN** 初始Agent SHALL 在几个聚集点附近生成

### Requirement: 网络种子节点

WorldSeed SHALL 支持配置种子节点地址列表，用于P2P网络的初始引导。

#### Scenario: 配置种子节点

- **WHEN** WorldSeed中配置seed_peers = ["/ip4/1.2.3.4/tcp/4001"]
- **THEN** 节点启动时 SHALL 尝试连接该种子节点
