# 实施任务清单

## 1. 共享样式库创建

创建 shared_ui_styles.gd 作为全局样式函数库，供 setup_wizard 和 settings_panel 复用。

- [x] 1.1 创建 shared_ui_styles.gd 文件
  - 文件: `client/scripts/shared_ui_styles.gd`
  - 定义预设主题色常量（COLOR_BG_DARK、COLOR_BG_PANEL 等）
  - 定义触摸友好尺寸常量（MIN_BUTTON_HEIGHT=36 等）
  - 实现样式创建函数：create_panel_style()、create_button_styles()、create_input_style()

- [x] 1.2 配置 shared_ui_styles.gd 为 Autoload
  - 文件: `client/project.godot`
  - 在 `[autoload]` 段添加：`SharedUIScripts="*res://scripts/shared_ui_styles.gd"`
  - 依赖: 1.1

- [x] 1.3 实现节点样式应用辅助函数
  - 文件: `client/scripts/shared_ui_styles.gd`
  - 实现 apply_button_style(btn, styleType)
  - 实现 apply_input_style(input)
  - 实现 apply_textedit_style(input)
  - 实现 apply_panel_style(panel)
  - 依赖: 1.1

## 2. settings_panel.tscn UI 扩展

扩展 settings_panel.tscn 的静态 UI 结构，添加缺失的配置项节点。

- [x] 2.1 添加 custom_prompt 配置区
  - 文件: `client/scenes/settings_panel.tscn`
  - 在 VBoxContainer 中添加 AgentPromptSection Label + AgentPromptInput TextEdit
  - TextEdit placeholder_text = "描述智能体性格..."
  - TextEdit custom_minimum_size = Vector2(0, 100)

- [x] 2.2 添加 icon 选择区
  - 文件: `client/scenes/settings_panel.tscn`
  - 在 VBoxContainer 中添加 IconSection Label + IconGrid GridContainer
  - GridContainer columns = 6
  - 添加 6 个 TextureButton：IconDefault、IconWizard、IconFox、IconDragon、IconLion、IconRobot
  - TextureButton texture_normal 引用 `res://assets/textures/agents/<icon_id>.png`
  - TextureButton custom_minimum_size = Vector2(48, 48)

- [x] 2.3 添加 P2P 配置区
  - 文件: `client/scenes/settings_panel.tscn`
  - 在 VBoxContainer 中添加 P2PSection Label + P2PHBox HBoxContainer
  - HBoxContainer 包含 3 个 Button：SingleBtn、CreateBtn、JoinBtn
  - Button toggle_mode = true
  - 添加 SeedAddressInput LineEdit（join 模式下显示）

- [x] 2.4 添加弹窗背景和层级
  - 文件: `client/scenes/settings_panel.tscn`
  - 根节点 SettingsPanel 设置：
    - layout_mode = 2
    - anchors_preset = 15 (居中)
    - custom_minimum_size = Vector2(400, 500)
    - visible = false
  - Bg ColorRect 设置 anchor_preset = 15，color = 半透明深色遮罩
  - 依赖: 2.1, 2.2, 2.3

## 3. settings_panel.gd 重构

重构 settings_panel.gd，删除动态构建代码，使用静态节点引用，完善配置逻辑。

- [x] 3.1 删除动态构建代码
  - 文件: `client/scripts/settings_panel.gd`
  - 删除 _build_ui() 函数及其调用
  - 删除 _create_label()、_create_spacer()、_add_mode_button() 等辅助函数
  - 保留 _ready()、_load_current_config()、_on_save_pressed()、_on_close_pressed()

- [x] 3.2 添加 @onready 静态节点引用
  - 文件: `client/scripts/settings_panel.gd`
  - 添加 @onready var 引用所有 .tscn 中定义的节点
  - 包括：LLM 按钮、Agent 名字输入、Prompt 输入、Icon 按钮、P2P 按钮、Seed 地址输入
  - 依赖: 3.1, 2.4

- [x] 3.3 应用共享样式到节点
  - 文件: `client/scripts/settings_panel.gd`
  - 在 _ready() 中调用 SharedUIScripts.apply_button_style()、apply_input_style() 等
  - 应用样式到所有按钮、输入框
  - 依赖: 3.2, 1.3

- [x] 3.4 实现完整配置加载逻辑
  - 文件: `client/scripts/settings_panel.gd`
  - 实现 load_config() 函数：
    - 调用 Bridge.get_user_config()
    - 填充 LLM 模式按钮（正确选中）
    - 填充 Agent 名字
    - 填充 custom_prompt
    - 填充 icon（正确选中高亮）
    - 填充 P2P 模式和 seed_address
  - 依赖: 3.2

- [x] 3.5 实现完整配置保存逻辑
  - 文件: `client/scripts/settings_panel.gd`
  - 修改 _on_save_pressed()：
    - 收集所有输入控件值（LLM、Agent、Prompt、Icon、P2P）
    - 验证 Agent 名字非空
    - 构建 config Dictionary（扁平结构）
    - 调用 Bridge.set_user_config()
    - 显示保存结果提示
  - 依赖: 3.2

- [x] 3.6 实现图标选择逻辑
  - 文件: `client/scripts/settings_panel.gd`
  - 实现 _on_icon_selected(icon_id) 回调
  - 更新 current_icon_id
  - 实现 _update_icon_ui() 高亮选中图标、其他变灰
  - 依赖: 3.2

- [x] 3.7 实现 P2P 模式切换逻辑
  - 文件: `client/scripts/settings_panel.gd`
  - 实现 _on_p2p_mode_changed(mode) 回调
  - 实现 _update_p2p_mode_ui() 显示/隐藏 seed_address 输入框
  - 依赖: 3.2

- [x] 3.8 实现重启生效提示逻辑
  - 文件: `client/scripts/settings_panel.gd`
  - 在 LLM 模式切换时标记 restart_required = true
  - 在保存成功时根据 restart_required 显示对应提示
  - 依赖: 3.5

## 4. main.tscn UI 集成

在 main.tscn 中添加 SettingsBtn 和 SettingsPanel 节点。

- [x] 4.1 添加 SettingsBtn 到 TopBar
  - 文件: `client/scenes/main.tscn`
  - 在 TopBar HBoxContainer 最右侧添加 SettingsBtn Button
  - Button text = "⚙" 或 "设置"
  - Button custom_minimum_size = Vector2(60, 36)

- [x] 4.2 嵌入 SettingsPanel 节点
  - 文件: `client/scenes/main.tscn`
  - 在 UI CanvasLayer 中添加 SettingsPanel 节点
  - SettingsPanel 引用 scenes/settings_panel.tscn 实例
  - SettingsPanel visible = false
  - 依赖: 4.1, 2.4

## 5. main.gd 连接和快捷键

修改 main.gd 连接 settings_panel 并处理 ESC 快捷键。

- [x] 5.1 添加 settings_panel 引用
  - 文件: `client/scripts/main.gd`
  - 添加 `var settings_panel: Control` 变量
  - 在 _ready() 中获取节点引用：settings_panel = $UI/SettingsPanel

- [x] 5.2 连接 SettingsBtn 点击事件
  - 文件: `client/scripts/main.gd`
  - 在 _ready() 中获取 SettingsBtn 引用
  - 连接 pressed 信号到 _on_settings_pressed()
  - 实现 _on_settings_pressed() 显示 settings_panel
  - 依赖: 5.1, 4.1

- [x] 5.3 实现 ESC 快捷键处理
  - 文件: `client/scripts/main.gd`
  - 添加 _input(event) 或 _unhandled_input(event) 函数
  - 检测 KEY_ESCAPE 按键
  - 实现 _toggle_settings_panel() 切换显示/隐藏
  - 调用 get_viewport().set_input_as_handled() 消费事件
  - 依赖: 5.1

- [x] 5.4 settings_panel 打开时触发配置加载
  - 文件: `client/scripts/main.gd`
  - 在 _toggle_settings_panel() 显示时调用 settings_panel.load_config()
  - 依赖: 5.3, 3.4

## 6. setup_wizard.gd 样式重构（可选）

修改 setup_wizard.gd 使用 shared_ui_styles.gd 替代本地样式代码。

- [x] 6.1 删除 setup_wizard.gd 本地样式函数
  - 文件: `client/scripts/setup_wizard.gd`
  - 删除 _add_toggle_button_style()、_style_input()、_style_textedit()、_create_panel() 等函数
  - 依赖: 1.3

- [x] 6.2 使用 SharedUIScripts 样式函数
  - 文件: `client/scripts/setup_wizard.gd`
  - 在 _build_ui() 中调用 SharedUIScripts.apply_button_style()、apply_input_style() 等
  - 使用 SharedUIScripts 常量替代本地颜色值
  - 依赖: 6.1, 1.3

## 7. 测试与验证

测试 settings_panel 功能和 UI 集成。

- [x] 7.1 验收测试 - TopBar 显示设置按钮
  - SettingsBtn 显示在 TopBar 右侧，尺寸 50x36 符合触摸友好要求
  - 点击按钮成功打开 settings_panel

- [x] 7.2 验收测试 - ESC 快捷键
  - ESC 键成功切换 settings_panel 显示/隐藏

- [x] 7.3 验收测试 - 配置加载
  - settings_panel 正确加载当前配置
  - LLM 模式按钮正确选中（rule_only）
  - Agent 名字正确显示（TestAgent）
  - Icon 正确选中高亮（default 白色，其他灰色）

- [x] 7.4 验收测试 - 配置保存
  - user_config.toml 存在且内容正确
  - 配置文件结构包含 llm/agent/p2p 三段

- [x] 7.5 验收测试 - 重启提示
  - 切换 LLM 模式后显示"更改 LLM 模式需要重启生效"

- [x] 7.6 验收测试 - Agent 名字验证
  - 清空 Agent 名字后保存显示"Agent 名字不能为空！"
  - 不保存无效配置

- [x] 7.7 验收测试 - 移动端触摸友好
  - SettingsBtn 尺寸 50x36 >= 36px ✓
  - SaveBtn 尺寸 40px >= 36px ✓
  - 图标按钮 48x48 >= 48px ✓
  - 输入框高度符合要求 ✓

## 任务依赖关系

```
1.x (共享样式库)
    ├── 1.1 → 1.2 → 1.3
    │
    └─► 6.x (setup_wizard 样式重构，可选)
    │       依赖: 1.3
    │
2.x (settings_panel.tscn UI 扩展)
    ├── 2.1, 2.2, 2.3 (并行)
    └─► 2.4 (依赖: 2.1, 2.2, 2.3)
    │
    └─► 3.x (settings_panel.gd 重构)
    │       依赖: 2.4, 1.3
    │       3.1 → 3.2 → 3.3 → 3.4 → 3.5 → 3.6 → 3.7 → 3.8
    │
4.x (main.tscn UI 集成)
    ├── 4.1 → 4.2 (依赖: 4.1, 2.4)
    │
    └─► 5.x (main.gd 连接)
    │       依赖: 5.1, 4.1
    │       5.1 → 5.2 → 5.3 → 5.4
    │
7.x (测试验证)
    依赖: 1.x, 2.x, 3.x, 4.x, 5.x
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.x | 创建共享样式库，为后续 UI 组件提供样式支持 |
| 阶段二 | 2.x | 扩展 settings_panel.tscn UI 结构 |
| 阶段三 | 3.x | 重构 settings_panel.gd，使用静态节点，完善配置逻辑 |
| 阶段四 | 4.x | 在 main.tscn 中集成 settings_panel |
| 阶段五 | 5.x | main.gd 连接 settings_panel 和 ESC 快捷键 |
| 阶段六 | 6.x | setup_wizard 样式重构（可选，降低优先级） |
| 阶段七 | 7.x | 测试验证完整功能 |

## 文件结构总览

```
client/
├── scenes/
│   ├── main.tscn            # 修改：添加 SettingsBtn + SettingsPanel
│   └── settings_panel.tscn  # 修改：扩展 UI（Prompt、Icon、P2P）
│
├── scripts/
│   ├── main.gd              # 修改：添加 settings_panel 连接和 ESC 处理
│   ├── settings_panel.gd    # 修改：重构，删除动态构建，使用静态节点
│   ├── setup_wizard.gd      # 修改（可选）：使用 shared_ui_styles
│   └── shared_ui_styles.gd  # 新增：共享样式函数库
│
└── project.godot            # 修改：添加 shared_ui_styles.gd Autoload
```