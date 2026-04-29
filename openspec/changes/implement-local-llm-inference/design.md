# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

**项目架构**：Agentora 是跨平台（PC + 移动端）去中心化文明模拟器，采用 Rust 核心 + Godot 4 GDExtension 桥接架构。

- **核心引擎**（`crates/core`）：World、Agent、DecisionPipeline、Memory、Strategy
- **AI 层**（`crates/ai`）：LlmProvider trait、OpenAI/Anthropic Provider、FallbackChain
- **桥接层**（`crates/bridge`）：SimulationBridge GDExtension、mpsc 通道、UserConfig
- **客户端**（`client`）：Godot 4 GDScript、settings_panel.gd、shared_ui_styles.gd

**现有本地推理模块**：
- `crates/ai/src/llama.rs`：骨架实现，仅检查文件存在
- `crates/ai/src/model_downloader.rs`：完整实现，支持 ModelScope/HuggingFace CDN
- `crates/ai/Cargo.toml`：已添加 llama-cpp-2 依赖（feature: local-inference）

**约束条件**：
- llama-cpp-2 是 llama.cpp 的 Rust binding，需要 C++ 编译环境
- 模型加载是同步操作，无法获取中间进度
- 需支持跨平台 GPU 后端：Metal（macOS）、Vulkan（Windows/Android）、CUDA（Windows NVIDIA）

### 1.2 现状分析

**已完成**：
- ModelDownloader 流式下载 + 进度计算
- UserConfig 配置结构和持久化
- Bridge 基础信号框架（download_progress 等）
- settings_panel.gd 本地模型 UI 骨架

**未完成**：
- LlamaProvider 实际推理实现
- GPU 后端检测和选择逻辑
- 模型加载进度反馈机制
- 进度条 UI 可视化显示
- Provider 创建流程集成 UserConfig

### 1.3 关键干系人

- **Rust 核心**：ai crate（LlamaProvider）、bridge crate（信号扩展）
- **Godot 客户端**：settings_panel.gd（进度 UI）
- **外部依赖**：llama-cpp-2、Vulkan SDK、Metal Framework
- **CDN 服务**：ModelScope（国内优先）、HuggingFace（备用）

## 2. 设计目标

### 目标

- 实现 LlamaProvider 完整推理能力，支持 GGUF 模型加载和生成
- 实现跨平台 GPU 后端检测（Metal/Vulkan/CUDA/CPU）
- 设计模型加载进度估算机制，通过 Bridge 信号传递
- 集成 settings_panel 进度条 UI，显示下载和加载状态
- 实现 UserConfig 驱动的 Provider 创建，支持降级到规则引擎

### 非目标

- 不实现模型转换功能（GGUF 需预先存在）
- 不实现多语言界面
- 不实现断点续传下载（可选，后续迭代）
- 不实现自定义 GPU 后端参数配置

## 3. 整体架构

### 3.1 架构概览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        本地推理集成架构                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────┐      ┌─────────────────────┐                     │
│   │  settings_panel.gd  │      │     bridge.rs       │                     │
│   │  (Godot 客户端)      │      │   (GDExtension)     │                     │
│   ├─────────────────────┤      ├─────────────────────┤                     │
│   │                     │      │                     │                     │
│   │  • 模型列表显示     │◀────▶│  • get_user_config  │                     │
│   │  • 下载进度条       │◀────▶│  • download_model   │                     │
│   │  • 加载状态显示     │◀────▶│  • load_local_model │                     │
│   │  • GPU 后端显示     │◀────▶│  • get_gpu_backend  │                     │
│   │                     │      │                     │                     │
│   │  信号监听：         │      │  信号发射：         │                     │
│   │  download_progress  │      │  download_progress  │                     │
│   │  model_load_*       │      │  model_load_*       │                     │
│   │                     │      │                     │                     │
│   └─────────────────────┘      └─────────────────────┘                     │
│                  │                      │                                  │
│                  │                      │                                  │
│                  ▼                      ▼                                  │
│   ┌───────────────────────────────────────────────────────┐               │
│   │                    crates/ai/                          │               │
│   ├───────────────────────────────────────────────────────┤               │
│   │                                                       │               │
│   │   ┌─────────────────────┐    ┌─────────────────────┐ │               │
│   │   │    LlamaProvider    │    │   GpuBackendDetect  │ │               │
│   │   │      llama.rs       │    │    (llama.rs 内)    │ │               │
│   │   ├─────────────────────┤    ├─────────────────────┤ │               │
│   │   │ • LlamaBackend 初始化│    │ • detect_platform  │ │               │
│   │   │ • GGUF 模型加载      │    │ • check_dll_exists │ │               │
│   │   │ • LlamaContext 创建  │    │ • select_backend   │ │               │
│   │   │ • Tokenize prompt   │    │                     │ │               │
│   │   │ • Sampler chain     │    │ Metal | Vulkan |    │ │               │
│   │   │ • Generate tokens   │    │ CUDA | CPU          │ │               │
│   │   │ • Detokenize output │    │                     │ │               │
│   │   └─────────────────────┘    └─────────────────────┘ │               │
│   │                                                       │               │
│   │   ┌─────────────────────┐                            │               │
│   │   │  ModelDownloader    │  ← 已完成                  │               │
│   │   │ model_downloader.rs │                            │               │
│   │   └─────────────────────┘                            │               │
│   │                                                       │               │
│   └───────────────────────────────────────────────────────┘               │
│                                                                             │
│   预编译 DLL 打包                                                           │
│   ─────────────────────────────────────────────────────────────────────── │
│                                                                             │
│   Windows: llama.dll + ggml.dll + ggml-vulkan.dll + (ggml-cuda.dll 可选)   │
│   macOS:   libllama.dylib (Metal 内置)                                      │
│   Android: libllama.so + libggml-vulkan.so                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件名 | 文件路径 | 职责说明 |
| --- | --- | --- |
| LlamaProvider | `crates/ai/src/llama.rs` | GGUF 模型加载、推理生成、LlmProvider trait 实现 |
| GpuBackendDetect | `crates/ai/src/llama.rs` (内嵌) | 平台检测、DLL 检测、GPU 后端选择 |
| BridgeSignals | `crates/bridge/src/bridge.rs` | 模型加载信号定义和发射 |
| ProviderFactory | `crates/bridge/src/bridge.rs` | 根据 UserConfig 创建 Provider |
| DownloadProgressBar | `client/scripts/settings_panel.gd` | 下载进度 UI 组件 |
| LoadStatusDisplay | `client/scripts/settings_panel.gd` | 加载状态 UI 组件 |

### 3.3 数据流设计

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     模型下载流程                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Godot UI                                                                  │
│       │                                                                     │
│       │ 1. 用户点击下载按钮                                                  │
│       ▼                                                                     │
│   Bridge.download_model(url, dest, model_name)                              │
│       │                                                                     │
│       │ 2. 启动 tokio async task                                            │
│       ▼                                                                     │
│   ModelDownloader.download_with_fallback()                                  │
│       │                                                                     │
│       │ 3. HTTP GET (reqwest 流式下载)                                       │
│       │    ┌─ ModelScope CDN (primary)                                      │
│       │    └─ HuggingFace CDN (fallback)                                    │
│       ▼                                                                     │
│   每 0.5 秒计算进度                                                          │
│       │                                                                     │
│       │ 4. 发送 download_progress 信号                                       │
│       ▼                                                                     │
│   Bridge.emit_signal("download_progress", ...)                              │
│       │                                                                     │
│       │ 5. Godot 接收信号                                                    │
│       ▼                                                                     │
│   ProgressBar 更新 + 状态文本更新                                            │
│       │                                                                     │
│       │ 6. 下载完成                                                          │
│       ▼                                                                     │
│   Bridge.emit_signal("download_complete", path)                             │
│       │                                                                     │
│       ▼                                                                     │
│   Godot: 标记"已下载" + 启用"使用"按钮                                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     模型加载流程                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Godot UI                                                                  │
│       │                                                                     │
│       │ 1. 用户选择已下载模型                                                │
│       │    点击"开始游戏"                                                    │
│       ▼                                                                     │
│   Bridge.set_user_config(config)                                            │
│       │                                                                     │
│       │ 2. 保存 UserConfig                                                  │
│       │    llm.mode = "local"                                               │
│       │    llm.local_model_path = ".../Qwen.gguf"                           │
│       ▼                                                                     │
│   Bridge.start_simulation()                                                 │
│       │                                                                     │
│       │ 3. 加载 UserConfig                                                  │
│       │    调用 create_llm_provider(user_config)                            │
│       ▼                                                                     │
│   GpuBackendDetect::detect_best_backend()                                   │
│       │                                                                     │
│       │ 4. 检测平台 + DLL                                                   │
│       │    返回: Metal | Vulkan | CUDA | CPU                                │
│       │                                                                     │
│       │ 5. 发送 model_load_start 信号                                       │
│       ▼                                                                     │
│   LlamaProvider::new(model_path, backend)                                   │
│       │                                                                     │
│       │ 6. 发送估算进度信号                                                  │
│       │    model_load_progress("reading", 15%)                              │
│       │    model_load_progress("parsing", 50%)                              │
│       │    model_load_progress("gpu_upload", 85%)                           │
│       ▼                                                                     │
│   LlamaBackend::init()                                                      │
│   LlamaModel::load_from_file()                                              │
│   LlamaContext::new_context()                                               │
│       │                                                                     │
│       │ 7. 加载完成                                                          │
│       ▼                                                                     │
│   Bridge.emit_signal("model_load_complete", model_name, backend)            │
│       │                                                                     │
│       │ 8. Godot 接收信号                                                    │
│       ▼                                                                     │
│   显示"已加载 ✅" + GPU 后端 + 内存占用                                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 4. 详细设计

### 4.1 接口设计

#### Bridge 新增 API

| 方法名 | 参数 | 返回值 | 说明 |
| --- | --- | --- | --- |
| `load_local_model` | `model_path: GString` | `bool` | 加载本地模型（返回是否成功启动） |
| `get_gpu_backend` | 无 | `GString` | 获取当前 GPU 后端 |
| `get_gpu_backend_info` | 无 | `Dictionary` | 获取 GPU 后端详细信息 |

#### Bridge 新增信号

| 信号名 | 参数 | 说明 |
| --- | --- | --- |
| `model_load_start` | `model_name: GString, estimated_time: f64` | 模型开始加载 |
| `model_load_progress` | `phase: GString, progress: f64, model_name: GString` | 加载进度（估算） |
| `model_load_complete` | `model_name: GString, backend: GString, memory_mb: f64` | 加载完成 |
| `model_load_failed` | `model_name: GString, error: GString` | 加载失败 |

#### LlmProvider trait 扩展（无变化，LlamaProvider 实现现有 trait）

```rust
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
}
```

### 4.2 外部接口调用

#### llama-cpp-2 接口调用

| 接口 | 调用位置 | 说明 |
| --- | --- | --- |
| `LlamaBackend::init()` | `LlamaProvider::new()` | 初始化 llama backend |
| `LlamaModelParams::default().with_n_gpu_layers(n)` | `LlamaProvider::new()` | 配置 GPU 层数 |
| `LlamaModel::load_from_file()` | `LlamaProvider::new()` | 加载 GGUF 模型 |
| `LlamaContextParams::default().with_n_ctx()` | `LlamaProvider::generate()` | 配置上下文长度 |
| `LlamaModel::str_to_token()` | `LlamaProvider::generate()` | Prompt tokenize |
| `LlamaSampler::chain_simple()` | `LlamaProvider::generate()` | 创建采样链 |
| `LlamaContext::decode()` | `LlamaProvider::generate()` | 推理 batch |
| `LlamaModel::token_to_piece()` | `LlamaProvider::generate()` | Detokenize |

### 4.3 数据模型

#### GpuBackend 枚举（新增）

```rust
pub enum GpuBackend {
    Metal,    // macOS/iOS
    Vulkan,   // Windows/Linux/Android
    Cuda,     // Windows/Linux (NVIDIA)
    Cpu,      // 兜底
}
```

#### LoadPhase 枚举（新增）

```rust
pub enum LoadPhase {
    Reading,     // 文件读取 0-30%
    Parsing,     // 权重解析 30-70%
    GpuUpload,   // GPU 上传 70-100%
}
```

#### UserConfig 扩展（无变化，已存在）

```rust
pub struct LlmUserConfig {
    pub mode: String,           // "local" | "remote" | "rule_only"
    pub local_model_path: String,
    // ...
}
```

### 4.4 核心算法

#### GPU 后端检测算法

```rust
pub fn detect_best_backend() -> GpuBackend {
    // 1. macOS/iOS: 直接 Metal
    #[cfg(target_os = "macos")]
    return GpuBackend::Metal;
    
    #[cfg(target_os = "ios")]
    return GpuBackend::Metal;
    
    // 2. Windows/Linux: CUDA → Vulkan → CPU
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        // 尝试 CUDA
        if has_nvidia_gpu() && cuda_dll_exists() {
            return GpuBackend::Cuda;
        }
        // 尝试 Vulkan
        if vulkan_dll_exists() {
            return GpuBackend::Vulkan;
        }
        return GpuBackend::Cpu;
    }
    
    // 3. Android: Vulkan → CPU
    #[cfg(target_os = "android")]
    {
        if vulkan_dll_exists() {
            return GpuBackend::Vulkan;
        }
        return GpuBackend::Cpu;
    }
    
    // 其他平台
    #[cfg(not(any(...)))]
    return GpuBackend::Cpu;
}

fn cuda_dll_exists() -> bool {
    // Windows: 检测 ggml-cuda.dll + cudart64_*.dll
    // Linux: 检测 libggml-cuda.so
}

fn vulkan_dll_exists() -> bool {
    // Windows: 检测 ggml-vulkan.dll
    // Linux/Android: 检测 libggml-vulkan.so
}
```

#### LlamaProvider 初始化算法

```rust
pub fn new(model_path: String) -> Result<Self, LlmError> {
    // 1. 检查文件存在
    if !Path::new(&model_path).exists() {
        return Err(LlmError::ConfigError("模型文件不存在"));
    }
    
    // 2. 检测 GPU 后端
    let backend = detect_best_backend();
    let n_gpu_layers = match backend {
        GpuBackend::Metal | GpuBackend::Vulkan | GpuBackend::Cuda => 1000,
        GpuBackend::Cpu => 0,
    };
    
    // 3. 初始化 Backend
    let backend = LlamaBackend::init()?;
    
    // 4. 配置模型参数
    let model_params = LlamaModelParams::default()
        .with_n_gpu_layers(n_gpu_layers);
    
    // 5. 加载模型
    let model = LlamaModel::load_from_file(
        &backend,
        Path::new(&model_path),
        &model_params,
    )?;
    
    Ok(Self { model, backend, model_path })
}
```

#### 推理生成算法

```rust
async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
    // 1. 创建推理上下文
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(4096))
        .with_n_threads(num_cpus::get());
    let ctx = self.model.new_context(&self.backend, ctx_params)?;
    
    // 2. Tokenize prompt
    let tokens = self.model.str_to_token(&request.prompt, AddBos::Always)?;
    
    // 3. 创建采样链
    let sampler = LlamaSampler::chain_simple([
        LlamaSampler::temp(request.temperature),
        LlamaSampler::top_k(40),
        LlamaSampler::top_p(0.95, 1),
        LlamaSampler::dist(42),
    ]);
    
    // 4. 初始化 batch
    let mut batch = LlamaBatch::new(512, 1);
    for (i, token) in tokens.iter().enumerate() {
        batch.add(*token, i as i32, &[0], i == tokens.len() - 1)?;
    }
    
    // 5. 首次 decode
    ctx.decode(&mut batch)?;
    
    // 6. 生成循环
    let mut output_tokens = Vec::new();
    for _ in 0..request.max_tokens {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        if self.model.is_eog_token(token) { break; }
        
        sampler.accept(token);
        output_tokens.push(token);
        
        batch.clear();
        batch.add(token, output_tokens.len() as i32, &[0], true)?;
        ctx.decode(&mut batch)?;
    }
    
    // 7. Detokenize
    let output = output_tokens.iter()
        .map(|t| self.model.token_to_piece(*t, ...))
        .collect::<Result<Vec<_>, _>>()?
        .join("");
    
    Ok(LlmResponse {
        raw_text: output,
        parsed_action: None,
        usage: TokenUsage::default(),
        provider_name: "llama_local",
    })
}
```

#### 加载进度估算算法

```rust
// 在 Bridge 线程中，启动加载时发送估算进度
fn estimate_load_progress(model_size_mb: u32, backend: &GpuBackend) {
    let phases = match backend {
        GpuBackend::Cpu => vec![
            ("reading", 0.0, 30.0),
            ("parsing", 30.0, 100.0),
        ],
        _ => vec![
            ("reading", 0.0, 30.0),
            ("parsing", 30.0, 70.0),
            ("gpu_upload", 70.0, 100.0),
        ],
    };
    
    // 使用 Timer 定时发送进度信号
    let total_time_estimate = model_size_mb * 10; // 假设 10ms/MB
    for (phase, start, end) in phases {
        let duration = (end - start) / 100.0 * total_time_estimate;
        // 每 100ms 发送一次进度更新
        for progress in (start..end).step_by(5) {
            emit_signal("model_load_progress", phase, progress, model_name);
            sleep(100ms);
        }
    }
}
```

### 4.5 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| 模型文件不存在 | 返回 ConfigError，记录日志，降级规则引擎 |
| GPU DLL 缺失 | 使用 CPU 兜底，记录警告日志 |
| 内存不足（OOM） | 拒绝加载，返回 ProviderUnavailable，降级规则引擎 |
| 推理超时 | 取消推理，返回 Timeout 错误，降级规则引擎 |
| 模型加载失败 | 发送 model_load_failed 信号，显示错误提示 |
| 下载网络失败 | 自动切换 CDN（ModelScope → HuggingFace），最终失败显示错误 |

### 4.6 前端设计

#### 技术栈

- **框架**：Godot 4 GDScript
- **UI 组件**：ProgressBar、Label、Button、TextureButton
- **样式系统**：shared_ui_styles.gd（SharedUIScripts）

#### 目录结构

```
client/
├── scenes/
│   └── settings_panel.tscn    # 现有设置面板（修改）
├── scripts/
│   ├── settings_panel.gd      # 现有脚本（扩展）
│   └── shared_ui_styles.gd    # 共享样式（复用）
└── assets/
    └── textures/
        └── icons/
            ├── download-icon.png
            └── loading-spinner.png
```

#### 组件设计

| 组件名 | 类型 | 文件路径 | 说明 |
| --- | --- | --- | --- |
| DownloadProgressBar | ProgressBar | settings_panel.gd 内嵌 | 下载进度条 |
| DownloadStatusLabel | Label | settings_panel.gd 内嵌 | 下载状态文本 |
| CancelDownloadBtn | Button | settings_panel.gd 内嵌 | 取消下载按钮 |
| LoadStatusLabel | Label | settings_panel.gd 内嵌 | 加载状态文本 |
| BackendInfoLabel | Label | settings_panel.gd 内嵌 | GPU 后端信息 |

#### 进度条样式

```gdscript
# 在 settings_panel.gd 中创建进度条
func _create_download_progress_bar() -> ProgressBar:
    var bar = ProgressBar.new()
    bar.name = "DownloadProgressBar"
    bar.min_value = 0
    bar.max_value = 100
    bar.value = 0
    bar.show_percentage = false
    bar.custom_minimum_size = Vector2(0, 24)
    
    # 应用样式
    var style = StyleBoxFlat.new()
    style.bg_color = SharedUIScripts.COLOR_BUTTON_PRESSED
    bar.add_theme_stylebox_override("fill", style)
    
    return bar
```

#### 交互逻辑

```
1. 用户点击模型下载按钮
   → Button 进入 loading 状态
   → 显示进度条 + 状态 Label + 取消按钮
   → 调用 Bridge.download_model()

2. 下载进行中
   → 监听 download_progress 信号
   → 更新 ProgressBar.value
   → 更新 Label 文本 "已下载: X/Y MB, 速度: Z MB/s"

3. 下载完成
   → 进度条填满
   → 隐藏取消按钮
   → 模型选项标记"已下载 ✅"
   → 启用"使用此模型"按钮

4. 用户点击开始游戏
   → 调用 Bridge.set_user_config()
   → 调用 Bridge.start_simulation()
   → 监听 model_load_* 信号
   → 显示加载状态

5. 加载完成
   → 显示"已加载 ✅"
   → 显示 GPU 后端信息
   → 启用进入游戏
```

#### 前端接口对接

| 接口 | 方法 | 调用时机 | 说明 |
| --- | --- | --- | --- |
| download_model | Bridge | 点击下载 | 启动下载任务 |
| get_available_models | Bridge | 面板加载 | 获取模型列表 |
| set_user_config | Bridge | 开始游戏 | 保存配置 |
| get_gpu_backend | Bridge | 加载完成 | 显示 GPU 后端 |

## 5. 技术决策

### 决策1：GPU 后端选择策略

- **选型方案**：按平台优先级选择（Metal > CUDA > Vulkan > CPU）
- **选择理由**：
  1. macOS/iOS Metal 是唯一高效选择
  2. Windows NVIDIA GPU 用户多，CUDA 性能最优
  3. Vulkan 是跨平台兜底，支持 AMD/Intel/Android
  4. CPU 兜底保证任何设备都能运行
- **备选方案**：仅使用 Vulkan（放弃 CUDA）
- **放弃原因**：NVIDIA GPU 用户多，CUDA 性能更好

### 决策2：模型加载进度实现

- **选型方案**：估算进度 + Timer 模拟
- **选择理由**：
  1. llama-cpp-2 同步加载，无法获取中间进度
  2. 估算基于模型大小和阶段比例
  3. Timer 定时发送信号模拟进度
- **备选方案**：仅显示"加载中..."无进度
- **放弃原因**：用户体验差，无法感知加载进度

### 决策3：预编译 DLL 打包

- **选型方案**：随应用打包，按需下载模型
- **选择理由**：
  1. 用户无需编译环境
  2. 模型文件大（1-2.5GB），按需下载减小初始体积
- **备选方案**：模型预打包进应用
- **放弃原因**：
  1. 应用体积过大
  2. App Store / Play Store 审核限制

### 决策4：Provider 创建时机

- **选型方案**：start_simulation() 时根据 UserConfig 创建
- **选择理由**：
  1. 与现有启动流程一致
  2. 配置已保存，用户选择明确
- **备选方案**：settings_panel 保存配置后立即加载
- **放弃原因**：加载时间长，阻塞 UI

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| llama-cpp-2 编译复杂（C++ 依赖） | 高 | 提供 Windows/macOS/Android 预编译 DLL |
| 移动端内存限制（2B 模型 OOM） | 高 | 检测可用内存，提供 Gemma-2-2B 更小模型 |
| GPU DLL 缺失 | 中 | 自动降级到 CPU，记录日志提示 |
| 下载中断（网络不稳定） | 中 | CDN 自动切换，提供重试选项 |
| iOS App Store 审核 | 中 | 模型作为用户下载内容，不打包进 App |

## 7. 迁移方案

### 7.1 部署步骤

1. 配置 Cargo.toml features（cuda/vulkan/metal）
2. 实现 `crates/ai/src/llama.rs` LlamaProvider
3. 扩展 `crates/bridge/src/bridge.rs` 信号和 API
4. 修改 `crates/bridge/src/simulation_runner.rs` Provider 创建
5. 扩展 `client/scripts/settings_panel.gd` 进度 UI
6. 编写测试覆盖核心功能
7. 编译预编译 DLL（各平台）
8. 集成测试验证端到端流程

### 7.2 灰度策略

- Phase 1：Windows + Vulkan 后端验证
- Phase 2：Windows + CUDA 后端验证
- Phase 3：macOS Metal 后端验证
- Phase 4：Android Vulkan 后端验证
- Phase 5：完整发布

### 7.3 回滚方案

- 若 llama-cpp-2 编译失败，降级到仅支持 remote + rule_only 模式
- 若 GPU 后端全部不可用，强制使用 CPU
- 进度 UI 可临时隐藏，恢复日志输出方式

## 8. 待定事项

- [ ] llama-cpp-2 具体版本号（当前 0.1.11 是否稳定）
- [ ] CUDA DLL 是否需要随 CUDA Runtime 一起打包
- [ ] Android Vulkan 后端是否需要额外权限
- [ ] 模型加载进度估算的具体时间参数（需实测调整）
- [ ] 是否支持多模型切换（运行时切换模型）