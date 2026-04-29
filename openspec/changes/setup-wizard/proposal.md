# 需求说明书

## 背景概述

当前 Agentora 启动时直接加载硬编码配置（`config/llm.toml`、`worldseeds/default.toml`），用户无法在运行前自定义任何参数。作为面向移动端为主的跨平台应用，产品化的关键障碍在于：

1. **LLM 配置缺失**：用户无法选择本地模型或远程 API，无法在设备上动态下载 GGUF 模型
2. **Agent 个性化缺失**：玩家 Agent 的名字、性格提示词、图标均无法自定义
3. **P2P 入口复杂**：用户无法直观选择单机/创建世界/加入世界模式

本次变更旨在构建一个简洁的单页引导页面，解决上述三个核心问题，实现首次启动配置化、游戏内可修改、移动端友好的用户体验。

## 变更目标

- 目标1：提供 LLM 配置引导，支持本地 GGUF 模型动态下载 + 远程 API 配置 + 规则引擎模式
- 目标2：提供 Agent 个性化配置，支持自定义名字、系统提示词、预设/自定义图标
- 目标3：提供 P2P 世界模式选择，支持单机模式、创建新世界（成为种子节点）、加入已有世界
- 目标4：配置持久化到 `config/user_config.toml`，首次启动触发引导，游戏内可修改
- 目标5：集成 llama.cpp 推理引擎，通过 llama-cpp-rs 实现跨平台本地 GGUF 推理（iOS Metal / Android Vulkan / 桌面端 Metal/Vulkan/CUDA）

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `llm-local-inference` | 本地 GGUF 模型推理引擎集成（llama-cpp-rs bindings），支持 iOS Metal / Android Vulkan / 桌面端 GPU 加速 |
| `model-downloader` | GGUF 模型 HTTP 下载模块，支持 ModelScope/HuggingFace CDN，显示下载进度，可取消/暂停 |
| `setup-wizard-ui` | Godot 引导页面场景，简洁单页设计，移动端触摸友好，包含 LLM/Agent/P2P 三区配置 |
| `agent-customization` | Agent 个性化配置：名字、系统提示词（注入 Prompt）、预设图标选择、自定义图标上传（自动缩放到 32x32） |
| `p2p-mode-selector` | P2P 世界模式选择 UI：单机/创建世界（显示地址供分享）/加入世界（输入地址或二维码扫描） |
| `user-config-persistence` | 用户配置持久化模块，首次启动检测 `config/user_config.toml`，生成/加载/保存配置 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `llm-integration` | 扩展 LlmProvider 支持 llama-cpp-rs 本地推理；新增模型下载进度信号 |
| `agent-personality-config` | PersonalitySeed 新增 `custom_prompt`、`icon_id` 字段，支持用户自定义系统提示词和图标 |
| `bridge-api` | SimulationBridge 新增配置 API：`set_user_config()`、`download_model()`（带进度）、`get_available_models()` |
| `godot-client` | 启动流程改为检测配置文件 → 无配置则加载引导页面 → 配置后切换主场景 |
| `p2p-network` | 扩展模式选择逻辑：单机模式跳过 P2P 初始化，创建世界显示本地地址，加入世界输入种子地址 |
| `world-seed` | WorldSeed 新增 `player_agent_config` 字段，用于玩家 Agent 的个性化配置注入 |

## 影响范围

- **代码模块**：
  - `crates/ai/src/`：新增 `llama.rs`（llama-cpp-rs Provider）、`model_downloader.rs`、`local_process.rs`
  - `crates/core/src/`：扩展 `seed.rs`（player_agent_config）、`prompt.rs`（custom_prompt 注入）
  - `crates/core/src/types.rs`：扩展 `PersonalitySeed`（custom_prompt, icon_id）
  - `crates/core/src/snapshot.rs`：扩展 `AgentSnapshot`（icon_id）
  - `crates/bridge/src/`：扩展 `bridge.rs`（配置 API、下载进度信号）、新增 `user_config.rs`
  - `client/scenes/`：新增 `setup_wizard.tscn`
  - `client/scripts/`：新增 `setup_wizard.gd`、`download_progress.gd`
  - `client/assets/textures/agents/`：新增预设图标 PNG（default/wizard/fox/dragon/lion/robot）

- **API接口**：
  - SimulationBridge 新增 `[func]`：`set_user_config(config: Dictionary)`、`download_model(url: String, dest: String)`、`get_available_models() -> Array`
  - SimulationBridge 新增 `[signal]`：`download_progress(downloaded_mb: float, total_mb: float, speed_mbps: float)`
  - SimulationBridge 新增 `[signal]`：`model_download_complete(path: String)`、`model_download_failed(error: String)`

- **依赖组件**：
  - 新增 Cargo 依赖：`llama-cpp-2 = "0.3"`（Rust GGUF 推理 bindings）
  - 新增 Cargo 依赖：`image = "0.25"`（图标缩放处理）
  - 新增 Godot 资源：预设图标 PNG、引导页面 UI 资源

- **关联系统**：
  - ModelScope CDN（国内模型下载源）
  - HuggingFace CDN（备用下载源）
  - 移动端打包流程（iOS/Android 需包含 llama-cli 可执行文件）

## 验收标准

- [ ] 首次启动（无 `config/user_config.toml`）显示引导页面，有配置则跳过引导
- [ ] LLM 配置支持三种模式切换：本地模型 / 远程 API / 仅规则引擎
- [ ] 本地模型模式可从预置列表选择（Qwen3.5-2B、Gemma-4-2B 等），显示模型大小和描述
- [ ] 模型下载显示实时进度条（下载量/总量/速度），支持暂停/取消
- [ ] 下载完成后模型可直接加载推理，无需预处理
- [ ] 远程 API 模式支持输入 endpoint、token、model name
- [ ] Agent 配置支持自定义名字（非空校验）
- [ ] Agent 配置支持自定义系统提示词（可选，注入到 Prompt 影响决策）
- [ ] Agent 配置支持 6 个预设图标选择，可视化展示
- [ ] Agent 配置支持上传自定义图标（PNG/JPG），自动缩放到 32x32
- [ ] P2P 配置支持三种模式：单机 / 创建世界 / 加入世界
- [ ] 创建世界模式显示本地 P2P 地址，支持复制/分享
- [ ] 加入世界模式支持输入种子地址，移动端支持二维码扫描（可选）
- [ ] 点击"开始游戏"后配置保存到 `config/user_config.toml`，场景切换到 main.tscn
- [ ] 游戏内设置面板可修改配置（重启生效）
- [ ] 移动端（Android/iOS）引导页面 UI 触摸友好，按钮区域足够大
- [ ] llama.cpp 本地推理在骁龙 8Gen3 上首 token < 100ms，决策延迟 < 150ms