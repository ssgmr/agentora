# 功能规格说明 - 共享 UI 样式库

## ADDED Requirements

### Requirement: 共享样式函数库

系统 SHALL 创建 shared_ui_styles.gd 作为共享样式函数库，供 setup_wizard.gd 和 settings_panel.gd 复用。

#### Scenario: 样式函数定义

- **WHEN** shared_ui_styles.gd 加载
- **THEN** SHALL 提供以下样式函数：
  - `create_panel_style()` - 创建圆角深色面板样式
  - `create_button_style()` - 创建按钮样式（normal/hover/pressed）
  - `create_input_style()` - 创建输入框样式
  - `create_textedit_style()` - 创建文本编辑框样式
  - `create_toggle_button_style()` - 创建切换按钮样式
  - `create_label_style()` - 创建标签样式

#### Scenario: 样式参数可配置

- **WHEN** 调用样式函数
- **THEN** SHALL 支持参数覆盖默认值：
  - `bg_color` - 背景颜色
  - `corner_radius` - 圆角半径
  - `font_size` - 字体大小
  - `font_color` - 字体颜色

#### Scenario: 预设主题色

- **WHEN** shared_ui_styles.gd 加载
- **THEN** SHALL 定义预设主题色常量：
  - `COLOR_BG_DARK` = Color(0.12, 0.14, 0.16, 1.0)
  - `COLOR_BG_PANEL` = Color(0.18, 0.20, 0.22, 1.0)
  - `COLOR_BG_INPUT` = Color(0.22, 0.24, 0.26, 1.0)
  - `COLOR_BUTTON_NORMAL` = Color(0.20, 0.22, 0.25, 1.0)
  - `COLOR_BUTTON_HOVER` = Color(0.30, 0.32, 0.35, 1.0)
  - `COLOR_BUTTON_PRESSED` = Color(0.25, 0.50, 0.65, 1.0)
  - `COLOR_BUTTON_SUCCESS` = Color(0.20, 0.55, 0.30, 1.0)
  - `COLOR_TEXT_PRIMARY` = Color(0.90, 0.90, 0.90, 1.0)
  - `COLOR_TEXT_SECONDARY` = Color(0.70, 0.70, 0.70, 1.0)
  - `COLOR_TEXT_PLACEHOLDER` = Color(0.50, 0.50, 0.50, 1.0)

### Requirement: 节点创建辅助函数

系统 SHALL 提供节点创建辅助函数，简化 UI 构建。

#### Scenario: 创建带样式的按钮

- **WHEN** 调用 `create_styled_button(text, style_type)`
- **THEN** SHALL 返回配置好样式的 Button 节点
- **AND** `style_type` 可选值：`normal`、`toggle`、`success`

#### Scenario: 创建带样式的输入框

- **WHEN** 调用 `create_styled_input(placeholder, min_size)`
- **THEN** SHALL 返回配置好样式的 LineEdit 节点
- **AND** 自动设置 placeholder 和 minimum_size

#### Scenario: 创建面板容器

- **WHEN** 调用 `create_styled_panel()`
- **THEN** SHALL 返回配置好样式的 PanelContainer 节点
- **AND** 自动应用圆角深色背景

### Requirement: 触摸友好尺寸常量

系统 SHALL 定义触摸友好的最小尺寸常量。

#### Scenario: 最小尺寸定义

- **WHEN** shared_ui_styles.gd 加载
- **THEN** SHALL 定义：
  - `MIN_BUTTON_HEIGHT` = 36 (按钮最小高度)
  - `MIN_INPUT_HEIGHT` = 36 (输入框最小高度)
  - `MIN_ICON_SIZE` = 48 (图标最小尺寸)
  - `TOUCH_TARGET_MIN` = 44 (触摸目标最小尺寸，符合移动端标准)

### Requirement: setup_wizard 和 settings_panel 集成

setup_wizard.gd 和 settings_panel.gd SHALL 使用 shared_ui_styles.gd 替代各自的样式代码。

#### Scenario: setup_wizard 使用共享样式

- **WHEN** setup_wizard.gd 构建 UI
- **THEN** SHALL 调用 shared_ui_styles 的样式函数
- **AND** 删除 `_add_toggle_button_style()`、`_style_input()` 等本地样式函数

#### Scenario: settings_panel 使用共享样式

- **WHEN** settings_panel.gd 构建 UI
- **THEN** SHALL 调用 shared_ui_styles 的样式函数
- **AND** 确保样式与 setup_wizard 一致