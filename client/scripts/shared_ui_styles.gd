# shared_ui_styles.gd - 共享 UI 样式函数库
# 供 setup_wizard.gd 和 settings_panel.gd 复用

extends Node

# === 预设主题色 ===
const COLOR_BG_DARK := Color(0.12, 0.14, 0.16, 1.0)
const COLOR_BG_PANEL := Color(0.18, 0.20, 0.22, 1.0)
const COLOR_BG_INPUT := Color(0.22, 0.24, 0.26, 1.0)
const COLOR_BUTTON_NORMAL := Color(0.20, 0.22, 0.25, 1.0)
const COLOR_BUTTON_HOVER := Color(0.30, 0.32, 0.35, 1.0)
const COLOR_BUTTON_PRESSED := Color(0.25, 0.50, 0.65, 1.0)
const COLOR_BUTTON_SUCCESS := Color(0.20, 0.55, 0.30, 1.0)
const COLOR_BUTTON_SUCCESS_HOVER := Color(0.30, 0.65, 0.40, 1.0)
const COLOR_TEXT_PRIMARY := Color(0.90, 0.90, 0.90, 1.0)
const COLOR_TEXT_SECONDARY := Color(0.70, 0.70, 0.70, 1.0)
const COLOR_TEXT_PLACEHOLDER := Color(0.50, 0.50, 0.50, 1.0)
const COLOR_TEXT_HIGHLIGHT := Color(0.40, 0.70, 0.90, 1.0)

# === 触摸友好尺寸 ===
const MIN_BUTTON_HEIGHT := 36
const MIN_INPUT_HEIGHT := 36
const MIN_ICON_SIZE := 48
const TOUCH_TARGET_MIN := 44
const CORNER_RADIUS_SMALL := 4
const CORNER_RADIUS_MEDIUM := 6
const CORNER_RADIUS_LARGE := 8

# === 样式创建函数 ===

## 创建面板样式
static func create_panel_style(cornerRadius: int = CORNER_RADIUS_LARGE, bgColor: Color = COLOR_BG_PANEL) -> StyleBoxFlat:
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

## 创建按钮样式组（返回 Dictionary: {normal, hover, pressed}）
static func create_button_styles(styleType: String = "normal") -> Dictionary:
	var normal := StyleBoxFlat.new()
	var hover := StyleBoxFlat.new()
	var pressed := StyleBoxFlat.new()

	var baseRadius := CORNER_RADIUS_MEDIUM

	if styleType == "success":
		normal.bg_color = COLOR_BUTTON_SUCCESS
		hover.bg_color = COLOR_BUTTON_SUCCESS_HOVER
		pressed.bg_color = Color(0.25, 0.50, 0.35, 1.0)
	elif styleType == "toggle":
		normal.bg_color = COLOR_BUTTON_NORMAL
		hover.bg_color = COLOR_BUTTON_HOVER
		pressed.bg_color = COLOR_BUTTON_PRESSED
	else:
		normal.bg_color = COLOR_BUTTON_NORMAL
		hover.bg_color = COLOR_BUTTON_HOVER
		pressed.bg_color = Color(0.35, 0.40, 0.45, 1.0)

	for style in [normal, hover, pressed]:
		style.corner_radius_top_left = baseRadius
		style.corner_radius_top_right = baseRadius
		style.corner_radius_bottom_left = baseRadius
		style.corner_radius_bottom_right = baseRadius

	return {"normal": normal, "hover": hover, "pressed": pressed}

## 创建输入框样式
static func create_input_style() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = COLOR_BG_INPUT
	style.corner_radius_top_left = CORNER_RADIUS_SMALL
	style.corner_radius_top_right = CORNER_RADIUS_SMALL
	style.corner_radius_bottom_left = CORNER_RADIUS_SMALL
	style.corner_radius_bottom_right = CORNER_RADIUS_SMALL
	style.content_margin_left = 10
	style.content_margin_right = 10
	return style

## 创建文本编辑框样式
static func create_textedit_style() -> StyleBoxFlat:
	return create_input_style()

## 创建深色背景样式（用于弹窗遮罩）
static func create_dark_bg_style() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.0, 0.0, 0.0, 0.7)
	return style

# === 样式应用函数 ===

## 应用样式到按钮
static func apply_button_style(btn: Button, styleType: String = "normal"):
	var styles := create_button_styles(styleType)
	btn.add_theme_stylebox_override("normal", styles["normal"])
	btn.add_theme_stylebox_override("hover", styles["hover"])
	btn.add_theme_stylebox_override("pressed", styles["pressed"])
	btn.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	btn.add_theme_color_override("font_hover_color", Color(1.0, 1.0, 1.0, 1.0))
	btn.add_theme_color_override("font_pressed_color", Color(1.0, 1.0, 1.0, 1.0))

	# 设置最小高度（触摸友好）
	if btn.custom_minimum_size.y < MIN_BUTTON_HEIGHT:
		btn.custom_minimum_size.y = MIN_BUTTON_HEIGHT

## 应用样式到切换按钮（带 ButtonGroup）
static func apply_toggle_button_style(btn: Button):
	apply_button_style(btn, "toggle")

## 应用样式到 LineEdit 输入框
static func apply_input_style(input: LineEdit):
	input.add_theme_stylebox_override("normal", create_input_style())
	input.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	input.add_theme_color_override("font_placeholder_color", COLOR_TEXT_PLACEHOLDER)
	input.add_theme_color_override("caret_color", COLOR_TEXT_PRIMARY)

	# 设置最小高度
	if input.custom_minimum_size.y < MIN_INPUT_HEIGHT:
		input.custom_minimum_size.y = MIN_INPUT_HEIGHT

## 应用样式到 TextEdit 文本编辑框
static func apply_textedit_style(input: TextEdit):
	input.add_theme_stylebox_override("normal", create_textedit_style())
	input.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	input.add_theme_color_override("font_placeholder_color", COLOR_TEXT_PLACEHOLDER)
	input.add_theme_color_override("caret_color", COLOR_TEXT_PRIMARY)

## 应用样式到 PanelContainer
static func apply_panel_style(panel: PanelContainer, cornerRadius: int = CORNER_RADIUS_LARGE):
	panel.add_theme_stylebox_override("panel", create_panel_style(cornerRadius))

## 创建带样式的 Label
static func create_label(text: String, fontSize: int = 14, color: Color = COLOR_TEXT_PRIMARY) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", fontSize)
	label.add_theme_color_override("font_color", color)
	return label

## 创建带样式的 Section Header Label
static func create_section_header(text: String) -> Label:
	return create_label(text, 18, COLOR_TEXT_HIGHLIGHT)

## 创建间隔 spacer
static func create_spacer(height: int) -> Control:
	var spacer := Control.new()
	spacer.custom_minimum_size = Vector2(0, height)
	return spacer

## 应用样式到 TextureButton（图标按钮）
static func apply_icon_button_style(btn: TextureButton):
	btn.custom_minimum_size = Vector2(MIN_ICON_SIZE, MIN_ICON_SIZE)
	btn.stretch_mode = TextureButton.STRETCH_KEEP_ASPECT_CENTERED
	btn.ignore_texture_size = true