# 功能规格说明 - Agent个性配置

## ADDED Requirements

### Requirement: WorldSeed支持Agent性格配置

WorldSeed SHALL 支持配置Agent的个性参数，包括性格描述文本和大五人格参数。

配置格式 SHALL 支持以下字段：

```toml
[agent_personalities]
# 默认性格模板（未配置时使用）
default = { openness = 0.5, agreeableness = 0.5, neuroticism = 0.5, description = "一个普通的世界居民" }

# 自定义性格模板
explorer = { openness = 0.8, agreeableness = 0.3, neuroticism = 0.4, description = "一个好奇的探索者，喜欢发现新事物，倾向于独自行动" }
socializer = { openness = 0.6, agreeableness = 0.8, neuroticism = 0.3, description = "一个友善的交际者，喜欢与其他Agent交流，乐于合作" }
survivor = { openness = 0.3, agreeableness = 0.4, neuroticism = 0.7, description = "一个谨慎的生存者，注重自身安全，会优先储备资源" }
builder = { openness = 0.5, agreeableness = 0.6, neuroticism = 0.3, description = "一个创造者，喜欢建造建筑和留下遗产" }

# Agent创建时使用的性格模板
agent_assignment = "random"  # 可选：random、default、explorer、socializer、survivor、builder
```

#### Scenario: 配置文件解析

- **WHEN** WorldSeed加载
- **THEN** 系统 SHALL 解析 `agent_personalities` 配置段
- **AND** 若配置缺失 SHALL 使用默认性格参数

#### Scenario: Agent创建时分配性格

- **WHEN** Agent创建
- **AND** `agent_assignment = "random"`
- **THEN** 系统 SHALL 随机选择一个性格模板
- **AND** Agent的PersonalitySeed SHALL 使用模板中的大五人格参数

#### Scenario: Agent创建时使用指定性格

- **WHEN** Agent创建
- **AND** `agent_assignment = "explorer"`
- **THEN** 所有Agent SHALL 使用explorer性格模板

#### Scenario: 性格配置缺失回退

- **WHEN** WorldSeed不包含 `agent_personalities` 配置
- **THEN** 系统 SHALL 使用硬编码的默认性格
- **AND** description SHALL 为空字符串

### Requirement: 性格描述注入Prompt

Agent的性格描述 SHALL 注入到System Prompt中，影响LLM的决策倾向。

#### Scenario: Prompt包含性格描述

- **WHEN** 构建Agent决策Prompt
- **THEN** System Prompt SHALL 包含性格描述文本
- **AND** 格式为："你是{agent_name}，{description}"

#### Scenario: 性格描述影响决策倾向

- **WHEN** LLM阅读包含性格描述的Prompt
- **THEN** 性格描述 SHALL 暗示决策倾向：
  - "好奇的探索者" SHALL 倾向Explore和Gather动作
  - "友善的交际者" SHALL 倾向Talk和Trade动作
  - "谨慎的生存者" SHALL 倾向Eat/Drink和储备资源
  - "创造者" SHALL 倾向Build和遗产相关动作

#### Scenario: 性格描述为空时回退

- **WHEN** Agent没有性格描述
- **THEN** Prompt SHALL 使用默认描述："一个自主决策的AI Agent"
- **AND** 不暗示特定决策倾向

### Requirement: 大五人格参数映射

大五人格参数（openness, agreeableness, neuroticism）SHALL 作为PersonalitySeed结构的一部分存储。

#### Scenario: PersonalitySeed结构

- **WHEN** Agent创建
- **THEN** PersonalitySeed SHALL 包含：
  - openness: 开放性 [0.0, 1.0]
  - agreeableness: 宜人性 [0.0, 1.0]
  - neuroticism: 神经质 [0.0, 1.0]

#### Scenario: openness影响探索倾向

- **WHEN** Agent openness > 0.7
- **THEN** Agent SHALL 更倾向探索新区域和发现新资源

#### Scenario: agreeableness影响社交倾向

- **WHEN** Agent agreeableness > 0.7
- **THEN** Agent SHALL 更倾向与他人合作和交流

#### Scenario: neuroticism影响风险规避

- **WHEN** Agent neuroticism > 0.7
- **THEN** Agent SHALL 更倾向规避风险和保守决策

### Requirement: 性格多样性

系统 SHALL 支持多个Agent使用不同性格，产生决策多样性。

#### Scenario: 随机分配性格

- **WHEN** 创建多个Agent
- **AND** `agent_assignment = "random"`
- **THEN** 不同Agent SHALL 可能使用不同性格模板
- **AND** 整体决策风格呈现多样性

#### Scenario: 全局统一性格

- **WHEN** 创建多个Agent
- **AND** `agent_assignment = "explorer"`
- **THEN** 所有Agent SHALL 使用相同的explorer性格
- **AND** 整体决策风格统一

### Requirement: 性格配置热更新

性格配置 SHALL 支持通过WorldSeed重新加载更新，但已创建的Agent性格不变。

#### Scenario: 重载WorldSeed

- **WHEN** WorldSeed重新加载
- **THEN** 新创建的Agent SHALL 使用新的性格配置
- **AND** 已存在的Agent性格 SHALL 保持不变