# 需求说明书

## 背景概述

当前项目已有本地 GGUF 推理骨架实现（`crates/ai/src/llama.rs`），使用 llama-cpp-2 bindings，但仅有占位代码，无法实际推理。客户端配置页面（`settings_panel.gd`）已有本地模型选项和下载按钮，但进度显示仅通过日志输出，缺少可视化进度条。用户配置系统（`user_config.rs`）支持 `llm.mode` 选择，但 Provider 创建逻辑尚未集成本地推理。

项目目标支持 PC 和移动端跨平台运行，需要一种 GPU 后端方案覆盖 Windows（CUDA/Vulkan）、macOS（Metal）、Android（Vulkan）。预编译运行环境（DLL）随应用打包，模型文件按需动态下载。

## 变更目标

- 实现 llama-cpp-2 Provider 完整推理能力，支持跨平台 GPU 加速
- 与客户端配置页面集成，显示下载进度条和模型加载进度
- 用户配置驱动的 Provider 选择，本地推理不可用时自动降级到规则引擎
- 预编译 llama.cpp DLL 随应用打包，模型从 CDN 动态下载

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `llama-provider` | llama-cpp-2 本地推理 Provider，支持 Metal/Vulkan/CUDA/CPU 后端自动检测和选择 |
| `gpu-backend-detection` | 跨平台 GPU 后端检测，根据平台和硬件自动选择最优后端 |
| `model-load-progress` | 模型加载进度反馈，通过 Bridge 信号传递到客户端显示 |
| `download-progress-ui` | 模型下载进度条 UI，显示已下载大小、总大小、下载速度 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `llm-config-loader` | 扩展配置加载逻辑，支持 UserConfig.llm.mode 决定 Provider 创建 |
| `bridge-api` | 新增信号：model_load_progress、model_load_complete、model_load_failed |
| `settings-panel-ui` | 扩展本地模型区域，添加下载进度条和加载状态显示 |

## 影响范围

- **代码模块**：
  - `crates/ai/src/llama.rs` - 完整实现 LlamaProvider
  - `crates/ai/src/lib.rs` - 导出 LlamaProvider
  - `crates/ai/Cargo.toml` - 配置 llama-cpp-2 feature flags
  - `crates/bridge/src/bridge.rs` - 扩展 Provider 创建逻辑，新增加载信号
  - `crates/bridge/src/simulation_runner.rs` - 传递 UserConfig 到 Provider 创建
  - `client/scripts/settings_panel.gd` - 添加进度条 UI 逻辑

- **API接口**：
  - 新增 Bridge 信号：`model_load_progress(phase, progress, model_name)`
  - 新增 Bridge 信号：`model_load_complete(model_name, backend)`
  - 新增 Bridge 信号：`model_load_failed(model_name, error)`
  - 修改 `download_model()` 支持进度信号传递

- **依赖组件**：
  - llama-cpp-2 crate（已添加，feature: local-inference）
  - 预编译 llama.dll / libllama.so（随应用打包）

- **关联系统**：
  - ModelScope CDN / HuggingFace CDN（模型下载源）
  - Vulkan SDK（Windows/Android GPU 后端）
  - Metal Framework（macOS/iOS GPU 后端，系统自带）
  - CUDA Runtime（Windows/Linux NVIDIA GPU，用户可选安装）

## 验收标准

- [ ] LlamaProvider 能成功加载 GGUF 模型并推理生成响应
- [ ] Windows 平台能检测并使用 CUDA 或 Vulkan 后端
- [ ] macOS 平台能使用 Metal 后端
- [ ] Android 平台能使用 Vulkan 后端（或 CPU 兜底）
- [ ] 后端不可用时自动使用 CPU 兜底
- [ ] 客户端显示下载进度条（百分比、已下载/总大小、速度）
- [ ] 客户端显示模型加载状态（加载中/已加载/加载失败）
- [ ] UserConfig.llm.mode=local 时创建 LlamaProvider
- [ ] 模型文件不存在或加载失败时降级到规则引擎
- [ ] 单元测试覆盖 Provider 创建和后端检测逻辑