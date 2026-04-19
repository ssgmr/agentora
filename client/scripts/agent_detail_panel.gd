# AgentDetailPanel - Agent详情面板
# 显示选中Agent的HP/饱食度/水分度条 + 背包资源列表 + 状态信息 + 引导按钮
extends PanelContainer

var _selected_agent_id: String = ""
var _hp_bar: ProgressBar = null
var _satiety_bar: ProgressBar = null
var _hydration_bar: ProgressBar = null
var _hp_label: Label = null
var _satiety_label: Label = null
var _hydration_label: Label = null
var _inventory_label: Label = null
var _content_vbox: VBoxContainer

# Agent 名称标签
var _name_label: Label = null

# 状态显示标签
var _action_label: Label = null
var _result_label: Label = null
var _level_label: Label = null

# 思考内容标签（reasoning）
var _reasoning_label: Label = null


func _ready() -> void:
	# 面板背景
	var panel_bg = StyleBoxFlat.new()
	panel_bg.bg_color = Color(0, 0, 0, 0.5)
	panel_bg.content_margin_left = 6
	panel_bg.content_margin_right = 6
	panel_bg.content_margin_top = 6
	panel_bg.content_margin_bottom = 6
	add_theme_stylebox_override("panel", panel_bg)

	# 内容容器
	_content_vbox = VBoxContainer.new()
	_content_vbox.add_theme_constant_override("separation", 12)
	_content_vbox.custom_minimum_size = Vector2(0, 80)
	add_child(_content_vbox)

	_setup_ui()

	# 连接信号
	var bridge = get_node_or_null("/root/Main/SimulationBridge")
	if bridge:
		bridge.agent_selected.connect(_on_agent_selected)
		bridge.world_updated.connect(_on_world_updated)

	# 初始隐藏
	visible = false


func _setup_ui() -> void:
	# Agent 名称标签
	_name_label = Label.new()
	_name_label.text = "Agent 名称"
	_name_label.add_theme_font_size_override("font_size", 16)
	_name_label.add_theme_color_override("font_color", Color.WHITE)
	_name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_content_vbox.add_child(_name_label)

	# 状态条行（HP + 饱食 + 水分 并排）
	var bars_hbox = HBoxContainer.new()
	bars_hbox.add_theme_constant_override("separation", 4)
	bars_hbox.custom_minimum_size = Vector2(0, 40)
	bars_hbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var hp_result = _create_status_bar("HP", Color(0.9, 0.2, 0.2))
	_hp_bar = hp_result["bar"]
	_hp_label = hp_result["label"]
	bars_hbox.add_child(hp_result["container"])

	var satiety_result = _create_status_bar("饱食", Color(0.2, 0.8, 0.2))
	_satiety_bar = satiety_result["bar"]
	_satiety_label = satiety_result["label"]
	bars_hbox.add_child(satiety_result["container"])

	var hydration_result = _create_status_bar("水分", Color(0.2, 0.5, 0.9))
	_hydration_bar = hydration_result["bar"]
	_hydration_label = hydration_result["label"]
	bars_hbox.add_child(hydration_result["container"])

	_content_vbox.add_child(bars_hbox)

	# 背包内容
	var inv_hbox = HBoxContainer.new()
	inv_hbox.add_theme_constant_override("separation", 2)
	inv_hbox.custom_minimum_size = Vector2(0, 26)
	inv_hbox.size_flags_vertical = Control.SIZE_SHRINK_CENTER

	var inv_title = Label.new()
	inv_title.text = "背包:"
	inv_title.add_theme_font_size_override("font_size", 13)
	inv_title.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	inv_hbox.add_child(inv_title)

	_inventory_label = Label.new()
	_inventory_label.text = "（空）"
	_inventory_label.add_theme_font_size_override("font_size", 13)
	_inventory_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	inv_hbox.add_child(_inventory_label)

	_content_vbox.add_child(inv_hbox)

	# 动作行（显示当前动作）
	var action_hbox = HBoxContainer.new()
	action_hbox.add_theme_constant_override("separation", 2)
	action_hbox.custom_minimum_size = Vector2(0, 22)

	var action_title = Label.new()
	action_title.text = "动作:"
	action_title.add_theme_font_size_override("font_size", 13)
	action_title.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	action_hbox.add_child(action_title)

	_action_label = Label.new()
	_action_label.text = "等待"
	_action_label.add_theme_font_size_override("font_size", 13)
	_action_label.add_theme_color_override("font_color", Color.WHITE)
	_action_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	action_hbox.add_child(_action_label)

	_content_vbox.add_child(action_hbox)

	# 结果行（显示上次动作结果）
	var result_hbox = HBoxContainer.new()
	result_hbox.add_theme_constant_override("separation", 2)
	result_hbox.custom_minimum_size = Vector2(0, 22)

	var result_title = Label.new()
	result_title.text = "结果:"
	result_title.add_theme_font_size_override("font_size", 12)
	result_title.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	result_hbox.add_child(result_title)

	_result_label = Label.new()
	_result_label.text = "无"
	_result_label.add_theme_font_size_override("font_size", 12)
	_result_label.add_theme_color_override("font_color", Color(0.3, 0.8, 0.3))  # 默认绿色
	_result_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_result_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	result_hbox.add_child(_result_label)

	_content_vbox.add_child(result_hbox)

	# 等级行（显示等级徽章）
	var level_hbox = HBoxContainer.new()
	level_hbox.add_theme_constant_override("separation", 2)
	level_hbox.custom_minimum_size = Vector2(0, 22)

	var level_title = Label.new()
	level_title.text = "等级:"
	level_title.add_theme_font_size_override("font_size", 14)
	level_title.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	level_hbox.add_child(level_title)

	_level_label = Label.new()
	_level_label.text = "Lv.1"
	_level_label.add_theme_font_size_override("font_size", 14)
	_level_label.add_theme_color_override("font_color", Color(1, 0.8, 0.2))  # 金色
	_level_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	level_hbox.add_child(_level_label)

	_content_vbox.add_child(level_hbox)

	# 分隔线
	var sep1 = HSeparator.new()
	_content_vbox.add_child(sep1)

	# 思考内容标题
	var reasoning_title = Label.new()
	reasoning_title.text = "🧠 思考内容"
	reasoning_title.add_theme_font_size_override("font_size", 14)
	reasoning_title.add_theme_color_override("font_color", Color(0.9, 0.7, 0.3))
	_content_vbox.add_child(reasoning_title)

	# 思考内容标签（reasoning）
	_reasoning_label = Label.new()
	_reasoning_label.text = "暂无思考内容..."
	_reasoning_label.add_theme_font_size_override("font_size", 12)
	_reasoning_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.8))
	_reasoning_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_reasoning_label.custom_minimum_size = Vector2(0, 60)
	_reasoning_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_reasoning_label.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_content_vbox.add_child(_reasoning_label)

	# 分隔线
	var sep2 = HSeparator.new()
	_content_vbox.add_child(sep2)

	# 引导按钮标题
	var guide_title = Label.new()
	guide_title.text = "状态引导"
	guide_title.add_theme_font_size_override("font_size", 12)
	guide_title.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	_content_vbox.add_child(guide_title)

	# 引导按钮网格（2行2列）
	var guide_grid = GridContainer.new()
	guide_grid.columns = 2
	guide_grid.add_theme_constant_override("h_separation", 3)
	guide_grid.add_theme_constant_override("v_separation", 3)

	var guides = [
		{"name": "进食", "key": "eat"},
		{"name": "饮水", "key": "drink"},
		{"name": "采集", "key": "gather"},
		{"name": "探索", "key": "explore"},
	]

	for guide in guides:
		var btn = Button.new()
		btn.text = guide.name
		btn.custom_minimum_size = Vector2(60, 26)
		btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		btn.pressed.connect(_inject_guide.bind(guide.key))
		guide_grid.add_child(btn)

	_content_vbox.add_child(guide_grid)

	# 分隔线
	var sep3 = HSeparator.new()
	_content_vbox.add_child(sep3)

	# 暂停按钮
	var pause_btn = Button.new()
	pause_btn.text = "暂停"
	pause_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	pause_btn.pressed.connect(_toggle_pause)
	_content_vbox.add_child(pause_btn)


func _create_status_bar(label_text: String, color: Color) -> Dictionary:
	var vbox = VBoxContainer.new()
	vbox.custom_minimum_size = Vector2(95, 40)
	vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var hbox = HBoxContainer.new()
	hbox.add_theme_constant_override("separation", 2)

	var label = Label.new()
	label.text = label_text
	label.add_theme_font_size_override("font_size", 12)
	label.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	hbox.add_child(label)

	var value_label = Label.new()
	value_label.text = "100"
	value_label.add_theme_font_size_override("font_size", 12)
	value_label.size_flags_horizontal = Control.SIZE_SHRINK_END
	hbox.add_child(value_label)

	vbox.add_child(hbox)

	var bar = ProgressBar.new()
	bar.min_value = 0
	bar.max_value = 100
	bar.value = 100
	bar.custom_minimum_size = Vector2(75, 10)
	bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	bar.show_percentage = false

	# 自定义进度条颜色
	var fill_style = StyleBoxFlat.new()
	fill_style.bg_color = color
	bar.add_theme_stylebox_override("fill", fill_style)

	var bg_style = StyleBoxFlat.new()
	bg_style.bg_color = Color(0.2, 0.2, 0.2, 0.5)
	bar.add_theme_stylebox_override("background", bg_style)

	vbox.add_child(bar)

	return {"container": vbox, "bar": bar, "label": value_label}


func _on_agent_selected(agent_id: String) -> void:
	_selected_agent_id = agent_id
	visible = not agent_id.is_empty()
	_update_display()


func _on_world_updated(_snapshot: Dictionary) -> void:
	_update_display()


func _update_display() -> void:
	if _selected_agent_id.is_empty():
		visible = false
		return

	var bridge = get_node_or_null("/root/Main/SimulationBridge")
	if not bridge:
		return

	var data = bridge.get_agent_data(_selected_agent_id)
	if data.is_empty():
		return

	# Agent 名称
	var agent_name: String = data.get("name", "Unknown")
	_name_label.text = agent_name

	# HP
	var hp: int = data.get("health", 100)
	var max_hp: int = data.get("max_health", 100)
	_hp_bar.value = hp
	_hp_label.text = "%d" % hp
	_update_bar_color(_hp_bar, hp, Color(0.9, 0.2, 0.2), Color(0.9, 0.8, 0.1), Color(0.9, 0.2, 0.2))

	# 饱食度
	var satiety: int = data.get("satiety", 100)
	_satiety_bar.value = satiety
	_satiety_label.text = "%d" % satiety
	_update_bar_color(_satiety_bar, satiety, Color(0.2, 0.8, 0.2), Color(0.9, 0.8, 0.1), Color(0.9, 0.2, 0.2))

	# 水分度
	var hydration: int = data.get("hydration", 100)
	_hydration_bar.value = hydration
	_hydration_label.text = "%d" % hydration
	_update_bar_color(_hydration_bar, hydration, Color(0.2, 0.5, 0.9), Color(0.9, 0.8, 0.1), Color(0.9, 0.2, 0.2))

	# 背包
	var inventory: Dictionary = data.get("inventory_summary", {})
	if inventory.is_empty():
		_inventory_label.text = "（空）"
	else:
		var items: Array[String] = []
		for resource_name in inventory.keys():
			items.append("%s:%d" % [resource_name, inventory[resource_name]])
		_inventory_label.text = " ".join(items)

	# 动作（current_action）
	var current_action: String = data.get("current_action", "")
	if current_action.is_empty():
		_action_label.text = "等待"
	else:
		_action_label.text = current_action

	# 结果（action_result）- 根据内容设置颜色
	var action_result: String = data.get("action_result", "")
	if action_result.is_empty():
		_result_label.text = "无"
		_result_label.add_theme_color_override("font_color", Color(0.3, 0.8, 0.3))  # 默认绿色
	else:
		_result_label.text = action_result
		# 检查是否包含失败关键词
		if action_result.contains("失败") or action_result.contains("被拒绝"):
			_result_label.add_theme_color_override("font_color", Color(0.9, 0.3, 0.3))  # 红色
		else:
			_result_label.add_theme_color_override("font_color", Color(0.3, 0.8, 0.3))  # 绿色

	# 等级（level）
	var level: int = data.get("level", 1)
	_level_label.text = "Lv.%d" % level

	# 思考内容（reasoning）
	var reasoning: String = data.get("reasoning", "")
	if reasoning.is_empty():
		_reasoning_label.text = "暂无思考内容..."
		_reasoning_label.add_theme_color_override("font_color", Color(0.5, 0.5, 0.6))
	else:
		# 截断过长的 reasoning（保留前300字符）
		# if reasoning.length() > 300:
		# 	reasoning = reasoning.substr(0, 300) + "..."
		_reasoning_label.text = reasoning
		_reasoning_label.add_theme_color_override("font_color", Color(0.75, 0.75, 0.85))

	visible = true


func _update_bar_color(bar: ProgressBar, value: int, high_color: Color, mid_color: Color, low_color: Color) -> void:
	var fill_style = StyleBoxFlat.new()
	if value > 50:
		fill_style.bg_color = high_color
	elif value > 25:
		fill_style.bg_color = mid_color
	else:
		fill_style.bg_color = low_color
	bar.add_theme_stylebox_override("fill", fill_style)


func _inject_guide(key: String) -> void:
	"""注入引导偏好"""
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("/root/Main/SimulationBridge")
		if bridge:
			bridge.inject_preference(_selected_agent_id, key, 0.5, 15)


func _toggle_pause() -> void:
	"""切换暂停状态"""
	var bridge = get_node_or_null("/root/Main/SimulationBridge")
	if bridge:
		if bridge.is_paused:
			bridge.start()
		else:
			bridge.pause()