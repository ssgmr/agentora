# 功能规格说明 - Agent 个性配置（增量）

## ADDED Requirements

### Requirement: PersonalitySeed 新增字段

PersonalitySeed 结构 SHALL 新增 custom_prompt 和 icon_id 字段，支持用户自定义配置。

#### Scenario: 字段定义扩展

- **WHEN** PersonalitySeed 结构体定义
- **THEN** SHALL 包含新字段：
  - custom_prompt: Option<String>（用户自定义系统提示词）
  - icon_id: Option<String>（预设图标标识符）
  - custom_icon_path: Option<String>（自定义图标文件路径）

#### Scenario: custom_prompt 注入 Prompt

- **WHEN** PromptBuilder.build_personality_section 执行
- **AND** personality.custom_prompt 存在
- **THEN** SHALL 优先使用 custom_prompt
- **AND** 格式为：custom_prompt + "\n\n" + 默认性格描述

#### Scenario: icon_id 传递到 AgentSnapshot

- **WHEN** 生成 AgentSnapshot
- **THEN** icon_id SHALL 包含在快照中
- **AND** Godot agent_manager SHALL 根据 icon_id 加载对应图标

### Requirement: 默认值处理

新增字段 SHALL 有合理的默认值处理逻辑。

#### Scenario: custom_prompt 为空

- **WHEN** personality.custom_prompt 为 None 或空字符串
- **THEN** PromptBuilder SHALL 仅使用 personality.description
- **AND** 行为与现有逻辑一致

#### Scenario: icon_id 为空

- **WHEN** personality.icon_id 为 None
- **THEN** AgentSnapshot.icon_id SHALL 使用 "default"
- **AND** Godot SHALL 加载默认图标