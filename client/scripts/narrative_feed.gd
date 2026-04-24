# NarrativeFeed - 叙事流面板
# RichTextLabel滚动显示Agent决策叙事
# 支持频道切换（本地/附近/世界）和 Agent 过滤
extends PanelContainer

@export var max_events: int = 200
@export var auto_scroll: bool = true

var _text_box: RichTextLabel
var _scroll_container: ScrollContainer
var _channel_buttons: HBoxContainer
var _current_channel: String = "local"  # local/nearby/world

# 事件缓存（用于过滤）
var _cached_events: Array = []


func _ready() -> void:
	# 给 PanelContainer 加半透明深色背景
	var panel_bg = StyleBoxFlat.new()
	panel_bg.bg_color = Color(0, 0, 0, 0.6)
	add_theme_stylebox_override("panel", panel_bg)

	# 先尝试获取 tscn 中预定义的节点
	_scroll_container = get_node_or_null("VBoxContainer/ScrollContainer")
	_text_box = get_node_or_null("VBoxContainer/ScrollContainer/EventText")

	# 检查是否已有预定义 UI
	if _scroll_container and _text_box:
		# 使用 tscn 预定义结构
		_setup_styling()
		# 查找或创建频道按钮容器
		_setup_channel_tabs_in_existing_ui()
	else:
		# 完全动态创建 UI
		_setup_ui_fallback()

	# 订阅 StateManager 信号（统一状态分发）
	StateManager.narrative_added.connect(_on_narrative_added)
	StateManager.filter_changed.connect(_on_filter_changed)
	print("[NarrativeFeed] StateManager 信号已连接")


func _setup_channel_tabs_in_existing_ui() -> void:
	# 查找 FilterBar/ChannelSelector 中的按钮
	var channel_selector = get_node_or_null("VBoxContainer/FilterBar/ChannelSelector")
	if channel_selector:
		# 已有 ChannelSelector，获取按钮并连接信号
		for btn in channel_selector.get_children():
			if btn is Button and btn.name.begins_with("Btn_"):
				var channel = btn.name.replace("Btn_", "")
				btn.pressed.connect(_on_channel_button_pressed.bind(channel))
		_channel_buttons = channel_selector
	else:
		# FilterBar 不存在，动态添加频道按钮
		var vbox = get_node_or_null("VBoxContainer")
		if vbox:
			_channel_buttons = HBoxContainer.new()
			_channel_buttons.name = "ChannelTabs"
			vbox.add_child(_channel_buttons)
			vbox.move_child(_channel_buttons, 0)  # 放到最上方
			_add_channel_buttons()


func _add_channel_buttons() -> void:
	var channels = ["local", "nearby", "world"]
	var channel_names = {"local": "本地", "nearby": "附近", "world": "世界"}
	for channel in channels:
		var btn = Button.new()
		btn.text = channel_names[channel]
		btn.name = "Btn_" + channel
		btn.toggle_mode = true
		btn.button_pressed = (channel == _current_channel)
		btn.pressed.connect(_on_channel_button_pressed.bind(channel))
		_channel_buttons.add_child(btn)


func _on_channel_button_pressed(channel: String) -> void:
	# 更新当前频道
	_current_channel = channel
	StateManager.set_narrative_channel(channel)

	# 更新按钮状态
	if _channel_buttons:
		for btn in _channel_buttons.get_children():
			if btn is Button:
				btn.button_pressed = (btn.name == "Btn_" + channel)

	# 重新渲染叙事
	_refresh_display()
	print("[NarrativeFeed] 频道切换为: %s" % channel)


func _on_filter_changed() -> void:
	# 过滤条件变化，重新渲染
	_refresh_display()


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
	# 完全动态创建 UI
	var vbox = VBoxContainer.new()
	vbox.name = "VBoxContainer"
	add_child(vbox)

	# 频道切换按钮
	_channel_buttons = HBoxContainer.new()
	_channel_buttons.name = "ChannelTabs"
	vbox.add_child(_channel_buttons)
	_add_channel_buttons()

	# ScrollContainer + RichTextLabel
	_scroll_container = ScrollContainer.new()
	_scroll_container.name = "ScrollContainer"
	_scroll_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_scroll_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll_container.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_scroll_container.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	vbox.add_child(_scroll_container)

	_text_box = RichTextLabel.new()
	_text_box.name = "EventText"
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
	# 使用过滤后的显示
	_refresh_display()


func _refresh_display() -> void:
	# 获取过滤后的叙事
	var filtered = StateManager.get_filtered_narratives()

	# 清空并重建显示
	_text_box.text = ""
	for event in filtered:
		_add_event_direct(event)

	# 自动滚动到底部
	if auto_scroll and filtered.size() > 0:
		await get_tree().process_frame
		_scroll_container.scroll_vertical = int(_scroll_container.get_v_scroll_bar().max_value)


func _add_event_direct(event: Dictionary) -> void:
	var tick: int = event.get("tick", 0)
	var agent_name: String = event.get("agent_name", "Unknown")
	var description: String = event.get("description", "")
	var color: String = event.get("color_code", "#FFFFFF")

	# 格式化事件文本
	var formatted: String = "[color=%s][tick %d] %s[/color]\n" % [color, tick, description]

	# 添加到文本框
	var current_text = _text_box.text
	if current_text.begins_with("[i]等待"):
		current_text = ""

	_text_box.text = current_text + formatted

	# 限制最大事件数
	_limit_events()


func _limit_events() -> void:
	var lines = _text_box.text.split("\n")
	if lines.size() > max_events * 2:
		lines = lines.slice(-max_events * 2)
		_text_box.text = "\n".join(lines)


func clear_log() -> void:
	_cached_events.clear()
	_text_box.text = "[i]日志已清空[/i]"