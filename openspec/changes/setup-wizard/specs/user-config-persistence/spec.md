# 功能规格说明 - 用户配置持久化

## ADDED Requirements

### Requirement: 配置文件结构

系统 SHALL 定义 UserConfig 结构体，存储用户的 LLM、Agent、P2P 配置。

#### Scenario: 配置文件位置

- **WHEN** 系统保存用户配置
- **THEN** 配置 SHALL 存储到 config/user_config.toml
- **AND** 文件 SHALL 使用 TOML 格式

#### Scenario: 配置结构定义

- **WHEN** UserConfig 结构定义
- **THEN** SHALL 包含以下字段：
```toml
[llm]
mode = "local"  # local / remote / rule_only
api_endpoint = ""
api_token = ""
model_name = ""
local_model_path = "models/Qwen3.5-2B-Q4_K_M.gguf"

[agent]
name = ""
custom_prompt = ""
icon_id = "default"
custom_icon_path = ""

[p2p]
mode = "single"  # single / create / join
seed_address = ""
```

### Requirement: 首次启动检测

系统 SHALL 在启动时检测配置文件是否存在，决定是否显示引导页面。

#### Scenario: 无配置文件

- **WHEN** 启动时 config/user_config.toml 不存在
- **THEN** SHALL 加载 setup_wizard.tscn 引导页面
- **AND** 不启动模拟

#### Scenario: 有配置文件

- **WHEN** 启动时 config/user_config.toml 存在
- **THEN** SHALL 加载配置
- **AND** 跳过引导页面
- **AND** 直接加载 main.tscn 并启动模拟

#### Scenario: 配置文件损坏

- **WHEN** 配置文件存在但解析失败
- **THEN** SHALL 显示错误提示
- **AND** 提供删除配置重新配置选项
- **AND** 或使用默认配置启动

### Requirement: 配置保存

引导页面完成配置后 SHALL 保存到配置文件。

#### Scenario: 保存时机

- **WHEN** 用户在引导页面点击"开始游戏"
- **THEN** SHALL 验证必填项
- **AND** 验证通过后 SHALL 保存配置

#### Scenario: 保存 API

- **WHEN** Godot 调用 Bridge.set_user_config(config)
- **THEN** Bridge SHALL 将 Dictionary 转换为 Rust 结构
- **AND** 使用 toml crate 序列化
- **AND** 写入到 config/user_config.toml

#### Scenario: 保存成功确认

- **WHEN** 配置保存成功
- **THEN** Bridge SHALL 返回成功信号
- **AND** Godot SHALL 切换到 main.tscn

### Requirement: 配置加载

系统启动时 SHALL 从配置文件加载用户配置。

#### Scenario: 加载流程

- **WHEN** 系统启动
- **THEN** SHALL 尝试读取 config/user_config.toml
- **AND** 使用 toml crate 解析
- **AND** 转换为 UserConfig 结构体

#### Scenario: 配置应用到模拟

- **WHEN** 配置加载成功
- **THEN** LLM 配置 SHALL 用于初始化 Provider
- **AND** Agent 配置 SHALL 用于玩家 Agent 创建
- **AND** P2P 配置 SHALL 用于网络初始化

#### Scenario: 缺失字段处理

- **WHEN** 配置文件中某些字段缺失
- **THEN** SHALL 使用默认值填充
- **AND** 不中断启动流程

### Requirement: 游戏内配置修改

游戏运行时 SHALL 支持修改配置（重启生效）。

#### Scenario: 打开设置面板

- **WHEN** 用户在游戏中打开设置面板
- **THEN** SHALL 显示当前配置信息
- **AND** SHALL 提供修改选项

#### Scenario: 修改配置保存

- **WHEN** 用户修改配置并保存
- **THEN** SHALL 更新 config/user_config.toml
- **AND** 显示提示"配置已保存，重启生效"

#### Scenario: 重启应用

- **WHEN** 用户重启应用
- **THEN** SHALL 加载新的配置
- **AND** 使用修改后的设置运行

### Requirement: Bridge API 扩展

SimulationBridge SHALL 新增配置相关 API。

#### Scenario: set_user_config 方法

- **WHEN** Godot 调用 Bridge.set_user_config(config: Dictionary)
- **THEN** SHALL 解析 Dictionary 为 Rust UserConfig
- **AND** 保存到配置文件
- **AND** 返回成功/失败状态

#### Scenario: get_user_config 方法

- **WHEN** Godot 调用 Bridge.get_user_config()
- **THEN** SHALL 返回当前配置的 Dictionary
- **AND** 包含 [llm]、[agent]、[p2p] 所有配置段

#### Scenario: has_user_config 方法

- **WHEN** Godot 调用 Bridge.has_user_config()
- **THEN** SHALL 返回 config/user_config.toml 是否存在