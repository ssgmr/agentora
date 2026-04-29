# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

**项目架构**：Agentora 是跨平台（移动端为主）的去中心化文明模拟器，采用 Rust 核心 + Godot 4 GDExtension 桥接架构。

- **核心引擎**（`crates/core`）：World、Agent、DecisionPipeline、Memory、Strategy
- **AI 层**（`crates/ai`）：LlmProvider trait、OpenAI/Anthropic Provider、FallbackChain
- **网络层**（`crates/network`）：libp2p GossipSub、P2P 同步
- **桥接层**（`crates/bridge`）：SimulationBridge GDExtension、mpsc 通道

**现有 LLM 配置**：
- 配置文件：`config/llm.toml`（硬编码，启动时直接加载）
- Provider 初始化：`SimulationBridge::create_llm_provider()` 从固定路径读取
- 无用户自定义入口

**现有 Agent 配置**：
- WorldSeed：`worldseeds/default.toml`（地形、资源、初始 Agent）
- PersonalitySeed：性格模板（explorer/socializer/survivor/builder）
- 无玩家 Agent 名字、提示词、图标自定义

**现有 P2P 配置**：
- WorldSeed.seed_peers：种子节点地址列表
- SimMode：Centralized/P2P 模式选择
- 无用户界面选择入口

### 1.2 现状分析

**问题**：
1. 用户无法在启动前选择 LLM 模式（本地/远程/规则）
2. 用户无法下载 GGUF 模型到设备
3. 用户无法自定义 Agent 名字和性格
4. 用户无法直观选择 P2P 世界参与模式
5. 配置无持久化，每次启动使用相同硬编码值

**影响**：
- 无法产品化发布（缺少首次使用引导）
- 用户体验差（无法个性化）
- 移动端用户无法使用本地模型

### 1.3 关键干系人

- **Rust 核心**：ai crate、core crate、bridge crate
- **Godot 客户端**：main.gd、新建 setup_wizard.gd
- **外部服务**：ModelScope CDN、HuggingFace CDN
- **移动端打包**：iOS App Store、Android APK

## 2. 设计目标

### 目标

- 实现 llama-cpp-rs 本地推理引擎集成，支持跨平台 GPU 加速
- 实现 GGUF 模型 HTTP 下载模块，支持进度显示和 CDN 切换
- 实现引导页面 UI，简洁单页设计，移动端触摸友好
- 实现 Agent 个性化配置：名字、系统提示词、预设/自定义图标
- 实现 P2P 世界模式选择：单机/创建/加入
- 实现用户配置持久化，首次启动检测，游戏内可修改

### 非目标

- 不实现模型转换功能（GGUF 需预先存在）
- 不实现多语言界面（当前仅中文）
- 不实现云同步配置
- 不实现二维码扫描（可选功能，后续迭代）

## 3. 整体架构

### 3.1 架构概览

```
┌─────────────────────────────────────────────────────────────────────┐
│                        系统启动流程                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Godot 启动                                                         │
│      │                                                              │
│      ├─► main.gd._ready()                                           │
│      │       │                                                      │
│      │       ├─► Bridge.has_user_config()                           │
│      │       │       ├─► false → change_scene(setup_wizard.tscn)   │
│      │       │       └─► true  → continue (加载 main.tscn)          │
│      │       │                                                      │
│      │       └─► [setup_wizard.tscn]                                │
│      │               │                                              │
│      │               ├─► LLM 配置区域                               │
│      │               │       ├─► 模式选择：本地/远程/规则           │
│      │               │       ├─► 本地：模型列表 + 下载               │
│      │               │       └─► 远程：endpoint/token/model         │
│      │               │                                              │
│      │               ├─► Agent 配置区域                             │
│      │               │       ├─► 名字输入                           │
│      │               │       ├─► 系统提示词                          │
│      │               │       └─► 图标选择                           │
│      │               │                                              │
│      │               ├─► P2P 配置区域                               │
│      │               │       ├─► 模式：单机/创建/加入                │
│      │               │       └─► 地址显示/输入                       │
│      │               │                                              │
│      │               └─► [开始游戏]                                  │
│      │                       │                                      │
│      │                       ├─► Bridge.set_user_config()           │
│      │                       └─► change_scene(main.tscn)            │
│      │                                                              │
│      └─► [main.tscn]                                                │
│              │                                                      │
│              ├─► Bridge.get_user_config()                           │
│              ├─► UserConfig → WorldSeed 合并                        │
│              └─► Simulation.start(配置)                             │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件名 | 职责说明 |
| --- | --- |
| `UserConfig` | Rust 结构体，用户配置（LLM/Agent/P2P），TOML 序列化 |
| `LlamaProvider` | llama-cpp-rs Provider，本地 GGUF 推理 |
| `ModelDownloader` | HTTP 下载模块，进度信号，CDN 切换 |
| `setup_wizard.gd` | Godot 引导脚本，UI 交互，配置收集 |
| `IconProcessor` | 图标处理模块，缩放到 32x32 |
| `ConfigManager` | 配置管理器，检测/加载/保存 user_config.toml |

### 3.3 数据流设计

```
┌─────────────────────────────────────────────────────────────────────┐
│                      模型下载数据流                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Godot UI                                                           │
│      │                                                              │
│      ├─► 用户点击下载                                                │
│      │                                                              │
│      ▼                                                              │
│  Bridge.download_model(url, dest)                                   │
│      │                                                              │
│      ├─► 启动 tokio async task                                      │
│      │       │                                                      │
│      │       ├─► HTTP GET (reqwest)                                 │
│      │       │       ├─► ModelScope CDN                            │
│      │       │       └─► HuggingFace CDN (fallback)                │
│      │       │                                                      │
│      │       ├─► 进度计算                                            │
│      │       │       downloaded_mb / total_mb                      │
│      │       │       speed_mbps                                     │
│      │       │                                                      │
│      │       └─► 定期 emit_signal                                   │
│      │               download_progress(downloaded, total, speed)   │
│      │                                                              │
│      ▼                                                              │
│  Godot UI                                                           │
│      │                                                              │
│      ├─► 进度条更新                                                  │
│      ├─► 速度显示                                                    │
│      │                                                              │
│      ▼                                                              │
│  download_complete / download_failed 信号                           │
│      │                                                              │
│      └─► UI 状态更新                                                │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

```
┌─────────────────────────────────────────────────────────────────────┐
│                      配置应用数据流                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  config/user_config.toml                                            │
│      │                                                              │
│      ▼                                                              │
│  UserConfig (Rust struct)                                           │
│      │                                                              │
│      ├─► [llm] → LlmProvider 初始化                                 │
│      │       mode = "local" → LlamaProvider                        │
│      │       mode = "remote" → OpenAiProvider                      │
│      │       mode = "rule_only" → 无 Provider                      │
│      │                                                              │
│      ├─► [agent] → PersonalitySeed 扩展                             │
│      │       name → Agent 命名                                      │
│      │       custom_prompt → Prompt 注入                            │
│      │       icon_id → AgentSnapshot.icon_id                        │
│      │                                                              │
│      └─► [p2p] → WorldSeed 合并                                     │
│              mode = "single" → SimMode::Centralized                 │
│              mode = "create" → SimMode::P2P                         │
│              mode = "join" → seed_peers 添加                        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## 4. 详细设计

### 4.1 接口设计

#### Bridge API 新增方法

| 方法名 | 参数 | 返回值 | 说明 |
| --- | --- | --- | --- |
| `set_user_config` | `config: Dictionary` | `bool` | 保存用户配置 |
| `get_user_config` | 无 | `Dictionary` | 获取当前配置 |
| `has_user_config` | 无 | `bool` | 检测配置文件是否存在 |
| `download_model` | `url: String, dest: String` | `bool` | 启动模型下载 |
| `get_available_models` | 无 | `Array` | 获取预置模型列表 |
| `cancel_download` | 无 | `bool` | 取消当前下载 |

#### Bridge 新增信号

| 信号名 | 参数 | 说明 |
| --- | --- | --- |
| `download_progress` | `downloaded_mb: float, total_mb: float, speed_mbps: float` | 下载进度 |
| `model_download_complete` | `path: String` | 下载成功 |
| `model_download_failed` | `error: String` | 下载失败 |

### 4.2 数据模型

#### UserConfig 结构（Rust）

```rust
/// 用户配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub llm: LlmUserConfig,
    pub agent: AgentUserConfig,
    pub p2p: P2PUserConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUserConfig {
    /// 模式：local / remote / rule_only
    pub mode: String,
    /// 远程 API endpoint
    pub api_endpoint: String,
    /// 远程 API token
    pub api_token: String,
    /// 远程 API model name
    pub model_name: String,
    /// 本地模型路径
    pub local_model_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUserConfig {
    /// Agent 名字
    pub name: String,
    /// 自定义系统提示词
    pub custom_prompt: String,
    /// 预设图标 ID
    pub icon_id: String,
    /// 自定义图标路径
    pub custom_icon_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PUserConfig {
    /// 模式：single / create / join
    pub mode: String,
    /// 种子节点地址（join 模式）
    pub seed_address: String,
}
```

#### UserConfig TOML 格式

```toml
[llm]
mode = "local"
api_endpoint = ""
api_token = ""
model_name = ""
local_model_path = "models/Qwen3.5-2B-Q4_K_M.gguf"

[agent]
name = "智行者"
custom_prompt = "你是一个谨慎的探索者，善于发现隐藏资源..."
icon_id = "fox"
custom_icon_path = ""

[p2p]
mode = "single"
seed_address = ""
```

#### PersonalitySeed 扩展

```rust
// 现有字段
pub struct PersonalitySeed {
    pub openness: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
    pub description: String,
    
    // 新增字段
    pub custom_prompt: Option<String>,  // 用户自定义系统提示词
    pub icon_id: Option<String>,         // 预设图标 ID
    pub custom_icon_path: Option<String>, // 自定义图标文件路径
}
```

#### 预置模型定义

```rust
pub struct ModelEntry {
    pub name: String,           // Qwen3.5-2B-Q4_K_M
    pub size_mb: u32,           // 1500
    pub description: String,    // 首token <100ms，推荐移动端
    pub primary_url: String,    // ModelScope CDN
    pub fallback_url: String,   // HuggingFace CDN
    pub filename: String,       // Qwen3.5-2B-Q4_K_M.gguf
}

pub const AVAILABLE_MODELS: &[ModelEntry] = &[
    ModelEntry {
        name: "Qwen3.5-2B-Q4_K_M",
        size_mb: 1500,
        description: "首token <100ms，推荐移动端",
        primary_url: "https://modelscope.cn/models/.../resolve/master/Qwen3.5-2B-Q4_K_M.gguf",
        fallback_url: "https://huggingface.co/.../resolve/main/Qwen3.5-2B-Q4_K_M.gguf",
        filename: "Qwen3.5-2B-Q4_K_M.gguf",
    },
    // ... 其他模型
];
```

### 4.3 核心算法

#### llama-cpp-rs Provider 实现

```rust
// crates/ai/src/llama.rs

use llama_cpp_2::{
    llama_backend::LlamaBackend,
    model::{LlamaModel, AddBos, params::LlamaModelParams},
    context::params::LlamaContextParams,
    sampling::LlamaSampler,
};
use std::path::Path;
use std::num::NonZeroU32;

pub struct LlamaProvider {
    model: LlamaModel,
    backend: LlamaBackend,
    model_path: String,
}

impl LlamaProvider {
    pub fn new(model_path: String) -> Result<Self, LlmError> {
        // 初始化 backend
        let backend = LlamaBackend::init()?;
        
        // 配置模型参数（GPU 加速）
        let model_params = LlamaModelParams::default()
            .with_n_gpu_layers(1000);  // 全量 GPU
        
        // 加载 GGUF 模型
        let model = LlamaModel::load_from_file(
            &backend,
            Path::new(&model_path),
            &model_params,
        )?;
        
        Ok(Self { model, backend, model_path })
    }
}

impl LlmProvider for LlamaProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // 创建推理上下文
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(4096))
            .with_n_threads(num_cpus::get());
        
        let ctx = self.model.new_context(&self.backend, ctx_params)?;
        
        // Tokenize prompt
        let tokens = self.model.str_to_token(&request.prompt, AddBos::Always)?;
        
        // 创建采样链
        let sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(request.temperature),
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.95, 1),
            LlamaSampler::dist(42),
        ]);
        
        // 推理生成
        let mut output_tokens = Vec::new();
        let mut batch = LlamaBatch::new(512, 1);
        
        for (i, token) in tokens.iter().enumerate() {
            batch.add(*token, i as i32, &[0], i == tokens.len() - 1)?;
        }
        
        ctx.decode(&mut batch)?;
        
        for _ in 0..request.max_tokens {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            if self.model.is_eog_token(token) { break; }
            
            sampler.accept(token);
            output_tokens.push(token);
            
            batch.clear();
            batch.add(token, output_tokens.len() as i32, &[0], true)?;
            ctx.decode(&mut batch)?;
        }
        
        // Detokenize
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let output = output_tokens.iter()
            .map(|t| self.model.token_to_piece(*t, &mut decoder, true, None))
            .collect::<Result<Vec<_>, _>>()?
            .join("");
        
        Ok(LlmResponse {
            raw_text: output,
            parsed_action: None,
            usage: TokenUsage::default(),
            provider_name: "llama_local".to_string(),
        })
    }
    
    fn name(&self) -> &str { "llama_local" }
    fn is_available(&self) -> bool { true }
}
```

#### 模型下载实现

```rust
// crates/ai/src/model_downloader.rs

use reqwest::Client;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

pub struct ModelDownloader {
    client: Client,
    progress_tx: Option<mpsc::Sender<DownloadProgress>>,
}

pub struct DownloadProgress {
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub speed_mbps: f64,
}

impl ModelDownloader {
    pub async fn download(
        &self,
        url: &str,
        dest: PathBuf,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<PathBuf, DownloadError> {
        // 发送请求
        let response = self.client.get(url).send().await?;
        
        let total_size = response.content_length().unwrap_or(0) as f64 / 1_048_576.0;
        let mut downloaded: f64 = 0.0;
        let mut last_time = std::time::Instant::now();
        let mut last_downloaded: f64 = 0.0;
        
        // 创建文件
        let mut file = tokio::fs::File::create(&dest).await?;
        
        // 流式下载
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            
            downloaded += chunk.len() as f64 / 1_048_576.0;
            
            // 计算速度
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(last_time).as_secs_f64();
            if elapsed > 0.5 {
                let speed = (downloaded - last_downloaded) / elapsed;
                last_time = now;
                last_downloaded = downloaded;
                
                // 发送进度
                progress_tx.send(DownloadProgress {
                    downloaded_mb: downloaded,
                    total_mb: total_size,
                    speed_mbps: speed,
                }).await?;
            }
        }
        
        file.flush().await?;
        Ok(dest)
    }
}
```

#### 图标缩放处理

```rust
// crates/bridge/src/icon_processor.rs

use image::{ImageFormat, imageops::FilterType};

pub fn process_custom_icon(source_path: &Path, dest_path: &Path) -> Result<(), IconError> {
    // 加载图片
    let img = image::load(source_path, ImageFormat::from_path(source_path)?)?;
    
    // 缩放到 32x32（使用 Lanczos3 过滤器）
    let resized = image::imageops::resize(
        &img,
        32, 32,
        FilterType::Lanczos3,
    );
    
    // 保存为 PNG
    resized.save_with_format(dest_path, ImageFormat::Png)?;
    
    Ok(())
}
```

#### Prompt 注入 custom_prompt

```rust
// crates/core/src/prompt.rs 修改

fn build_personality_section(&self, agent_name: &str, personality: &PersonalitySeed) -> String {
    let mut section = String::new();
    
    // 用户自定义提示词（优先）
    if let Some(custom) = &personality.custom_prompt {
        if !custom.is_empty() {
            section.push_str(custom);
            section.push_str("\n\n");
        }
    }
    
    // 默认性格描述
    if !personality.description.is_empty() {
        section.push_str(&format!("你是 {}，{}。\n", agent_name, personality.description));
    } else {
        section.push_str(&format!("你是 {}，一个自主决策的 AI Agent。\n", agent_name));
    }
    
    section
}
```

### 4.4 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| 配置文件不存在 | 返回 has_user_config() = false，触发引导页面 |
| 配置文件解析失败 | 显示错误提示，提供删除配置重新配置选项 |
| 模型文件不存在 | 显示"模型未下载"，提供下载按钮 |
| 模型下载网络失败 | 自动切换 CDN（ModelScope → HuggingFace），失败显示错误提示 |
| 模型加载 OOM | 拒绝加载，提示用户选择更小模型或使用远程 API |
| 本地推理超时 | 自动降级到远程 API Provider |
| Agent 名字为空 | 拒绝提交，聚焦到名字输入框显示提示 |
| 图标上传格式错误 | 显示"仅支持 PNG/JPG 格式" |
| P2P 连接失败 | 显示错误提示，提供重试选项 |

### 4.5 前端设计

#### 技术栈

- **框架**：Godot 4 GDScript
- **UI 节点**：Control、VBoxContainer、HBoxContainer、LineEdit、TextEdit、Button、TextureButton
- **资源**：预设图标 PNG（32x32）

#### 目录结构

```
client/
├── scenes/
│   ├── setup_wizard.tscn    # 新增引导页面场景
│   └── main.tscn            # 现有主场景（修改启动逻辑）
├── scripts/
│   ├── setup_wizard.gd      # 新增引导脚本
│   ├── main.gd              # 现有主脚本（修改启动检测）
│   └── download_progress.gd # 新增下载进度组件
├── assets/
│   └── textures/
│       └── agents/          # 新增预设图标目录
│           ├── default.png
│           ├── wizard.png
│           ├── fox.png
│           ├── dragon.png
│           ├── lion.png
│           └── robot.png
└── user_icons/              # 新增自定义图标目录
```

#### 页面设计

| 页面 | 路径 | 说明 |
|------|------|------|
| 引导页面 | scenes/setup_wizard.tscn | 首次配置页面 |

**页面布局**：
- 顶部标题：Agentora 智纪 - 配置向导
- LLM 配置区（折叠面板）
- Agent 配置区（折叠面板）
- P2P 配置区（折叠面板）
- 底部"开始游戏"按钮

#### 组件设计

| 组件名 | 类型 | 文件路径 | 说明 |
|--------|------|----------|------|
| LLMConfigPanel | Control | setup_wizard.gd 内嵌 | LLM 模式选择和配置 |
| AgentConfigPanel | Control | setup_wizard.gd 内嵌 | Agent 名字、提示词、图标 |
| P2PConfigPanel | Control | setup_wizard.gd 内嵌 | P2P 模式选择 |
| ModelList | VBoxContainer | setup_wizard.gd 内嵌 | 预置模型列表 |
| IconSelector | GridContainer | setup_wizard.gd 内嵌 | 图标选择网格 |
| DownloadProgress | ProgressBar | download_progress.gd | 下载进度条 |

#### 交互逻辑

1. **启动检测**
   - main.gd._ready() → Bridge.has_user_config()
   - false → change_scene(setup_wizard.tscn)
   - true → 继续正常启动

2. **LLM 模式切换**
   - 点击模式选项 → 显示对应配置界面
   - 本地模式 → 显示模型列表
   - 远程模式 → 显示输入框

3. **模型下载**
   - 点击下载 → Bridge.download_model(url, dest)
   - 监听 download_progress 信号 → 更新进度条
   - 收到 download_complete → 标记"已下载"

4. **图标选择**
   - 点击预设图标 → 高亮选中
   - 点击上传 → 文件对话框 → 缩放预览

5. **开始游戏**
   - 点击按钮 → 验证必填项
   - 调用 Bridge.set_user_config()
   - change_scene(main.tscn)

#### 前端接口对接

| 接口 | 方法 | 调用时机 | 说明 |
|------|------|----------|------|
| has_user_config | Bridge | 启动检测 | 判断是否显示引导 |
| get_available_models | Bridge | 引导页面加载 | 获取模型列表 |
| download_model | Bridge | 点击下载 | 启动下载任务 |
| set_user_config | Bridge | 开始游戏 | 保存配置 |
| get_user_config | Bridge | 设置面板加载 | 获取当前配置 |

## 5. 技术决策

### 决策1：推理引擎选型

- **选型方案**：llama.cpp（通过 llama-cpp-rs bindings）
- **选择理由**：
  1. GGUF 直接加载，无需预处理（用户可动态下载模型）
  2. 跨平台一致，同一 GGUF 文件适用于所有平台
  3. Metal/Vulkan 移动端 GPU 加速
  4. 有现成的 iOS (llama.swiftui) 和 Android 示例
  5. Rust bindings 可直接在 ai crate 中集成
- **备选方案**：MLC LLM
- **放弃原因**：需要权重转换 + 模型编译预处理，用户无法动态下载使用

### 决策2：配置文件格式

- **选型方案**：TOML
- **选择理由**：
  1. 项目现有配置文件（llm.toml、sim.toml）均为 TOML
  2. Rust toml crate 支持完善
  3. 可读性好，便于用户手动修改
- **备选方案**：JSON
- **放弃原因**：与现有配置风格不一致

### 决策3：引导页面触发时机

- **选型方案**：首次启动检测（无配置文件时触发）
- **选择理由**：
  1. 简单明确，无需额外状态管理
  2. 符合常规应用引导模式
- **备选方案**：每次启动都显示
- **放弃原因**：用户重复操作繁琐

### 决策4：模型下载 CDN 优先级

- **选型方案**：ModelScope 优先，HuggingFace 备用
- **选择理由**：
  1. 项目面向国内用户，ModelScope CDN 更快
  2. HuggingFace 作为国际备用
- **备选方案**：仅使用 HuggingFace
- **放弃原因**：国内下载速度慢

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| llama-cpp-rs 编译复杂（C++ 依赖） | 高 | 提供预编译 guide，使用 feature 门控 |
| 移动端内存限制（2B 模型可能 OOM） | 高 | 检测可用内存，提供更小模型（Gemma-4-2B） |
| GGUF 模型下载中断（网络不稳定） | 中 | 支持取消/暂停，提供重试选项 |
| iOS App Store 审核限制（模型打包） | 中 | 模型作为用户下载内容，不打包进 App |
| 引导页面 UI 复杂度 | 低 | 使用 Godot 原生控件，参考现有 UI 风格 |

## 7. 迁移方案

### 7.1 部署步骤

1. 添加 Cargo 依赖：llama-cpp-2、image
2. 实现 crates/ai/src/llama.rs、model_downloader.rs
3. 实现 crates/bridge/src/user_config.rs、icon_processor.rs
4. 扩展 crates/core/src/types.rs、prompt.rs、seed.rs
5. 扩展 crates/bridge/src/bridge.rs（新增 API 和信号）
6. 创建 client/scenes/setup_wizard.tscn、setup_wizard.gd
7. 创建 client/assets/textures/agents/ 预设图标
8. 修改 client/scripts/main.gd 启动检测逻辑
9. 编写测试覆盖核心功能

### 7.2 灰度策略

- Phase 1：桌面端验证（Windows/macOS/Linux）
- Phase 2：移动端适配（iOS/Android）
- Phase 3：完整发布

### 7.3 回滚方案

- 若 llama-cpp-rs 编译失败，可降级到仅支持远程 API 模式
- 引导页面可临时禁用，恢复原有硬编码配置启动

## 8. 待定事项

- [ ] 二维码扫描功能（可选，后续迭代）
- [ ] 模型下载暂停/断点续传（可选，后续迭代）
- [ ] 多语言界面支持（可选）
- [ ] 自定义图标颜色调整（可选）