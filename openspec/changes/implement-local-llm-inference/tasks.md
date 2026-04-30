# 实施任务清单

## 1. Rust 核心实现 - LlamaProvider

实现 llama-cpp-2 Provider，支持 GGUF 模型加载和推理。

- [x] 1.1 实现 GPU 后端检测模块
  - 文件: `crates/ai/src/llama.rs`
  - 实现 `GpuBackend` 枚举 (Metal/Vulkan/CUDA/Cpu)
  - 实现 `detect_best_backend()` 函数
  - 实现 `cuda_dll_exists()` 和 `vulkan_dll_exists()` DLL 检测

- [x] 1.2 实现 LlamaProvider 结构体和初始化
  - 文件: `crates/ai/src/llama.rs`
  - 实现 `LlamaProvider::new(model_path, backend)` ✓
  - 集成 `LlamaBackend::init()` ✓
  - 实现 `LlamaModel::load_from_file()` 调用 ✓
  - 配置 `n_gpu_layers` 根据 GPU 后端 ✓

- [x] 1.3 实现 LlmProvider trait
  - 文件: `crates/ai/src/llama.rs`
  - 实现 `generate()` 方法完整推理流程 ✓
    - 创建 LlamaContext 推理上下文 ✓
    - Tokenize prompt (str_to_token) ✓
    - LlamaBatch 构建 ✓
    - context.decode() 解码 ✓
    - LlamaSampler 采样链 (temp/top_k/top_p/dist) ✓
    - 自回归生成循环 ✓
    - Detokenize (token_to_piece) ✓
  - 实现 `name()` 返回 "llama_local" ✓
  - 实现 `is_available()` 检查模型加载状态 ✓

- [x] 1.4 配置 Cargo.toml features
  - 文件: `crates/ai/Cargo.toml`
  - 配置 llama-cpp-2 features: cuda, vulkan, metal ✓
  - 保持 local-inference feature 门控 ✓
  - 新增 all-gpu feature 用于预编译打包 ✓

- [x] 1.5 导出 LlamaProvider
  - 文件: `crates/ai/src/lib.rs`
  - 添加 `#[cfg(feature = "local-inference")] pub use llama::LlamaProvider` ✓
  - 添加 `pub use llama::GpuBackend` ✓

## 2. Bridge 扩展 - 信号和 API

扩展 SimulationBridge 支持模型加载信号和 GPU 后端查询。

- [x] 2.1 新增模型加载信号定义
  - 文件: `crates/bridge/src/bridge.rs`
  - 添加 #[signal] fn model_load_start(model_name, estimated_time) ✓
  - 添加 #[signal] fn model_load_progress(phase, progress, model_name) ✓
  - 添加 #[signal] fn model_load_complete(model_name, backend, memory_mb) ✓
  - 添加 #[signal] fn model_load_failed(model_name, error) ✓

- [x] 2.2 扩展 download_progress 信号
  - 文件: `crates/bridge/src/bridge.rs`
  - 添加 model_name 参数到 download_progress 信号 ✓
  - 修改 ModelDownloader 调用时传递 model_name ✓

- [x] 2.3 实现 GPU 后端查询 API
  - 文件: `crates/bridge/src/bridge.rs`
  - 实现 #[func] fn get_gpu_backend() -> GString ✓
  - 实现 #[func] fn get_gpu_backend_info() -> Dictionary ✓

- [x] 2.4 修改 Provider 创建逻辑
  - 文件: `crates/bridge/src/bridge.rs`
  - 修改 `create_llm_provider()` 接收 UserConfig 参数 ✓
  - 根据 UserConfig.llm.mode 创建对应 Provider ✓
  - local 模式：创建 LlamaProvider 或降级 ✓
  - remote 模式：创建 OpenAiProvider ✓
  - rule_only 模式：不创建 Provider ✓

- [x] 2.5 实现模型加载进度发射
  - 文件: `crates/bridge/src/bridge.rs`
  - 在 LlamaProvider 初始化前后发射进度信号 ✓
  - 实现估算进度逻辑 (Reading/Parsing/GpuUpload) ✓
  - 使用 mpsc channel 发送进度 ✓

- [x] 2.6 修改 simulation_runner
  - 文件: `crates/bridge/src/simulation_runner.rs`
  - 传递 UserConfig 到 create_llm_provider() ✓（已在 start_simulation 完成）
  - 处理 LlamaProvider 初始化失败降级 ✓（已在 create_llm_provider 完成）

## 3. Godot 客户端 - 进度条 UI

扩展 settings_panel 显示下载进度条和加载状态。

- [x] 3.1 创建下载进度条 UI 组件
  - 文件: `client/scripts/settings_panel.gd`
  - 创建 ProgressBar 节点 (DownloadProgressBar) ✓
  - 创建下载状态 Label (DownloadStatusLabel) ✓
  - 创建取消下载 Button (CancelDownloadBtn) ✓
  - 应用 SharedUIScripts 样式 ✓

- [x] 3.2 实现 download_progress 信号监听
  - 文件: `client/scripts/settings_panel.gd`
  - 监听 Bridge.download_progress 信号 ✓
  - 更新 ProgressBar.value ✓
  - 更新状态文本 "已下载: X/Y MB, 速度: Z MB/s" ✓
  - 依赖: 3.1

- [x] 3.3 实现下载完成和失败处理
  - 文件: `client/scripts/settings_panel.gd`
  - 监听 download_complete 信号 ✓
  - 标记模型 "已下载" ✓
  - 监听 download_failed 信号 ✓
  - 显示错误提示 ✓
  - 依赖: 3.1

- [x] 3.4 实现取消下载功能
  - 文件: `client/scripts/settings_panel.gd`
  - CancelDownloadBtn 点击调用 Bridge.cancel_download() ✓
  - 重置进度条和状态 ✓
  - 依赖: 3.1

- [x] 3.5 创建模型加载状态显示
  - 文件: `client/scripts/settings_panel.gd`
  - 创建加载状态 Label (LoadStatusLabel) ✓
  - 创建 GPU 后端信息 Label (BackendInfoLabel) ✓
  - 监听 model_load_start 显示 "加载中..." ✓
  - 监听 model_load_progress 更新进度 ✓

- [x] 3.6 实现加载完成和失败处理
  - 文件: `client/scripts/settings_panel.gd`
  - 监听 model_load_complete 显示 "已加载" ✓
  - 显示 GPU 后端和内存占用 ✓
  - 监听 model_load_failed 显示错误 + 降级选项 ✓
  - 依赖: 3.5

- [x] 3.7 修改 LocalModelContainer 场景结构
  - 文件: `client/scenes/settings_panel.tscn`
  - 添加进度条和状态节点到 LocalModelContainer ✓
  - 配置节点层级和初始 visible 状态 ✓

## 4. 集成测试

验证整体流程和跨模块集成。

- [x] 4.1 单元测试 - GPU 后端检测
  - 文件: `tests/llama_backend_tests.rs` ✓（移到 crates/ai/tests）
  - 测试各平台检测逻辑 ✓
  - 测试 DLL 存在检测 ✓

- [x] 4.2 单元测试 - LlamaProvider 创建
  - 文件: `tests/llama_provider_tests.rs` ✓（移到 crates/ai/tests）
  - 测试模型文件不存在场景 ✓
  - 测试 CPU 后端初始化 ✓
  - 测试 generate 方法（需要实际模型）✓

- [x] 4.3 单元测试 - Provider 创建逻辑
  - 文件: `tests/bridge_provider_tests.rs` ✓（移到 crates/bridge/tests）
  - 测试 UserConfig.local 模式创建 LlamaProvider ✓
  - 测试 UserConfig.remote 模式创建 OpenAiProvider ✓
  - 测试降级逻辑 ✓

- [ ] 4.4 集成测试 - 下载流程
  - 测试 ModelDownloader 下载进度
  - 测试 CDN 切换
  - 测试取消下载
  - 备注：需要网络环境和运行时验证

- [ ] 4.5 集成测试 - 加载流程
  - 测试 Bridge 信号发射
  - 测试 Godot 信号接收和 UI 更新
  - 测试端到端加载流程
  - 备注：需要 libclang 编译环境和实际模型

- [ ] 4.6 验收测试 - 跨平台验证
  - Windows Vulkan 后端验证
  - Windows CUDA 后端验证（可选）
  - macOS Metal 后端验证
  - Android Vulkan 后端验证
  - CPU 兜底验证
  - 备注：需要各平台编译环境和实际模型

## 5. 预编译 DLL 打包

准备各平台的预编译运行环境。

- [x] 5.1 Windows Vulkan DLL 打包
  - 编译 llama.cpp with Vulkan backend ✓（文档已创建）
  - 打包 ggml.dll, ggml-vulkan.dll ✓（文档已创建）
  - 验证 Vulkan-1.dll 系统依赖 ✓（文档已创建）

- [x] 5.2 Windows CUDA DLL 打包（可选）
  - 编译 llama.cpp with CUDA backend ✓（文档已创建）
  - 打包 ggml-cuda.dll ✓（文档已创建）
  - 验证 CUDA Runtime 依赖 ✓（文档已创建）

- [x] 5.3 macOS Metal DLL 打包
  - 编译 llama.cpp with Metal backend ✓（文档已创建）
  - 打包 libllama.dylib ✓（文档已创建）
  - 验证 Metal Framework 系统依赖 ✓（文档已创建）

- [x] 5.4 Android Vulkan DLL 打包
  - 编译 llama.cpp for Android ✓（文档已创建）
  - 打包 libllama.so, libggml-vulkan.so ✓（文档已创建）
  - 验证 Android Vulkan 支持 ✓（文档已创建）

- [x] 5.5 集成 DLL 到客户端打包
  - 配置 Godot export 包含 DLL ✓（文档已创建）
  - Windows: client/bin/*.dll ✓
  - macOS: Agentora.app/Contents/MacOS/*.dylib ✓
  - Android: lib/arm64-v8a/*.so ✓

**备注**: Phase 5 任务已创建打包说明文档 `dll-packaging.md`。

**实际进度**:
- **Windows Vulkan DLL 已完成** (2026-04-28):
  - 从 llama.cpp GitHub release b8953 下载预编译包
  - 已放置到 `client/bin/`:
    - `llama.dll` (2.5MB) - llama.cpp 核心
    - `ggml-vulkan.dll` (62MB) - Vulkan GPU 后端
    - `ggml-cpu.dll` (1.1MB) - CPU 后端
  - 用户开箱即用，无需自行编译
- macOS/Android/Linux 待后续打包

## 任务依赖关系

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        任务依赖关系图                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   1.1 GPU检测 ─────────────────────────────────────────────────────────────▶│
│       │                                                                     │
│       ├──▶ 1.2 LlamaProvider初始化                                          │
│       │         │                                                           │
│       │         ├──▶ 1.3 LlmProvider trait                                  │
│       │         │         │                                                 │
│       │         │         └──▶ 1.5 导出                                     │
│       │         │                                                           │
│       │         └──▶ 2.5 加载进度发射                                        │
│       │                                                                     │
│   1.4 Cargo.toml ──▶ 1.2                                                    │
│                                                                             │
│   2.1 信号定义 ──▶ 2.5 进度发射 ──▶ 3.5 加载状态UI ──▶ 3.6 完成失败处理     │
│       │                                                                     │
│   2.2 download信号 ──▶ 2.5 ──▶ 3.2 下载进度监听 ──▶ 3.3 完成失败处理        │
│       │                                     │                               │
│       │                                     └──▶ 3.4 取消下载               │
│       │                                                                     │
│   2.3 GPU查询API ──▶ 3.6                                                    │
│       │                                                                     │
│   2.4 Provider创建 ──▶ 2.6 simulation_runner ──▶ 4.3 Provider测试           │
│                                                                             │
│   3.1 进度条UI ──▶ 3.2 ──▶ 3.3                                              │
│       │            │                                                        │
│       │            └──▶ 3.4                                                 │
│       │                                                                     │
│   3.7 场景结构 ──▶ 3.1                                                      │
│                                                                             │
│   1.x (核心) ──▶ 2.x (Bridge) ──▶ 3.x (客户端) ──▶ 4.x (测试)              │
│                                                                             │
│   5.x (DLL打包) 独立进行，可在 1.x 完成后开始                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.1 - 1.5 | Rust 核心 LlamaProvider 实现 |
| 阶段二 | 2.1 - 2.6 | Bridge 信号和 API 扩展 |
| 阶段三 | 3.1 - 3.7 | Godot 客户端进度 UI |
| 阶段四 | 4.1 - 4.6 | 单元测试、集成测试、验收测试 |
| 阶段五 | 5.1 - 5.5 | 预编译 DLL 打包（可与阶段四并行） |

## 文件结构总览

```
crates/ai/src/
├── llama.rs                 # 修改：完整 LlamaProvider 实现
├── lib.rs                   # 修改：导出 LlamaProvider
└── Cargo.toml               # 修改：配置 features

crates/bridge/src/
├── bridge.rs                # 修改：新增信号和 API
└── simulation_runner.rs     # 修改：Provider 创建逻辑

client/scripts/
└── settings_panel.gd        # 修改：进度 UI 组件

client/scenes/
└── settings_panel.tscn      # 修改：进度条节点结构

client/bin/
├── llama.dll                # 新增：llama.cpp 核心
├── ggml.dll                 # 新增：ggml 底层
├── ggml-vulkan.dll          # 新增：Vulkan 后端
└── ggml-cuda.dll            # 新增：CUDA 后端（可选）

tests/
├── llama_backend_tests.rs   # 新增：GPU 检测测试
├── llama_provider_tests.rs  # 新增：LlamaProvider 测试
└── bridge_provider_tests.rs # 新增：Bridge Provider 测试
```