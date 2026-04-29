# 功能规格说明 - bridge-api（增量）

## ADDED Requirements

### Requirement: 模型加载信号

Bridge SHALL 新增模型加载相关信号。

#### Scenario: model_load_start 信号定义

- **WHEN** 定义 Bridge 信号
- **THEN** 系统 SHALL 新增 #[signal] fn model_load_start(model_name: GString, estimated_time: f64)

#### Scenario: model_load_progress 信号定义

- **WHEN** 定义 Bridge 信号
- **THEN** 系统 SHALL 新增 #[signal] fn model_load_progress(phase: GString, progress: f64, model_name: GString)

#### Scenario: model_load_complete 信号定义

- **WHEN** 定义 Bridge 信号
- **THEN** 系统 SHALL 新增 #[signal] fn model_load_complete(model_name: GString, backend: GString, memory_mb: f64)

#### Scenario: model_load_failed 信号定义

- **WHEN** 定义 Bridge 信号
- **THEN** 系统 SHALL 新增 #[signal] fn model_load_failed(model_name: GString, error: GString)

### Requirement: 下载进度信号扩展

Bridge SHALL 扩展现有下载进度信号。

#### Scenario: download_progress 信号扩展

- **WHEN** 发送 download_progress 信号
- **THEN** 信号 SHALL 包含 model_name 参数
- **AND** 参数顺序 SHALL 为：downloaded_mb, total_mb, speed_mbps, model_name

### Requirement: GPU 后端查询 API

Bridge SHALL 新增 GPU 后端状态查询方法。

#### Scenario: get_gpu_backend 方法

- **WHEN** 调用 Bridge.get_gpu_backend()
- **THEN** 系统 SHALL 返回当前使用的 GPU 后端名称
- **AND** 返回值 SHALL 为 "metal" | "vulkan" | "cuda" | "cpu"

#### Scenario: get_gpu_backend_info 方法

- **WHEN** 调用 Bridge.get_gpu_backend_info()
- **THEN** 系统 SHALL 返回 Dictionary 包含：
  - backend: 后端名称
  - n_gpu_layers: GPU 层数
  - available_memory_mb: 可用内存