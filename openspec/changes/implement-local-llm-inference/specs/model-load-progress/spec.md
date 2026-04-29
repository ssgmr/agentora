# 功能规格说明 - model-load-progress

## ADDED Requirements

### Requirement: 模型加载进度信号

系统 SHALL 通过 Bridge 信号传递模型加载进度到 Godot 客户端。

#### Scenario: 加载开始信号

- **WHEN** 开始加载 GGUF 模型
- **THEN** Bridge SHALL 发送 model_load_start 信号
- **AND** 信号参数 SHALL 包含：model_name, estimated_time

#### Scenario: 加载进度信号

- **WHEN** 模型加载过程中
- **THEN** Bridge SHALL 定期发送 model_load_progress 信号
- **AND** 信号参数 SHALL 包含：
  - phase: "reading" | "parsing" | "gpu_upload"
  - progress: 0-100 百分比
  - model_name: 模型名称

#### Scenario: 加载完成信号

- **WHEN** 模型加载成功完成
- **THEN** Bridge SHALL 发送 model_load_complete 信号
- **AND** 信号参数 SHALL 包含：model_name, backend, memory_used

#### Scenario: 加载失败信号

- **WHEN** 模型加载失败
- **THEN** Bridge SHALL 发送 model_load_failed 信号
- **AND** 信号参数 SHALL 包含：model_name, error_code, error_message

### Requirement: 进度估算算法

系统 SHALL 使用估算算法提供加载进度（llama-cpp-2 同步加载无法获取真实进度）。

#### Scenario: 文件读取阶段估算

- **WHEN** 加载开始
- **THEN** 系统 SHALL 估算文件读取阶段进度
- **AND** 根据模型文件大小估算时间
- **AND** 进度范围 SHALL 为 0-30%

#### Scenario: 权重解析阶段估算

- **WHEN** 文件读取完成
- **THEN** 系统 SHALL 估算权重解析阶段进度
- **AND** 进度范围 SHALL 为 30-70%

#### Scenario: GPU 上传阶段估算

- **WHEN** 权重解析完成
- **AND** 使用 GPU 后端
- **THEN** 系统 SHALL 估算 GPU 上传阶段进度
- **AND** 进度范围 SHALL 为 70-100%

#### Scenario: CPU 模式无 GPU 上传

- **WHEN** 权重解析完成
- **AND** 使用 CPU 后端
- **THEN** 系统 SHALL 直接标记完成
- **AND** 进度 SHALL 为 100%

### Requirement: 客户端进度显示

Godot 客户端 SHALL 显示模型加载进度。

#### Scenario: 显示加载进度条

- **WHEN** 收到 model_load_progress 信号
- **THEN** 客户端 SHALL 更新进度条显示
- **AND** 显示阶段文本（读取权重 / 解析模型 / GPU 加载）
- **AND** 显示百分比进度

#### Scenario: 显示加载完成状态

- **WHEN** 收到 model_load_complete 信号
- **THEN** 客户端 SHALL 显示"模型已加载"状态
- **AND** 显示使用的 GPU 后端
- **AND** 显示内存占用

#### Scenario: 显示加载失败状态

- **WHEN** 收到 model_load_failed 信号
- **THEN** 客户端 SHALL 显示加载失败提示
- **AND** 显示错误原因
- **AND** 提供"使用规则引擎"选项