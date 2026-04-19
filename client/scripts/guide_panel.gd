# GuidePanel - 玩家引导面板
# 状态引导按钮 + 暂停控制
extends VBoxContainer

var _selected_agent_id: String = ""


func _ready() -> void:
	_setup_guide_buttons()
	_setup_control_buttons()

	var bridge = get_node_or_null("../../../../SimulationBridge")
	if bridge:
		bridge.agent_selected.connect(_on_agent_selected)


func _setup_guide_buttons() -> void:
	# 标题
	var title = Label.new()
	title.text = "状态引导"
	title.add_theme_font_size_override("font_size", 12)
	title.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	add_child(title)

	# 引导按钮（2行2列）
	var grid = GridContainer.new()
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 3)
	grid.add_theme_constant_override("v_separation", 3)

	var guides = [
		{"name": "进食", "key": "eat", "tooltip": "引导Agent进食恢复饱食度"},
		{"name": "饮水", "key": "drink", "tooltip": "引导Agent饮水恢复水分度"},
		{"name": "采集", "key": "gather", "tooltip": "引导Agent采集资源"},
		{"name": "探索", "key": "explore", "tooltip": "引导Agent探索周围"},
	]

	for guide in guides:
		var btn = Button.new()
		btn.text = guide.name
		btn.tooltip_text = "%s\n点击临时提升该倾向" % guide.tooltip
		btn.custom_minimum_size = Vector2(60, 26)
		btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		btn.pressed.connect(_inject_guide.bind(guide.key))
		grid.add_child(btn)

	add_child(grid)


func _setup_control_buttons() -> void:
	var sep = HSeparator.new()
	add_child(sep)

	var ctrl_hbox = HBoxContainer.new()
	ctrl_hbox.add_theme_constant_override("separation", 4)

	var pause_btn = Button.new()
	pause_btn.text = "暂停"
	pause_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	pause_btn.pressed.connect(_toggle_pause)
	ctrl_hbox.add_child(pause_btn)

	add_child(ctrl_hbox)


func _on_agent_selected(agent_id: String) -> void:
	_selected_agent_id = agent_id


func _inject_guide(key: String) -> void:
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			bridge.inject_preference(_selected_agent_id, key, 0.5, 15)


func _toggle_pause() -> void:
	var bridge = get_node_or_null("../../../../SimulationBridge")
	if bridge:
		if bridge.is_paused:
			bridge.start()
		else:
			bridge.pause()
