# 功能规格说明 - 世界种子配置（增量）

## ADDED Requirements

### Requirement: 玩家 Agent 配置字段

WorldSeed SHALL 新增 player_agent_config 字段，用于玩家 Agent 的个性化配置注入。

#### Scenario: 字段定义

- **WHEN** WorldSeed 结构体定义
- **THEN** SHALL 新增字段：
```toml
[player_agent_config]
name = "智行者"
custom_prompt = "你是一个谨慎的探索者..."
icon_id = "fox"
custom_icon_path = ""
```

#### Scenario: 配置注入 Agent 创建

- **WHEN** World 创建玩家 Agent
- **AND** WorldSeed.player_agent_config 存在
- **THEN** Agent.name SHALL 使用配置中的名字
- **AND** PersonalitySeed.custom_prompt SHALL 使用配置中的提示词
- **AND** PersonalitySeed.icon_id SHALL 使用配置中的图标

#### Scenario: 配置缺失处理

- **WHEN** WorldSeed.player_agent_config 缺失或为空
- **THEN** 玩家 Agent SHALL 使用默认配置
- **AND** 名字为 "Player_Agent"
- **AND** 无自定义提示词

### Requirement: UserConfig 与 WorldSeed 合并

用户配置 SHALL 在启动时合并到 WorldSeed。

#### Scenario: 配置合并流程

- **WHEN** Simulation 初始化
- **THEN** SHALL 加载 UserConfig
- **AND** 将 UserConfig.agent 合并到 WorldSeed.player_agent_config
- **AND** 将 UserConfig.p2p 合并到 WorldSeed.seed_peers（如 join 模式）

#### Scenario: P2P 配置合并

- **WHEN** UserConfig.p2p.mode = "join"
- **AND** UserConfig.p2p.seed_address 存在
- **THEN** WorldSeed.seed_peers SHALL 包含该地址
- **AND** Simulation SHALL 连接到该种子节点