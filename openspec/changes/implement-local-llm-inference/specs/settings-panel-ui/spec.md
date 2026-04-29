# 功能规格说明 - settings-panel-ui（增量）

## ADDED Requirements

### Requirement: 下载进度条组件

settings_panel SHALL 新增下载进度条 UI 组件。

#### Scenario: 进度条节点创建

- **WHEN** settings_panel._ready() 执行
- **THEN** 系统 SHALL 在 LocalModelContainer 下创建 ProgressBar 节点
- **AND** 节点名称 SHALL 为 "DownloadProgressBar"
- **AND** 初始 visible SHALL 为 false

#### Scenario: 进度状态 Label 创建

- **WHEN** settings_panel._ready() 执行
- **THEN** 系统 SHALL 创建下载状态 Label
- **AND** 节点名称 SHALL 为 "DownloadStatusLabel"
- **AND** 用于显示"已下载: X/Y MB, 速度: Z MB/s"

#### Scenario: 取消下载按钮创建

- **WHEN** settings_panel._ready() 执行
- **THEN** 系统 SHALL 创建"取消下载"按钮
- **AND** 节点名称 SHALL 为 "CancelDownloadBtn"
- **AND** 初始 visible SHALL 为 false

### Requirement: 模型加载状态显示

settings_panel SHALL 显示模型加载状态。

#### Scenario: 加载中状态显示

- **WHEN** 收到 model_load_start 信号
- **THEN** 模型选项 SHALL 显示"加载中..."状态
- **AND** 进度条 SHALL 显示估算进度
- **AND** 禁用"开始游戏"按钮

#### Scenario: 加载完成状态显示

- **WHEN** 收到 model_load_complete 信号
- **THEN** 模型选项 SHALL 显示"已加载 ✅"状态
- **AND** 显示使用的 GPU 后端
- **AND** 启用"开始游戏"按钮

#### Scenario: 加载失败状态显示

- **WHEN** 收到 model_load_failed 信号
- **THEN** 模型选项 SHALL 显示"加载失败"状态
- **AND** 显示错误信息
- **AND** 提供"使用规则引擎"备选按钮

### Requirement: 进度条样式应用

settings_panel SHALL 应用共享样式到进度条。

#### Scenario: 进度条样式

- **WHEN** 创建 DownloadProgressBar
- **THEN** 系统 SHALL 应用 SharedUIScripts 样式
- **AND** 进度条颜色 SHALL 使用 COLOR_BUTTON_PRESSED
- **AND** 背景色 SHALL 使用 COLOR_BG_INPUT