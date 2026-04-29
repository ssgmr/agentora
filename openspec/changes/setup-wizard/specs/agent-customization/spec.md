# 功能规格说明 - Agent 个性化配置

## ADDED Requirements

### Requirement: Agent 名字自定义

系统 SHALL 允许用户为玩家 Agent 设置自定义名字，用于世界中的显示标识。

#### Scenario: 设置 Agent 名字

- **WHEN** 用户在引导页面输入 Agent 名字
- **THEN** 名字 SHALL 存储到 UserConfig.agent.name
- **AND** 名字 SHALL 用于 Agent 生成时的命名
- **AND** 名字 SHALL 在世界中显示

#### Scenario: 名字长度限制

- **WHEN** 用户输入名字超过 20 字符
- **THEN** 系统 SHALL 截断或拒绝输入
- **AND** 显示提示"名字长度不能超过 20 字符"

#### Scenario: 名字非空校验

- **WHEN** 用户提交配置时名字为空
- **THEN** 系统 SHALL 拒绝提交
- **AND** 显示错误提示"请输入 Agent 名字"

### Requirement: 系统提示词自定义

用户 SHALL 可设置自定义系统提示词，注入到 Agent 的决策 Prompt 中影响决策倾向。

#### Scenario: 设置系统提示词

- **WHEN** 用户输入自定义系统提示词
- **THEN** 提示词 SHALL 存储到 PersonalitySeed.custom_prompt
- **AND** PromptBuilder.build_personality_section SHALL 优先使用 custom_prompt

#### Scenario: 提示词注入位置

- **WHEN** 构建 Agent 决策 Prompt
- **THEN** custom_prompt SHALL 注入在性格描述之前
- **AND** Prompt 结构 SHALL 为：custom_prompt + 默认性格描述 + 世界规则

#### Scenario: 提示词影响决策

- **WHEN** LLM 接收包含 custom_prompt 的 Prompt
- **THEN** custom_prompt SHALL 指导 Agent 决策倾向
- **AND** 例如"谨慎的探索者" SHALL 倾向探索和规避风险

#### Scenario: 空提示词处理

- **WHEN** 用户未设置自定义提示词
- **THEN** Prompt SHALL 仅使用 PersonalitySeed.description
- **AND** 决策倾向 SHALL 由性格模板决定

### Requirement: Agent 图标选择

用户 SHALL 可选择预设图标或上传自定义图标，用于游戏中 Agent 的可视化显示。

#### Scenario: 预设图标列表

- **WHEN** 系统定义预设图标
- **THEN** SHALL 包含至少以下图标：
  - default（默认人物）
  - wizard（法师）
  - fox（狐狸）
  - dragon（龙）
  - lion（狮子）
  - robot（机器人）

#### Scenario: 选择预设图标

- **WHEN** 用户选择预设图标
- **THEN** icon_id SHALL 存储到 PersonalitySeed.icon_id
- **AND** icon_id SHALL 为图标标识符（如 "fox"）
- **AND** AgentSnapshot.icon_id SHALL 包含该值

#### Scenario: 上传自定义图标

- **WHEN** 用户上传自定义图标文件
- **THEN** 系统 SHALL 使用 image crate 处理
- **AND** 自动缩放至 32x32 像素
- **AND** 保存到 user_icons/ 目录
- **AND** custom_icon_path SHALL 存储到 PersonalitySeed

#### Scenario: 图标加载显示

- **WHEN** 游戏渲染 Agent
- **THEN** agent_manager.gd SHALL 根据 icon_id 加载对应 Sprite
- **AND** 预设图标从 assets/textures/agents/ 加载
- **AND** 自定义图标从 custom_icon_path 加载

#### Scenario: 图标缩放处理

- **WHEN** 处理上传的自定义图标
- **THEN** SHALL 使用 image::imageops::resize
- **AND** 使用 Lanczos3 过滤器保持质量
- **AND** 输出格式 SHALL 为 PNG
- **AND** 输出尺寸 SHALL 固定为 32x32

### Requirement: PersonalitySeed 扩展

PersonalitySeed 结构 SHALL 新增字段支持用户自定义配置。

#### Scenario: 新增字段定义

- **WHEN** PersonalitySeed 定义
- **THEN** SHALL 包含以下新字段：
  - custom_prompt: Option<String>（自定义系统提示词）
  - icon_id: Option<String>（预设图标标识）
  - custom_icon_path: Option<String>（自定义图标文件路径）

#### Scenario: 数据传递到 AgentSnapshot

- **WHEN** 生成 WorldSnapshot
- **THEN** AgentSnapshot SHALL 包含 icon_id 字段
- **AND** Godot SHALL 通过 snapshot 获取 Agent 图标信息

### Requirement: 配置持久化

Agent 个性化配置 SHALL 随 UserConfig 持久化。

#### Scenario: 配置保存格式

- **WHEN** 保存 Agent 配置到 user_config.toml
- **THEN** 格式 SHALL 为：
```toml
[agent]
name = "智行者"
custom_prompt = "你是一个谨慎的探索者..."
icon_id = "fox"
custom_icon_path = ""
```

#### Scenario: 配置加载恢复

- **WHEN** 系统启动时加载 user_config.toml
- **THEN** SHALL 解析 [agent] 配置段
- **AND** PersonalitySeed SHALL 使用保存的值