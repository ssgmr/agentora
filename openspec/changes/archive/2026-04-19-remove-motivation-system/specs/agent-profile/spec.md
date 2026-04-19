# 角色配置系统

## ADDED Requirements

### Requirement: AgentProfile 角色档案

系统 SHALL 为每个 Agent 维护一个角色档案（AgentProfile），包含性格特征、行为倾向和初始叙事设定，用于编译进 System Prompt。

#### Scenario: 创建默认角色档案

- **WHEN** 创建新 Agent 且未指定角色配置
- **THEN** 系统 SHALL 生成默认角色档案
- **AND** 档案 SHALL 包含：基础人格种子（PersonalitySeed）和空的行为倾向

#### Scenario: 从配置加载角色档案

- **WHEN** 用户提供角色配置文件（YAML/TOML）
- **THEN** 系统 SHALL 解析并应用配置到 AgentProfile
- **AND** 配置 SHALL 支持：MBTI类型、性格描述、行为偏好、初始叙事

#### Scenario: 角色档案编译进 System Prompt

- **WHEN** 构建决策 Prompt
- **THEN** 系统 SHALL 将 AgentProfile 内容编译为 System Prompt 的一部分
- **AND** 格式 SHALL 为自然语言描述（如"你是一个勇敢的冒险者，喜欢探索未知"）

### Requirement: 动态倾向调整

系统 SHALL 支持在运行时动态调整 Agent 的行为倾向，通过注入新提示词或修改现有倾向实现。

#### Scenario: 注入临时倾向

- **WHEN** 外部系统（如玩家配置）注入新的行为倾向
- **THEN** 系统 SHALL 将新倾向添加到 AgentProfile
- **AND** 新倾向 SHALL 在下次决策时反映在 Prompt 中

#### Scenario: 移除倾向

- **WHEN** 临时倾向过期或被显式移除
- **THEN** 系统 SHALL 从 AgentProfile 中移除该倾向
- **AND** 后续决策 Prompt SHALL 不再包含该内容
