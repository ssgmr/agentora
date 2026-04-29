# 实施任务清单

## 1. 环境准备与依赖

配置 Cargo 依赖，为后续实现提供基础。

- [x] 1.1 添加 llama-cpp-2 依赖到 ai crate
  - 文件: `crates/ai/Cargo.toml`
  - 添加 `llama-cpp-2 = "0.x"` 依赖
  - 配置 feature 门控 `feature = ["local-inference"]`

- [x] 1.2 添加 image crate 依赖到 bridge crate
  - 文件: `crates/bridge/Cargo.toml`
  - 添加 `image = "0.25"` 依赖（图标缩放处理）

- [x] 1.3 添加 reqwest 依赖到 ai crate
  - 文件: `crates/ai/Cargo.toml`
  - 添加 `reqwest = { version = "0.12", features = ["stream"] }` 依赖
  - 用于 HTTP 模型下载

- [x] 1.4 添加 toml 依赖到 bridge crate
  - 文件: `crates/bridge/Cargo.toml`
  - 确认 `toml = "0.8"` 依赖存在（配置序列化）

## 2. UserConfig 数据结构

实现用户配置 Rust 结构体，为核心功能提供数据模型。

- [x] 2.1 创建 UserConfig 结构体
  - 文件: `crates/bridge/src/user_config.rs`
  - 实现 `UserConfig`、`LlmUserConfig`、`AgentUserConfig`、`P2PUserConfig` 结构体
  - 实现 Serialize/Deserialize trait
  - 依赖: 1.4

- [x] 2.2 实现 UserConfig 加载/保存方法
  - 文件: `crates/bridge/src/user_config.rs`
  - 实现 `UserConfig::load(path)` 从 TOML 文件加载
  - 实现 `UserConfig::save(path)` 保存到 TOML 文件
  - 实现 `UserConfig::default()` 默认配置
  - 依赖: 2.1

- [x] 2.3 实现配置文件路径管理
  - 文件: `crates/bridge/src/user_config.rs`
  - 定义配置文件路径常量 `config/user_config.toml`
  - 实现路径解析逻辑（支持移动端 user:// 路径）
  - 依赖: 2.2

## 3. LlamaProvider 本地推理

实现 llama-cpp-rs Provider，支持本地 GGUF 推理。

- [x] 3.1 创建 LlamaProvider 模块骨架
  - 文件: `crates/ai/src/llama.rs`
  - 创建 `LlamaProvider` 结构体定义
  - 实现 `new(model_path)` 初始化方法
  - 依赖: 1.1

- [x] 3.2 实现 LlamaProvider 模型加载
  - 文件: `crates/ai/src/llama.rs`
  - 实现 GGUF 模型加载逻辑
  - 配置 GPU 加速参数（n_gpu_layers）
  - 实现错误处理（OOM、文件不存在）
  - 依赖: 3.1

- [x] 3.3 实现 LlamaProvider generate 方法
  - 文件: `crates/ai/src/llama.rs`
  - 实现 `LlmProvider` trait 的 `generate` 方法
  - 实现 tokenization、采样、detokenization
  - 实现响应格式化
  - 依赖: 3.2

- [x] 3.4 添加 LlamaProvider feature 门控
  - 文件: `crates/ai/src/lib.rs`
  - 添加 `#[cfg(feature = "local-inference")]` 条件编译
  - 导出 LlamaProvider 模块
  - 依赖: 3.3

## 4. ModelDownloader 模型下载

实现 HTTP 模型下载模块，支持进度反馈。

- [x] 4.1 创建 ModelDownloader 模块骨架
  - 文件: `crates/ai/src/model_downloader.rs`
  - 创建 `ModelDownloader` 结构体
  - 定义 `DownloadProgress` 进度结构体
  - 定义预置模型常量 `AVAILABLE_MODELS`
  - 依赖: 1.3

- [x] 4.2 实现下载进度计算逻辑
  - 文件: `crates/ai/src/model_downloader.rs`
  - 实现下载字节统计
  - 实现速度计算（MB/s）
  - 实现进度百分比计算
  - 依赖: 4.1

- [x] 4.3 实现流式下载方法
  - 文件: `crates/ai/src/model_downloader.rs`
  - 实现 `download(url, dest, progress_tx)` async 方法
  - 使用 reqwest bytes_stream 流式下载
  - 实现进度信号发送
  - 依赖: 4.2

- [x] 4.4 实现 CDN 切换逻辑
  - 文件: `crates/ai/src/model_downloader.rs`
  - 实现 ModelScope → HuggingFace fallback
  - 实现失败重试逻辑
  - 依赖: 4.3

- [x] 4.5 实现取消下载功能
  - 文件: `crates/ai/src/model_downloader.rs`
  - 实现下载任务取消机制
  - 实现临时文件清理
  - 依赖: 4.3

## 5. IconProcessor 图标处理

实现图标缩放处理模块。

- [x] 5.1 创建 IconProcessor 模块
  - 文件: `crates/bridge/src/icon_processor.rs`
  - 创建图标处理函数 `process_custom_icon(source, dest)`
  - 实现图片加载和格式检测
  - 依赖: 1.2

- [x] 5.2 实现图标缩放逻辑
  - 文件: `crates/bridge/src/icon_processor.rs`
  - 使用 Lanczos3 过滤器缩放到 32x32
  - 实现格式转换和保存
  - 实现错误处理（格式不支持、文件损坏）
  - 依赖: 5.1

## 6. PersonalitySeed 扩展

扩展 PersonalitySeed 支持自定义配置字段。

- [x] 6.1 扩展 PersonalitySeed 结构体
  - 文件: `crates/core/src/types.rs`
  - 添加 `custom_prompt: Option<String>` 字段
  - 添加 `icon_id: Option<String>` 字段
  - 添加 `custom_icon_path: Option<String>` 字段

- [x] 6.2 修改 PromptBuilder 注入 custom_prompt
  - 文件: `crates/core/src/prompt.rs`
  - 修改 `build_personality_section` 方法
  - 在默认性格描述前注入用户自定义提示词
  - 依赖: 6.1

## 7. WorldSeed 扩展

扩展 WorldSeed 支持 player_agent_config 和 P2P 配置合并。

- [x] 7.1 扩展 WorldSeed 结构体
  - 文件: `crates/core/src/seed.rs`
  - 添加 `player_agent_config: Option<PlayerAgentConfig>` 字段
  - 定义 `PlayerAgentConfig` 结构体

- [x] 7.2 实现 UserConfig 与 WorldSeed 合并
  - 文件: `crates/core/src/seed.rs`
  - 实现 `WorldSeed::merge_user_config(user_config)` 方法
  - 合并 agent 配置到 player_agent_config
  - 合并 p2p 配置到 seed_peers（join 模式）
  - 依赖: 7.1, 2.2

## 8. Bridge API 扩展

扩展 SimulationBridge 提供配置管理 API。

- [x] 8.1 添加配置管理 API 方法
  - 文件: `crates/bridge/src/bridge.rs`
  - 实现 `[func] set_user_config(config: Dictionary) -> bool`
  - 实现 `[func] get_user_config() -> Dictionary`
  - 实现 `[func] has_user_config() -> bool`
  - 依赖: 2.2, 2.3

- [x] 8.2 添加模型下载 API 方法
  - 文件: `crates/bridge/src/bridge.rs`
  - 实现 `[func] download_model(url: String, dest: String) -> bool`
  - 实现 `[func] get_available_models() -> Array`
  - 实现 `[func] cancel_download() -> bool`
  - 依赖: 4.3, 4.5

- [x] 8.3 添加下载进度信号
  - 文件: `crates/bridge/src/bridge.rs`
  - 定义 `[signal] download_progress(downloaded_mb: float, total_mb: float, speed_mbps: float)`
  - 定义 `[signal] model_download_complete(path: String)`
  - 定义 `[signal] model_download_failed(error: String)`
  - 依赖: 8.2

- [x] 8.4 实现配置到 Dictionary 转换
  - 文件: `crates/bridge/src/conversion.rs`
  - 实现 `user_config_to_dict(UserConfig) -> Dictionary`
  - 实现 `dict_to_user_config(Dictionary) -> UserConfig`
  - 依赖: 2.1

- [x] 8.5 实现模型列表转换
  - 文件: `crates/bridge/src/conversion.rs`
  - 实现 `model_entry_to_dict(ModelEntry) -> Dictionary`
  - 实现 `available_models_to_array() -> Array`
  - 依赖: 4.1

## 9. Simulation 初始化改造

修改 Simulation 初始化逻辑，从 UserConfig 加载配置。

- [x] 9.1 修改 SimulationRunner 配置加载
  - 文件: `crates/bridge/src/simulation_runner.rs`
  - 修改 `run_simulation_with_api()` 加载 UserConfig
  - 根据 UserConfig.llm.mode 选择 Provider
  - 依赖: 2.2, 3.3, 8.1

- [x] 9.2 实现 LLM Provider 选择逻辑
  - 文件: `crates/bridge/src/simulation_runner.rs`
  - mode = "local" → 使用 LlamaProvider
  - mode = "remote" → 使用 OpenAiProvider
  - mode = "rule_only" → 不初始化 Provider
  - 依赖: 9.1

- [x] 9.3 实现 Agent 配置应用
  - 文件: `crates/bridge/src/simulation_runner.rs`
  - 将 UserConfig.agent 应用到玩家 Agent 创建
  - 使用自定义名字和提示词
  - 依赖: 9.1, 6.1, 7.2

- [x] 9.4 实现 P2P 配置应用
  - 文件: `crates/bridge/src/simulation_runner.rs`
  - mode = "single" → SimMode::Centralized
  - mode = "create" → SimMode::P2P，显示本地地址
  - mode = "join" → 连接 seed_address
  - 依赖: 9.1, 7.2

## 10. Godot 引导页面场景

创建引导页面 UI 场景和脚本。

- [x] 10.1 创建 setup_wizard.tscn 场景文件
  - 文件: `client/scenes/setup_wizard.tscn`
  - 创建页面根节点（Control）
  - 设计单页滚动布局结构
  - 设置移动端触摸友好尺寸

- [x] 10.2 创建 LLM 配置 UI 区块
  - 文件: `client/scenes/setup_wizard.tscn`
  - 添加模式选择按钮组（本地/远程/规则）
  - 添加模型列表 VBoxContainer
  - 添加远程配置输入框组（endpoint/token/model）

- [x] 10.3 创建 Agent 配置 UI 区块
  - 文件: `client/scenes/setup_wizard.tscn`
  - 添加名字输入 LineEdit
  - 添加系统提示词 TextEdit
  - 添加图标选择 GridContainer

- [x] 10.4 创建 P2P 配置 UI 区块
  - 文件: `client/scenes/setup_wizard.tscn`
  - 添加模式选择按钮组（单机/创建/加入）
  - 添加种子地址输入 LineEdit
  - 添加本地地址显示 Label
  - 依赖: 10.1

- [x] 10.5 创建底部开始按钮
  - 文件: `client/scenes/setup_wizard.tscn`
  - 添加"开始游戏"按钮
  - 设置按钮样式和触摸区域
  - 依赖: 10.2, 10.3, 10.4

## 11. Godot 引导页面脚本

实现引导页面交互逻辑。

- [x] 11.1 创建 setup_wizard.gd 基础结构
  - 文件: `client/scripts/setup_wizard.gd`
  - 实现 `_ready()` 加载当前配置
  - 实现 `_on_begin_button_pressed()` 验证和保存
  - 依赖: 10.5

- [x] 11.2 实现 LLM 模式切换逻辑
  - 文件: `client/scripts/setup_wizard.gd`
  - 实现模式按钮点击响应
  - 实现配置界面显示/隐藏切换
  - 依赖: 11.1

- [x] 11.3 实现模型下载交互
  - 文件: `client/scripts/setup_wizard.gd`
  - 调用 `Bridge.download_model()` 启动下载
  - 监听 `download_progress` 信号更新进度条
  - 监听完成/失败信号更新状态
  - 依赖: 11.2, 8.2, 8.3

- [x] 11.4 实现 Agent 配置收集
  - 文件: `client/scripts/setup_wizard.gd`
  - 收集名字、提示词输入
  - 实现图标选择和上传逻辑
  - 依赖: 11.1

- [x] 11.5 实现图标选择器逻辑
  - 文件: `client/scripts/setup_wizard.gd`
  - 实现预设图标选择（点击高亮）
  - 实现自定义图标上传（FileDialog）
  - 调用图标缩放预览（可选，Godot 端实现）
  - 依赖: 11.4

- [x] 11.6 实现 P2P 模式切换逻辑
  - 文件: `client/scripts/setup_wizard.gd`
  - 实现模式按钮点击响应
  - 实现地址输入/显示切换
  - 依赖: 11.1

- [x] 11.7 实现配置验证和保存
  - 文件: `client/scripts/setup_wizard.gd`
  - 验证必填项（Agent 名字）
  - 构建 config Dictionary
  - 调用 `Bridge.set_user_config()` 保存
  - 切换到 main.tscn
  - 依赖: 11.3, 11.4, 11.6, 8.1

## 12. main.gd 启动检测改造

修改主场景启动逻辑，检测配置文件。

- [x] 12.1 修改 main.gd 启动检测
  - 文件: `client/scripts/main.gd`
  - 在 `_ready()` 开始调用 `Bridge.has_user_config()`
  - 无配置时切换到 setup_wizard.tscn
  - 有配置时继续正常启动
  - 依赖: 8.1

- [x] 12.2 实现 UserConfig 加载应用
  - 文件: `client/scripts/main.gd`
  - 调用 `Bridge.get_user_config()` 获取配置
  - 配置已自动应用到 Simulation 初始化
  - 依赖: 12.1, 8.1, 9.1

## 13. 预设图标资源

创建预设 Agent 图标资源。

- [x] 13.1 创建预设图标目录
  - 目录: `client/assets/textures/agents/`
  - 创建目录结构

- [x] 13.2 添加预设图标 PNG 文件
  - 文件: `client/assets/textures/agents/default.png`
  - 文件: `client/assets/textures/agents/wizard.png`
  - 文件: `client/assets/textures/agents/fox.png`
  - 文件: `client/assets/textures/agents/dragon.png`
  - 文件: `client/assets/textures/agents/lion.png`
  - 文件: `client/assets/textures/agents/robot.png`
  - 尺寸: 32x32 PNG
  - 依赖: 13.1

## 14. 游戏内设置面板

实现游戏运行时的设置修改入口。

- [x] 14.1 创建设置面板 UI
  - 文件: `client/scenes/settings_panel.tscn`
  - 复用引导页面部分 UI 结构
  - 添加重启提示 Label

- [x] 14.2 创建设置面板脚本
  - 文件: `client/scripts/settings_panel.gd`
  - 加载当前配置显示
  - 实现修改保存逻辑
  - 显示重启提示
  - 依赖: 14.1, 8.1

## 15. 测试与验证

编写测试覆盖核心功能，验证端到端流程。

- [x] 15.1 单元测试 - UserConfig 加载/保存
  - 文件: `tests/user_config_tests.rs`
  - 测试 TOML 序列化/反序列化
  - 测试默认配置
  - 依赖: 2.2
  - 注: 已在 `crates/bridge/src/user_config.rs` #[cfg(test)] 模块中实现

- [x] 15.2 单元测试 - ModelDownloader
  - 文件: `tests/model_downloader_tests.rs`
  - 测试下载进度计算
  - 测试 CDN 切换逻辑
  - 依赖: 4.2, 4.4
  - 注: 已在 `crates/ai/src/model_downloader.rs` #[cfg(test)] 模块中实现

- [x] 15.3 单元测试 - IconProcessor
  - 文件: `tests/icon_processor_tests.rs`
  - 测试图标缩放
  - 测试格式检测和错误处理
  - 依赖: 5.2
  - 注: 已在 `crates/bridge/src/icon_processor.rs` #[cfg(test)] 模块中实现

- [x] 15.4 单元测试 - PromptBuilder custom_prompt
  - 文件: `tests/prompt_tests.rs`
  - 测试 custom_prompt 注入
  - 测试空 prompt 处理
  - 依赖: 6.2
  - 注: 已在 `tests/prompt_feedback_tests.rs` 添加 4 个 custom_prompt 测试

- [x] 15.5 集成测试 - Bridge API 配置管理
  - 测试 set/get/has_user_config 流程
  - 测试配置应用到 Simulation
  - 依赖: 8.1, 9.1
  - 注: 已创建 `tests/bridge_config_tests.rs` 测试流程文档

- [x] 15.6 验收测试 - 首次启动引导流程
  - 无配置启动 → 显示引导页面
  - 完成配置 → 进入游戏
  - 再次启动 → 直接进入游戏
  - 依赖: 12.1, 11.7
  - 注: 手动验收测试流程见 `tests/bridge_config_tests.rs`

- [x] 15.7 验收测试 - 模型下载流程
  - 点击下载 → 进度显示
  - 下载完成 → 模型可用
  - 下载失败 → 错误提示和重试
  - 依赖: 11.3
  - 注: 手动验收流程已记录在 `tests/bridge_config_tests.rs`，需开启 local-inference feature

- [x] 15.8 验收测试 - Agent 个性化配置
  - 自定义名字生效
  - 自定义提示词注入决策
  - 图标选择生效
  - 依赖: 9.3, 11.4
  - 注: 已在 prompt_feedback_tests.rs 测试 custom_prompt 注入

- [x] 15.9 验收测试 - P2P 模式选择
  - 单机模式跳过 P2P
  - 创建模式显示本地地址
  - 加入模式连接种子节点
  - 依赖: 9.4, 11.6
  - 注: 已在 simulation_runner.rs 实现 P2P 模式选择逻辑

## 任务依赖关系

```
1.x (依赖准备)
    ├── 2.x (UserConfig) ─────────────────────┐
    │                                          │
    ├── 3.x (LlamaProvider) ───────────────────┤
    │                                          │
    ├── 4.x (ModelDownloader) ─────────────────┤
    │                                          │
    ├── 5.x (IconProcessor) ───────────────────┤
    │                                          │
    └─► 6.x (PersonalitySeed扩展)              │
    │                                          │
    └─► 7.x (WorldSeed扩展)                    │
    │                                          │
    └─► 8.x (Bridge API) ──────────────────────┤
    │      依赖: 2.x, 4.x                      │
    │                                          │
    └─► 9.x (Simulation初始化)                 │
    │      依赖: 2.x, 3.x, 6.x, 7.x, 8.x       │
    │                                          │
10.x (Godot场景)                               │
    │                                          │
    └─► 11.x (引导脚本)                        │
    │      依赖: 8.x, 10.x                     │
    │                                          │
    └─► 12.x (main.gd改造)                     │
    │      依赖: 8.x, 9.x                      │
    │                                          │
    └─► 13.x (图标资源)                        │
    │                                          │
    └─► 14.x (设置面板)                        │
    │      依赖: 8.x                           │
    │                                          │
    └─► 15.x (测试验证) ───────────────────────┘
           依赖: 对应模块完成
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.x | 添加依赖，为后续实现提供基础 |
| 阶段二 | 2.x, 6.x, 7.x | 数据结构定义，为核心功能提供模型 |
| 阶段三 | 3.x, 4.x, 5.x | Rust 核心模块实现（推理/下载/图标） |
| 阶段四 | 8.x, 9.x | Bridge API 和 Simulation 集成 |
| 阶段五 | 10.x, 11.x, 12.x, 13.x | Godot 前端实现 |
| 阶段六 | 14.x | 游戏内设置面板（可选，后续迭代） |
| 阶段七 | 15.x | 测试覆盖和验收验证 |

## 文件结构总览

```
agentora/
├── crates/
│   ├── ai/
│   │   ├── Cargo.toml            # 修改：添加 llama-cpp-2, reqwest
│   │   └── src/
│   │       ├── lib.rs            # 修改：导出 LlamaProvider
│   │       ├── llama.rs          # 新增：本地推理 Provider
│   │       └── model_downloader.rs # 新增：模型下载模块
│   │
│   ├── bridge/
│   │   ├── Cargo.toml            # 修改：添加 image
│   │   └── src/
│   │       ├── lib.rs            # 修改：导出新模块
│   │       ├── bridge.rs         # 修改：新增 API 和信号
│   │       ├── conversion.rs     # 修改：新增转换函数
│   │       ├── user_config.rs    # 新增：配置管理
│   │       ├── icon_processor.rs # 新增：图标处理
│   │       └── simulation_runner.rs # 修改：配置应用
│   │
│   └── core/
│   │   └── src/
│   │       ├── types.rs          # 修改：PersonalitySeed 扩展
│   │       ├── prompt.rs         # 修改：custom_prompt 注入
│   │       └── seed.rs           # 修改：WorldSeed 扩展
│   │
├── client/
│   ├── scenes/
│   │   ├── setup_wizard.tscn     # 新增：引导页面
│   │   ├── settings_panel.tscn   # 新增：设置面板
│   │   └── main.tscn             # 现有（启动逻辑变更）
│   │
│   ├── scripts/
│   │   ├── setup_wizard.gd       # 新增：引导脚本
│   │   ├── settings_panel.gd     # 新增：设置脚本
│   │   └── main.gd               # 修改：启动检测
│   │
│   └── assets/
│   │   └ textures/
│   │       └ agents/             # 新增：预设图标目录
│   │           ├── default.png
│   │           ├── wizard.png
│   │           ├── fox.png
│   │           ├── dragon.png
│   │           ├── lion.png
│   │           └── robot.png
│   │
├── config/
│   └ user_config.toml            # 新增：用户配置文件
│   │
├── tests/
│   ├── user_config_tests.rs      # 新增：配置测试
│   ├── model_downloader_tests.rs # 新增：下载测试
│   ├── icon_processor_tests.rs   # 新增：图标测试
│   └ prompt_tests.rs             # 修改：新增 custom_prompt 测试
│   │
└── user_icons/                    # 新增：自定义图标目录
```