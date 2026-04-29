# 功能规格说明 - download-progress-ui

## ADDED Requirements

### Requirement: 下载进度信号传递

Bridge SHALL 将模型下载进度通过信号传递到 Godot 客户端。

#### Scenario: 下载进度信号格式

- **WHEN** 模型下载过程中
- **THEN** Bridge SHALL 发送 download_progress 信号
- **AND** 信号参数 SHALL 包含：
  - downloaded_mb: 已下载大小（MB）
  - total_mb: 总大小（MB）
  - speed_mbps: 下载速度（MB/s）
  - model_name: 模型名称

#### Scenario: 进度更新频率

- **WHEN** 下载进行中
- **THEN** Bridge SHALL 每 0.5 秒发送一次进度信号
- **AND** 进度变化小于 1% 时可跳过发送

### Requirement: 客户端进度条显示

Godot 客户端 SHALL 显示可视化下载进度条。

#### Scenario: 进度条更新

- **WHEN** 收到 download_progress 信号
- **THEN** ProgressBar.value SHALL 设置为 percent
- **AND** 进度条颜色 SHALL 使用主题色 COLOR_BUTTON_PRESSED

#### Scenario: 下载状态文本

- **WHEN** 下载进行中
- **THEN** 客户端 SHALL 显示文本：
  - "已下载: X MB / Y MB"
  - "速度: Z MB/s"
  - 百分比 "N%"

#### Scenario: 取消下载按钮

- **WHEN** 下载进行中
- **THEN** 客户端 SHALL 显示"取消下载"按钮
- **AND** 点击按钮 SHALL 调用 Bridge.cancel_download()
- **AND** 取消后进度条 SHALL 重置

### Requirement: 下载完成状态显示

客户端 SHALL 显示下载完成状态。

#### Scenario: 下载成功显示

- **WHEN** 收到 model_download_complete 信号
- **THEN** 进度条 SHALL 填满并显示"下载完成"
- **AND** 模型选项 SHALL 标记为"已下载 ✅"
- **AND** 显示"使用此模型"按钮

#### Scenario: 下载失败显示

- **WHEN** 收到 model_download_failed 信号
- **THEN** 进度条 SHALL 显示红色
- **AND** 显示错误提示
- **AND** 提供"重新下载"按钮

### Requirement: 预置模型列表显示

客户端 SHALL 显示预置模型列表及其下载状态。

#### Scenario: 模型列表加载

- **WHEN** settings_panel 显示本地模型区域
- **THEN** 客户端 SHALL 调用 Bridge.get_available_models()
- **AND** 显示模型列表，包含：名称、大小、描述

#### Scenario: 检测已下载模型

- **WHEN** 显示模型列表
- **THEN** 客户端 SHALL 检测 models/<filename>.gguf 是否存在
- **AND** 已存在的模型 SHALL 标记"已下载"
- **AND** 未存在的模型 SHALL 显示"下载"按钮

#### Scenario: 模型选择状态

- **WHEN** 用户点击已下载模型
- **THEN** 模型 SHALL 高亮选中
- **AND** 配置中 llm_local_model_path SHALL 更新为该模型路径