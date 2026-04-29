# 详细设计文档

## 1. 背景与现状

### 1.1 技术背景

**项目架构**：Agentora 是跨平台去中心化文明模拟器，采用 Rust 核心 + Godot 4 GDExtension 桥接架构。

- **核心引擎**（`crates/core`）：World、Agent、DecisionPipeline、Memory、Strategy
- **桥接层**（`crates/bridge`）：SimulationBridge GDExtension、UserConfig 配置管理
- **Godot 客户端**（`client/`）：main.tscn 主场景、setup_wizard.tscn 引导页面、settings_panel.tscn 设置面板

**现有 UI 框架**：
- Godot 4 GDScript
- 动态 UI 构建（setup_wizard.gd 使用 `_build_ui()` 函数）
- 样式代码分散在各个脚本中

**已有资源**：
- 预设图标：6 个 PNG（default/wizard/fox/dragon/lion/robot）
- UserConfig Rust 结构体（TOML 序列化）
- Bridge API（has_user_config、get_user_config、set_user_config）

### 1.2 现状分析

**问题**：

1. **settings_panel 完全没有集成**：
   - 文件存在但无入口按钮
   - main.tscn 不包含 settings_panel 节点
   - main.gd 无相关代码连接

2. **settings_panel 功能不完整**：
   - 仅支持 LLM 模式和 Agent 名字
   - 缺少 custom_prompt、icon 选择、p2p 配置

3. **UI 架构混乱**：
   - settings_panel.gd 动态构建 UI 覆盖 .tscn 静态节点
   - setup_wizard.tscn 是空壳（只有根节点 + SimulationBridge）
   - 样式代码重复（setup_wizard.gd 和 settings_panel.gd 各自定义）

4. **缺少快捷键**：
   - 无 ESC 打开设置功能

### 1.3 关键干系人

- **Godot 客户端**：main.gd、settings_panel.gd、setup_wizard.gd
- **Rust Bridge**：已完成的 UserConfig API（无需修改）

## 2. 设计目标

### 目标

- 在游戏中提供完整的设置面板入口（按钮 + ESC 快捷键）
- 完善 settings_panel 功能，支持所有配置项修改
- 修复 UI 架构问题，统一使用静态 UI 定义
- 创建共享样式库，减少代码重复

### 非目标

- 不修改 Rust Bridge API（已完成）
- 不实现 setup_wizard.tscn 静态化（降低优先级，后续迭代）
- 不实现模型下载进度条（属于 setup-wizard 变更，本地推理功能）

## 3. 整体架构

### 3.1 架构概览

```
┌─────────────────────────────────────────────────────────────────────┐
│                        main.tscn UI 结构                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  CanvasLayer (UI)                                                   │
│      ├── TopBar (HBoxContainer)                                    │
│      │       ├── TickCounter                                       │
│      │       ├── AgentCount                                        │
│      │       ├── SpeedControl                                      │
│      │       ├── P2PBtnWrapper                                     │
│      │       └── SettingsBtn [新增] ──────────┐                    │
│      │                                        │                    │
│      ├── RightPanel                          │                    │
│      ├── NarrativeFeed                       │                    │
│      ├── P2PPopup                            │                    │
│      └── SettingsPanel [新增] ◄──────────────┘                    │
│              │                                                      │
│              ├── Bg (ColorRect)                                     │
│              ├── MarginContainer                                    │
│              │       └── VBoxContainer                              │
│              │               ├── TitleLabel                         │
│              │               ├── LLM 配置区                          │
│              │               ├── Agent 配置区                        │
│              │               ├── P2P 配置区                          │
│              │               ├── RestartLabel                       │
│              │               └── BtnHBox                            │
│              └── ...                                                │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件名 | 职责说明 |
| --- | --- |
| `shared_ui_styles.gd` | 共享样式函数库，提供按钮/输入框/面板样式函数 |
| `SettingsBtn` | TopBar 设置按钮，点击打开 settings_panel |
| `SettingsPanel` | 设置面板弹窗，支持所有配置项修改 |
| `main.gd` | 处理 ESC 快捷键、settings_panel 打开/关闭逻辑 |
| `settings_panel.gd` | 设置面板脚本，加载/保存配置，使用静态节点引用 |

### 3.3 数据流设计

```
┌─────────────────────────────────────────────────────────────────────┐
│                    settings_panel 数据流                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  用户操作                                                           │
│      │                                                              │
│      ├─► 点击 SettingsBtn 或 ESC                                    │
│      │       │                                                      │
│      │       └─► main.gd 显示 settings_panel                       │
│      │               │                                              │
│      │               └─► settings_panel.gd._load_config()           │
│      │                       │                                      │
│      │                       ├─► Bridge.get_user_config()           │
│      │                       │       └── 返回 config Dictionary     │
│      │                       │                                      │
│      │                       └─► 填充 UI 输入控件                   │
│      │                               ├── LLM 模式按钮选中           │
│      │                               ├── Agent 名字填充            │
│      │                               ├── custom_prompt 填充        │
│      │                               ├── icon 选中高亮              │
│      │                               └── P2P 模式按钮选中           │
│      │                                                              │
│      └─► 用户修改配置                                               │
│      │       │                                                      │
│      └─► 点击保存                                                    │
│      │       │                                                      │
│      │       └─► settings_panel.gd._on_save_pressed()              │
│      │               │                                              │
│      │               ├─► 验证 Agent 名字非空                        │
│      │               │                                              │
│      │               ├─► 收集 UI 输入值                             │
│      │               │       ├── llm_mode                           │
│      │               │       ├── agent_name                         │
│      │               │       ├── agent_custom_prompt                │
│      │               │       ├── agent_icon_id                      │
│      │               │       └── p2p_mode, p2p_seed_address         │
│      │               │                                              │
│      │               └─► Bridge.set_user_config(config)            │
│      │                       │                                      │
│      │                       └─► UserConfig.save()                  │
│      │                               └── 写入 user_config.toml     │
│      │                                                              │
│      └─► 显示重启提示（如果需要）                                   │
│      │       │                                                      │
│      └─► 点击关闭                                                    │
│      │       │                                                      │
│      └─► main.gd 隐藏 settings_panel                               │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## 4. 详细设计

### 4.1 前端架构

#### 技术栈

- **框架**：Godot 4 GDScript
- **UI 节点**：Control、PanelContainer、VBoxContainer、HBoxContainer、LineEdit、TextEdit、Button、TextureButton、GridContainer
- **样式**：StyleBoxFlat 动态创建，统一使用 shared_ui_styles.gd

#### 目录结构

```
client/
├── scenes/
│   ├── main.tscn            # 修改：添加 SettingsBtn + SettingsPanel
│   ├── settings_panel.tscn  # 修改：扩展 UI 结构
│   └── setup_wizard.tscn    # 不修改（本次不静态化）
│
├── scripts/
│   ├── main.gd              # 修改：添加 settings_panel 连接和 ESC 处理
│   ├── settings_panel.gd    # 修改：删除动态构建，使用静态节点，完善配置
│   ├── setup_wizard.gd      # 修改：使用 shared_ui_styles
│   └── shared_ui_styles.gd  # 新增：共享样式函数库
│
└── assets/
    └── textures/
        └── agents/          # 已有：预设图标 PNG
```

#### 路由设计

无新路由，settings_panel 作为弹窗显示。

### 4.2 页面设计

| 页面 | 路径 | 说明 |
|------|------|------|
| 主场景 | scenes/main.tscn | 包含 SettingsBtn 和 SettingsPanel 节点 |

**SettingsPanel 布局**：

```
┌─────────────────────────────────────────┐
│              设置                        │
├─────────────────────────────────────────┤
│                                         │
│  【LLM 模式】                           │
│  ┌──────┐ ┌──────┐ ┌──────┐           │
│  │本地  │ │远程  │ │规则  │           │
│  └──────┘ └──────┘ └──────┘           │
│                                         │
│  【Agent 名字】                         │
│  ┌─────────────────────────────┐       │
│  │ 智行者                       │       │
│  └─────────────────────────────┘       │
│                                         │
│  【系统提示词】                         │
│  ┌─────────────────────────────┐       │
│  │ 描述智能体性格...            │       │
│  │                              │       │
│  │                              │       │
│  └─────────────────────────────┘       │
│                                         │
│  【头像】                               │
│  ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐       │
│  │默│ │法│ │狐│ │龙│ │狮│ │机│       │
│  │认│ │师│ │狸│ │ │ │子│ │器│       │
│  └──┘ └──┘ └──┘ └──┘ └──┘ └──┘       │
│                                         │
│  【P2P 模式】                           │
│  ┌──────┐ ┌──────┐ ┌──────┐           │
│  │单机  │ │创建  │ │加入  │           │
│  └──────┘ └──────┘ └──────┘           │
│                                         │
│  [更改 LLM 模式需要重启生效]            │
│                                         │
│  ┌──────────┐  ┌──────────┐           │
│  │   保存   │  │   关闭   │           │
│  └──────────┘  └──────────┘           │
│                                         │
└─────────────────────────────────────────┘
```

### 4.3 组件设计

| 组件名 | 类型 | 文件路径 | 说明 |
|--------|------|----------|------|
| SettingsBtn | Button | scenes/main.tscn | TopBar 设置按钮 |
| SettingsPanel | PanelContainer | scenes/settings_panel.tscn | 设置面板根节点 |
| LLMModeButtons | HBoxContainer | scenes/settings_panel.tscn | LLM 模式切换按钮组 |
| AgentNameInput | LineEdit | scenes/settings_panel.tscn | Agent 名字输入框 |
| AgentPromptInput | TextEdit | scenes/settings_panel.tscn | 自定义提示词输入框 |
| IconGrid | GridContainer | scenes/settings_panel.tscn | 图标选择网格 |
| P2PModeButtons | HBoxContainer | scenes/settings_panel.tscn | P2P 模式切换按钮组 |
| RestartLabel | Label | scenes/settings_panel.tscn | 重启提示标签 |
| SaveBtn | Button | scenes/settings_panel.tscn | 保存按钮 |
| CloseBtn | Button | scenes/settings_panel.tscn | 关闭按钮 |

### 4.4 核心算法

#### shared_ui_styles.gd 样式函数

```gdscript
# shared_ui_styles.gd

# 预设主题色
const COLOR_BG_DARK := Color(0.12, 0.14, 0.16, 1.0)
const COLOR_BG_PANEL := Color(0.18, 0.20, 0.22, 1.0)
const COLOR_BG_INPUT := Color(0.22, 0.24, 0.26, 1.0)
const COLOR_BUTTON_NORMAL := Color(0.20, 0.22, 0.25, 1.0)
const COLOR_BUTTON_HOVER := Color(0.30, 0.32, 0.35, 1.0)
const COLOR_BUTTON_PRESSED := Color(0.25, 0.50, 0.65, 1.0)
const COLOR_BUTTON_SUCCESS := Color(0.20, 0.55, 0.30, 1.0)
const COLOR_TEXT_PRIMARY := Color(0.90, 0.90, 0.90, 1.0)
const COLOR_TEXT_PLACEHOLDER := Color(0.50, 0.50, 0.50, 1.0)

# 触摸友好尺寸
const MIN_BUTTON_HEIGHT := 36
const MIN_INPUT_HEIGHT := 36
const MIN_ICON_SIZE := 48

# 创建面板样式
static func create_panel_style(cornerRadius := 8, bgColor := COLOR_BG_PANEL) -> StyleBoxFlat:
    var style := StyleBoxFlat.new()
    style.bg_color = bgColor
    style.corner_radius_top_left = cornerRadius
    style.corner_radius_top_right = cornerRadius
    style.corner_radius_bottom_left = cornerRadius
    style.corner_radius_bottom_right = cornerRadius
    style.content_margin_left = 16
    style.content_margin_right = 16
    style.content_margin_top = 12
    style.content_margin_bottom = 12
    return style

# 创建按钮样式组
static func create_button_styles(styleType := "normal") -> Dictionary:
    var normal := StyleBoxFlat.new()
    var hover := StyleBoxFlat.new()
    var pressed := StyleBoxFlat.new()
    
    var baseRadius := 6
    
    if styleType == "success":
        normal.bg_color = COLOR_BUTTON_SUCCESS
        hover.bg_color = Color(0.30, 0.65, 0.40, 1.0)
        pressed.bg_color = Color(0.25, 0.50, 0.35, 1.0)
    else:
        normal.bg_color = COLOR_BUTTON_NORMAL
        hover.bg_color = COLOR_BUTTON_HOVER
        pressed.bg_color = COLOR_BUTTON_PRESSED
    
    for style in [normal, hover, pressed]:
        style.corner_radius_top_left = baseRadius
        style.corner_radius_top_right = baseRadius
        style.corner_radius_bottom_left = baseRadius
        style.corner_radius_bottom_right = baseRadius
    
    return {"normal": normal, "hover": hover, "pressed": pressed}

# 创建输入框样式
static func create_input_style() -> StyleBoxFlat:
    var style := StyleBoxFlat.new()
    style.bg_color = COLOR_BG_INPUT
    style.corner_radius_top_left = 4
    style.corner_radius_top_right = 4
    style.corner_radius_bottom_left = 4
    style.corner_radius_bottom_right = 4
    style.content_margin_left = 10
    style.content_margin_right = 10
    return style

# 应用样式到按钮
static func apply_button_style(btn: Button, styleType := "normal"):
    var styles := create_button_styles(styleType)
    btn.add_theme_stylebox_override("normal", styles["normal"])
    btn.add_theme_stylebox_override("hover", styles["hover"])
    btn.add_theme_stylebox_override("pressed", styles["pressed"])
    btn.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
    btn.add_theme_color_override("font_hover_color", Color(1.0, 1.0, 1.0, 1.0))

# 应用样式到输入框
static func apply_input_style(input: LineEdit):
    input.add_theme_stylebox_override("normal", create_input_style())
    input.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
    input.add_theme_color_override("font_placeholder_color", COLOR_TEXT_PLACEHOLDER)
    input.add_theme_color_override("caret_color", COLOR_TEXT_PRIMARY)

# 应用样式到文本编辑框
static func apply_textedit_style(input: TextEdit):
    input.add_theme_stylebox_override("normal", create_input_style())
    input.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
    input.add_theme_color_override("font_placeholder_color", COLOR_TEXT_PLACEHOLDER)
    input.add_theme_color_override("caret_color", COLOR_TEXT_PRIMARY)
```

#### main.gd ESC 处理

```gdscript
# main.gd 新增代码

var settings_panel: Control

func _ready() -> void:
    # ... 现有代码 ...
    
    # 获取 settings_panel 引用
    settings_panel = $UI/SettingsPanel
    if settings_panel:
        settings_panel.hide()
    
    # 连接 SettingsBtn
    var settings_btn = $UI/TopBar/SettingsBtn
    if settings_btn:
        settings_btn.pressed.connect(_on_settings_pressed)

func _input(event: InputEvent) -> void:
    if event is InputEventKey:
        if event.keycode == KEY_ESCAPE and event.pressed:
            _toggle_settings_panel()
            get_viewport().set_input_as_handled()

func _on_settings_pressed() -> void:
    _toggle_settings_panel()

func _toggle_settings_panel() -> void:
    if settings_panel:
        if settings_panel.visible:
            settings_panel.hide()
        else:
            settings_panel.show()
            # 触发配置加载
            if settings_panel.has_method("load_config"):
                settings_panel.load_config()
```

#### settings_panel.gd 静态节点引用

```gdscript
# settings_panel.gd 重构版本

extends PanelContainer

@onready var bridge: Node = get_node_or_null("../../SimulationBridge")

# UI 引用（使用 @onready 获取静态节点）
@onready var local_btn: Button = $MarginContainer/VBox/LLMSection/LLMHBox/LocalBtn
@onready var remote_btn: Button = $MarginContainer/VBox/LLMSection/LLMHBox/RemoteBtn
@onready var rule_btn: Button = $MarginContainer/VBox/LLMSection/LLMHBox/RuleBtn

@onready var agent_name_input: LineEdit = $MarginContainer/VBox/AgentSection/AgentNameInput
@onready var agent_prompt_input: TextEdit = $MarginContainer/VBox/AgentPromptSection/AgentPromptInput

@onready var icon_buttons: Array = [
    $MarginContainer/VBox/IconSection/IconGrid/IconDefault,
    $MarginContainer/VBox/IconSection/IconGrid/IconWizard,
    $MarginContainer/VBox/IconSection/IconGrid/IconFox,
    $MarginContainer/VBox/IconSection/IconGrid/IconDragon,
    $MarginContainer/VBox/IconSection/IconGrid/IconLion,
    $MarginContainer/VBox/IconSection/IconGrid/IconRobot,
]

@onready var p2p_single_btn: Button = $MarginContainer/VBox/P2PSection/P2PHBox/SingleBtn
@onready var p2p_create_btn: Button = $MarginContainer/VBox/P2PSection/P2PHBox/CreateBtn
@onready var p2p_join_btn: Button = $MarginContainer/VBox/P2PSection/P2PHBox/JoinBtn
@onready var seed_address_input: LineEdit = $MarginContainer/VBox/P2PSection/SeedAddressInput

@onready var restart_label: Label = $MarginContainer/VBox/RestartLabel
@onready var save_btn: Button = $MarginContainer/VBox/BtnHBox/SaveBtn
@onready var close_btn: Button = $MarginContainer/VBox/BtnHBox/CloseBtn

var current_llm_mode: String = "rule_only"
var current_icon_id: String = "default"
var current_p2p_mode: String = "single"
var restart_required: bool = false

func _ready():
    # 应用共享样式
    SharedUIScripts.apply_button_style(local_btn)
    SharedUIScripts.apply_button_style(remote_btn)
    SharedUIScripts.apply_button_style(rule_btn)
    SharedUIScripts.apply_input_style(agent_name_input)
    SharedUIScripts.apply_textedit_style(agent_prompt_input)
    SharedUIScripts.apply_input_style(seed_address_input)
    
    # 连接按钮事件
    local_btn.pressed.connect(_on_llm_mode_changed.bind("local"))
    remote_btn.pressed.connect(_on_llm_mode_changed.bind("remote"))
    rule_btn.pressed.connect(_on_llm_mode_changed.bind("rule_only"))
    
    for i in range(icon_buttons.size()):
        var icon_id = ["default", "wizard", "fox", "dragon", "lion", "robot"][i]
        icon_buttons[i].pressed.connect(_on_icon_selected.bind(icon_id))
    
    p2p_single_btn.pressed.connect(_on_p2p_mode_changed.bind("single"))
    p2p_create_btn.pressed.connect(_on_p2p_mode_changed.bind("create"))
    p2p_join_btn.pressed.connect(_on_p2p_mode_changed.bind("join"))
    
    save_btn.pressed.connect(_on_save_pressed)
    close_btn.pressed.connect(_on_close_pressed)
    
    # 加载配置
    load_config()

func load_config():
    if not bridge or not bridge.has_method("get_user_config"):
        return
    
    var config = bridge.get_user_config()
    
    # LLM 模式
    current_llm_mode = config.get("llm_mode", "rule_only")
    _update_llm_mode_ui()
    
    # Agent 名字
    agent_name_input.text = config.get("agent_name", "智行者")
    
    # custom_prompt
    agent_prompt_input.text = config.get("agent_custom_prompt", "")
    
    # icon
    current_icon_id = config.get("agent_icon_id", "default")
    _update_icon_ui()
    
    # P2P 模式
    current_p2p_mode = config.get("p2p_mode", "single")
    seed_address_input.text = config.get("p2p_seed_address", "")
    _update_p2p_mode_ui()

func _on_llm_mode_changed(mode: String):
    if mode != current_llm_mode:
        restart_required = true
        restart_label.text = "更改 LLM 模式需要重启生效"
    current_llm_mode = mode
    _update_llm_mode_ui()

func _on_icon_selected(icon_id: String):
    current_icon_id = icon_id
    _update_icon_ui()

func _on_p2p_mode_changed(mode: String):
    current_p2p_mode = mode
    _update_p2p_mode_ui()

func _on_save_pressed():
    var agent_name = agent_name_input.text.strip_edges()
    if agent_name.is_empty():
        restart_label.text = "Agent 名字不能为空！"
        restart_label.modulate = Color.RED
        return
    
    var config = {
        "llm_mode": current_llm_mode,
        "llm_api_endpoint": "",
        "llm_api_token": "",
        "llm_model_name": "",
        "llm_local_model_path": "",
        "agent_name": agent_name,
        "agent_custom_prompt": agent_prompt_input.text.strip_edges(),
        "agent_icon_id": current_icon_id,
        "agent_custom_icon_path": "",
        "p2p_mode": current_p2p_mode,
        "p2p_seed_address": seed_address_input.text.strip_edges()
    }
    
    if bridge and bridge.has_method("set_user_config"):
        var success = bridge.set_user_config(config)
        if success:
            if restart_required:
                restart_label.text = "配置已保存，请重启游戏"
                restart_label.modulate = Color.YELLOW
            else:
                restart_label.text = "配置已保存"
                restart_label.modulate = Color.GREEN
        else:
            restart_label.text = "配置保存失败"
            restart_label.modulate = Color.RED

func _on_close_pressed():
    hide()
```

### 4.5 异常处理

| 异常场景 | 处理策略 |
| --- | --- |
| Agent 名字为空 | 显示红色错误提示，不保存，聚焦到输入框 |
| Bridge 未就绪 | 显示提示 "Bridge 未就绪"，禁用保存按钮 |
| 配置保存失败 | 显示红色错误提示 "配置保存失败" |
| 图标资源不存在 | 使用 fallback 图标（default.png） |

## 5. 技术决策

### 决策1：settings_panel UI 实现方式

- **选型方案**：使用静态 .tscn 定义 + @onready 节点引用
- **选择理由**：
  1. 可在编辑器预览布局，调试效率更高
  2. 样式可保存为资源，避免每次启动重建
  3. 与 setup_wizard 动态构建不同，settings_panel 功能固定，适合静态定义
- **备选方案**：继续使用动态构建
- **放弃原因**：动态构建覆盖静态节点，调试困难，样式代码分散

### 决策2：共享样式库实现方式

- **选型方案**：创建 shared_ui_styles.gd 作为 Autoload 单例
- **选择理由**：
  1. 全局可访问，无需每个脚本手动引入
  2. 统一主题色和尺寸常量
  3. 减少代码重复，便于维护
- **备选方案**：在每个脚本中复制样式代码
- **放弃原因**：代码重复，样式不一致，维护困难

### 决策3：setup_wizard 静态化优先级

- **选型方案**：本次变更不静态化 setup_wizard.tscn
- **选择理由**：
  1. setup_wizard 功能已完成，不影响当前问题
  2. 静态化需要重构大量代码（500+ 行），风险较大
  3. 可作为后续迭代任务
- **备选方案**：同时静态化 setup_wizard
- **放弃原因**：变更范围过大，超出本次目标

## 6. 风险评估

| 风险点 | 风险等级 | 应对策略 |
| --- | --- | --- |
| @onready 节点路径不匹配 | 中 | 确保 settings_panel.tscn 节点结构与 @onready 路径一致 |
| shared_ui_styles.gd Autoload 未配置 | 中 | 需在 project.godot [autoload] 添加配置 |
| ESC 键冲突（其他 UI 弹窗） | 低 | 使用 input_as_handled() 消费事件，优先级处理 |
| 图标 TextureButton 加载失败 | 低 | 检查预设图标 PNG 文件存在，使用 default.png fallback |

## 7. 迁移方案

### 7.1 部署步骤

1. 创建 `shared_ui_styles.gd` 并配置为 Autoload
2. 修改 `settings_panel.tscn` 扩展 UI 结构
3. 重构 `settings_panel.gd` 使用静态节点引用
4. 修改 `main.tscn` 添加 SettingsBtn + SettingsPanel 节点
5. 修改 `main.gd` 连接 settings_panel 和 ESC 处理
6. 修改 `setup_wizard.gd` 使用 shared_ui_styles（可选）
7. 测试完整流程

### 7.2 灰度策略

- 本变更不涉及网络功能，可一次性发布
- 测试顺序：设置入口 → 配置加载 → 配置保存 → 重启提示

### 7.3 回滚方案

- 若 settings_panel 功能异常，可暂时禁用 SettingsBtn
- 若 shared_ui_styles 影响现有 UI，可暂时移除 Autoload 配置

## 8. 待定事项

- [ ] setup_wizard.tscn 静态化（后续迭代）
- [ ] "立即重启" 按钮（可选功能）
- [ ] 自定义图标上传 FileDialog（属于 setup-wizard 变更）