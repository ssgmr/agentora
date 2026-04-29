# 功能规格说明 - 模型下载器

## ADDED Requirements

### Requirement: GGUF 模型 HTTP 下载

系统 SHALL 提供 HTTP 下载模块，从 CDN 下载 GGUF 模型文件，支持实时进度显示。

#### Scenario: 开始下载模型

- **WHEN** 用户在引导页面选择下载模型
- **THEN** 系统 SHALL 向 Bridge 发送 download_model 命令
- **AND** Bridge SHALL 启动异步下载任务
- **AND** 下载进度信号 SHALL 定期发送到 Godot

#### Scenario: 下载进度显示

- **WHEN** 下载进行中
- **THEN** Godot SHALL 接收 download_progress 信号
- **AND** 信号参数 SHALL 包含：downloaded_mb、total_mb、speed_mbps
- **AND** 进度条 SHALL 实时更新显示

#### Scenario: 下载成功完成

- **WHEN** 下载完成
- **THEN** Bridge SHALL 发送 model_download_complete 信号
- **AND** 信号参数 SHALL 包含模型文件路径
- **AND** Godot SHALL 显示"下载完成"提示
- **AND** 模型选择项 SHALL 标记为已下载

#### Scenario: 下载失败处理

- **WHEN** 下载失败（网络错误、文件损坏）
- **THEN** Bridge SHALL 发送 model_download_failed 信号
- **AND** 信号参数 SHALL 包含错误描述
- **AND** Godot SHALL 显示错误提示，提供重试选项

### Requirement: CDN 源优先级

模型下载 SHALL 优先使用国内 CDN，失败时自动切换备用源。

#### Scenario: ModelScope 优先下载

- **WHEN** 用户在中国大陆
- **THEN** 系统 SHALL 优先从 ModelScope CDN 下载
- **AND** 下载 URL SHALL 为 modelscope.cn 域名

#### Scenario: HuggingFace 备用下载

- **WHEN** ModelScope 下载失败或超时
- **THEN** 系统 SHALL 自动切换到 HuggingFace CDN
- **AND** 使用备用 URL 重试下载

#### Scenario: CDN 源配置

- **WHEN** 预置模型列表定义
- **THEN** 每个模型 SHALL 包含 primary_url 和 fallback_url
- **AND** URL SHALL 为直接下载链接（非页面链接）

### Requirement: 预置模型列表

系统 SHALL 提供预置模型列表，显示模型名称、大小、描述和下载状态。

#### Scenario: 获取可用模型列表

- **WHEN** 引导页面加载
- **THEN** 系统 SHALL 调用 Bridge.get_available_models()
- **AND** 返回列表 SHALL 包含预置模型信息
- **AND** 每个模型 SHALL 包含：name、size_mb、description、download_status

#### Scenario: 模型信息展示

- **WHEN** 显示模型选择列表
- **THEN** 每个模型选项 SHALL 显示：
  - 模型名称（如 Qwen3.5-2B-Q4_K_M）
  - 模型大小（如 ~1.5GB）
  - 性能描述（如 "首token <100ms，推荐移动端"）
  - 下载状态（已下载/未下载/下载中）

#### Scenario: 预置模型规格

- **WHEN** 系统定义预置模型
- **THEN** SHALL 包含至少以下模型：
  - Qwen3.5-2B-Q4_K_M (~1.5GB) - 推荐移动端
  - Gemma-4-2B-Q4_K_M (~1.2GB) - 轻量备选
  - Qwen3.5-4B-Q4_K_M (~2.5GB) - 高性能（桌面端推荐）

### Requirement: 下载取消和暂停

下载 SHALL 支持用户取消和暂停操作。

#### Scenario: 取消下载

- **WHEN** 用户点击取消下载按钮
- **THEN** 系统 SHALL 终止下载任务
- **AND** 删除部分下载的临时文件
- **AND** 模型状态 SHALL 恢复为"未下载"

#### Scenario: 暂停下载（可选）

- **WHEN** 用户点击暂停按钮
- **THEN** 系统 SHALL 暂停下载任务
- **AND** 保存已下载部分到临时文件
- **AND** 恢复时 SHALL 从断点继续下载

### Requirement: 模型文件存储

下载的模型文件 SHALL 存储到指定目录，便于后续加载。

#### Scenario: 模型存储路径

- **WHEN** 下载完成
- **THEN** 模型 SHALL 存储到 models/ 目录
- **AND** 文件名 SHALL 与模型名称一致（如 Qwen3.5-2B-Q4_K_M.gguf）

#### Scenario: 存储路径配置

- **WHEN** 配置本地模型路径
- **THEN** 默认路径 SHALL 为 models/<model_name>.gguf
- **AND** 用户 SHALL 可自定义存储目录