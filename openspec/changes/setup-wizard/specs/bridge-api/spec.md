# 功能规格说明 - Bridge API（增量）

## ADDED Requirements

### Requirement: 用户配置 API

SimulationBridge SHALL 新增配置管理 API，支持 Godot 读写用户配置。

#### Scenario: set_user_config 方法

- **WHEN** Godot 调用 bridge.set_user_config(config: Dictionary)
- **THEN** Bridge SHALL 解析 Dictionary 为 UserConfig 结构
- **AND** 保存到 config/user_config.toml
- **AND** 返回 true 表示成功，false 表示失败

#### Scenario: get_user_config 方法

- **WHEN** Godot 调用 bridge.get_user_config()
- **THEN** SHALL 返回 Dictionary 包含当前配置
- **AND** 格式与 set_user_config 输入一致
- **AND** 无配置时返回空 Dictionary

#### Scenario: has_user_config 方法

- **WHEN** Godot 调用 bridge.has_user_config()
- **THEN** SHALL 返回 bool 表示配置文件是否存在

### Requirement: 模型下载 API

SimulationBridge SHALL 新增模型下载 API，支持异步下载和进度反馈。

#### Scenario: download_model 方法

- **WHEN** Godot 调用 bridge.download_model(url: String, dest: String)
- **THEN** Bridge SHALL 启动异步下载任务
- **AND** 返回 bool 表示任务是否成功启动

#### Scenario: get_available_models 方法

- **WHEN** Godot 调用 bridge.get_available_models()
- **THEN** SHALL 返回 Array 包含预置模型信息
- **AND** 每个元素为 Dictionary：{name, size_mb, description, is_downloaded}

#### Scenario: cancel_download 方法

- **WHEN** Godot 调用 bridge.cancel_download()
- **THEN** SHALL 取消当前下载任务
- **AND** 删除部分下载的临时文件

### Requirement: 下载进度信号

Bridge SHALL 通过信号通知 Godot 下载状态。

#### Scenario: download_progress 信号定义

- **WHEN** 定义 Bridge 信号
- **THEN** download_progress SHALL 为 [signal]
- **AND** 参数：downloaded_mb: float, total_mb: float, speed_mbps: float

#### Scenario: model_download_complete 信号定义

- **WHEN** 定义 Bridge 信号
- **THEN** model_download_complete SHALL 为 [signal]
- **AND** 参数：path: String

#### Scenario: model_download_failed 信号定义

- **WHEN** 定义 Bridge 信号
- **THEN** model_download_failed SHALL 为 [signal]
- **AND** 参数：error: String

### Requirement: 配置应用到模拟

配置设置后 SHALL 影响 Simulation 的初始化。

#### Scenario: LLM 配置应用

- **WHEN** 配置中 llm.mode = "local"
- **THEN** Simulation SHALL 使用 LlamaProvider
- **AND** 从 local_model_path 加载模型

#### Scenario: Agent 配置应用

- **WHEN** 配置中 agent.name 存在
- **THEN** 玩家 Agent SHALL 使用自定义名字
- **AND** custom_prompt SHALL 注入到 Prompt

#### Scenario: P2P 配置应用

- **WHEN** 配置中 p2p.mode = "join"
- **THEN** Simulation SHALL 连接到 seed_address
- **AND** p2p.mode = "single" SHALL 跳过 P2P 初始化