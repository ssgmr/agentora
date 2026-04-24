# NarrativeFeed - 叙事流面板
# RichTextLabel滚动显示Agent决策叙事
extends PanelContainer

@export var max_events: int = 200
@export var auto_scroll: bool = true

var _text_box: RichTextLabel
var _scroll_container: ScrollContainer

# 事件缓存（用于过滤）
var _cached_events: Array = []


func _ready() -> void:
	# 给 PanelContainer 加半透明深色背景
	var panel_bg = StyleBoxFlat.new()
	panel_bg.bg_color = Color(0, 0, 0, 0.6)
	add_theme_stylebox_override("panel", panel_bg)

	# 使用 tscn 中预定义的节点（新结构：VBoxContainer/ScrollContainer/EventText）
	_scroll_container = get_node_or_null("VBoxContainer/ScrollContainer")
	_text_box = get_node_or_null("VBoxContainer/ScrollContainer/EventText")

	# 如果 tscn 节点不存在，回退到动态创建
	if not _scroll_container or not _text_box:
		_setup_ui_fallback()
	else:
		_setup_styling()

	# 订阅 StateManager 信号（统一状态分发）
	StateManager.narrative_added.connect(_on_narrative_added)
	print("[NarrativeFeed] StateManager.narrative_added 信号已连接")


func _setup_styling() -> void:
	_text_box.add_theme_color_override("default_color", Color.WHITE)
	_text_box.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	var bg_style = StyleBoxFlat.new()
	bg_style.bg_color = Color(0, 0, 0, 0.7)
	bg_style.content_margin_left = 8
	bg_style.content_margin_right = 8
	bg_style.content_margin_top = 4
	bg_style.content_margin_bottom = 4
	_text_box.add_theme_stylebox_override("normal", bg_style)
	_text_box.text = "[i]等待模拟开始...[/i]"


func _setup_ui_fallback() -> void:
	_scroll_container = ScrollContainer.new()
	_scroll_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_scroll_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll_container.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_scroll_container.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	add_child(_scroll_container)

	_text_box = RichTextLabel.new()
	_text_box.bbcode_enabled = true
	_text_box.fit_content = true
	_text_box.scroll_active = false
	_text_box.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_text_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_text_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll_container.add_child(_text_box)

	_setup_styling()


func _on_narrative_added(event: Variant) -> void:
	print("[NarrativeFeed] 收到事件: type=%s agent=%s desc=%s" % [
		event.get("event_type", "?"), event.get("agent_name", "?"), event.get("description", "?")
	])
	_cached_events.append(event)
	# 直接显示事件（不做过滤，过滤功能暂时禁用）
	_add_event_direct(event)


func _add_event_direct(event: Dictionary) -> void:
	var tick: int = event.get("tick", 0)
	var agent_name: String = event.get("agent_name", "Unknown")
	var event_type: String = event.get("event_type", "unknown")
	var description: String = event.get("description", "")
	var reasoning: String = event.get("reasoning", "")
	var color: String = event.get("color", "#FFFFFF")

	# 格式化事件文本
	var formatted: String = "[color=%s][tick %d] %s[/color]\n" % [color, tick, description]

	# 如果有 reasoning，添加思考内容（灰色小字）
	if reasoning != "":
		formatted += "[color=#888888]  思考: %s[/color]\n" % reasoning

	# 添加到文本框
	var current_text = _text_box.text
	if current_text.begins_with("[i]等待"):
		current_text = ""

	_text_box.text = current_text + formatted

	# 自动滚动到底部
	if auto_scroll:
		await get_tree().process_frame
		_scroll_container.scroll_vertical = int(_scroll_container.get_v_scroll_bar().max_value)

	# 限制最大事件数
	_limit_events()


func _limit_events() -> void:
	var lines = _text_box.text.split("\n")
	if lines.size() > max_events * 2:  # 每个事件可能有两行（描述+思考）
		lines = lines.slice(-max_events * 2)
		_text_box.text = "\n".join(lines)


func clear_log() -> void:
	_cached_events.clear()
	_text_box.text = "[i]日志已清空[/i]"