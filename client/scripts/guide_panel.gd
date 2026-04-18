# GuidePanel - 玩家引导面板
# 6个预设倾向按钮 + 可折叠高级滑块面板
extends VBoxContainer

var _sliders: Array[HSlider] = []
var _value_labels: Array[Label] = []
var _dimension_names: Array[String] = ["生存", "社交", "认知", "表达", "权力", "传承"]
var _selected_agent_id: String = ""
var _advanced_visible: bool = false
var _advanced_container: VBoxContainer = null
var _toggle_btn: Button = null


func _ready() -> void:
	_setup_preset_buttons()
	_setup_advanced_toggle()
	_setup_advanced_sliders()
	_setup_control_buttons()

	var bridge = get_node_or_null("../../../../SimulationBridge")
	if bridge:
		bridge.agent_selected.connect(_on_agent_selected)
		bridge.world_updated.connect(_on_world_updated)


func _on_world_updated(snapshot: Dictionary) -> void:
	if _selected_agent_id.is_empty():
		var agents = snapshot.get("agents", {})
		if not agents.is_empty():
			for agent_data in agents.values():
				if agent_data.get("is_alive", false):
					_selected_agent_id = agent_data.get("id", "")
					var bridge = get_node_or_null("../../../../SimulationBridge")
					if bridge:
						bridge.select_agent(_selected_agent_id)
					break

	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			var data = bridge.get_agent_data(_selected_agent_id)
			if not data.is_empty():
				var motivation: Array = data.get("motivation", [])
				if not motivation.is_empty():
					for i in range(6):
						_sliders[i].value_changed.disconnect(_on_slider_changed.bind(i))

					for i in range(6):
						var v = clamp(float(motivation[i]), 0.0, 1.0)
						_sliders[i].value = v
						if i < _value_labels.size():
							_value_labels[i].text = "%d%%" % int(v * 100)

					for i in range(6):
						_sliders[i].value_changed.connect(_on_slider_changed.bind(i))


func _setup_preset_buttons() -> void:
	# 标题
	var title = Label.new()
	title.text = "引导面板"
	title.add_theme_font_size_override("font_size", 14)
	add_child(title)

	# 6个预设按钮（2行3列）
	var grid = GridContainer.new()
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 4)
	grid.add_theme_constant_override("v_separation", 4)

	var presets = [
		{"name": "生存", "dim": 0, "tooltip": "增加生存动机30%"},
		{"name": "社交", "dim": 1, "tooltip": "增加社交动机30%"},
		{"name": "探索", "dim": 2, "tooltip": "增加认知动机30%"},
		{"name": "创造", "dim": 3, "tooltip": "增加表达动机30%"},
		{"name": "征服", "dim": 4, "tooltip": "增加权力动机30%"},
		{"name": "传承", "dim": 5, "tooltip": "增加传承动机30%"},
	]

	for preset in presets:
		var btn = Button.new()
		btn.text = preset.name
		btn.tooltip_text = preset.tooltip
		btn.custom_minimum_size = Vector2(60, 28)
		btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		btn.pressed.connect(_inject_preset.bind(preset.dim, 0.3))
		grid.add_child(btn)

	add_child(grid)


func _setup_advanced_toggle() -> void:
	var sep = HSeparator.new()
	add_child(sep)

	_toggle_btn = Button.new()
	_toggle_btn.text = "▶ 高级自定义"
	_toggle_btn.alignment = HORIZONTAL_ALIGNMENT_LEFT
	_toggle_btn.pressed.connect(_toggle_advanced)
	add_child(_toggle_btn)


func _setup_advanced_sliders() -> void:
	_advanced_container = VBoxContainer.new()
	_advanced_container.visible = false
	add_child(_advanced_container)

	# 创建6个滑块（0%-100%范围，与动机值一致）
	for i in range(6):
		var hbox = HBoxContainer.new()
		hbox.add_theme_constant_override("separation", 2)

		var name_label = Label.new()
		name_label.text = _dimension_names[i]
		name_label.custom_minimum_size = Vector2(32, 0)
		name_label.add_theme_font_size_override("font_size", 10)
		hbox.add_child(name_label)

		var slider = HSlider.new()
		slider.min_value = 0.0
		slider.max_value = 1.0
		slider.step = 0.01
		slider.value = 0.5
		slider.custom_minimum_size = Vector2(80, 0)
		slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		slider.value_changed.connect(_on_slider_changed.bind(i))
		hbox.add_child(slider)
		_sliders.append(slider)

		var value_label = Label.new()
		value_label.text = "50%"
		value_label.custom_minimum_size = Vector2(28, 0)
		value_label.add_theme_font_size_override("font_size", 10)
		hbox.add_child(value_label)
		_value_labels.append(value_label)

		_advanced_container.add_child(hbox)


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

	var reset_btn = Button.new()
	reset_btn.text = "重置"
	reset_btn.tooltip_text = "将所有动机恢复到默认值"
	reset_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	reset_btn.pressed.connect(_reset_motivations)
	ctrl_hbox.add_child(reset_btn)

	add_child(ctrl_hbox)


func _toggle_advanced() -> void:
	_advanced_visible = not _advanced_visible
	_advanced_container.visible = _advanced_visible
	if _toggle_btn:
		_toggle_btn.text = ("▼ 高级自定义" if _advanced_visible else "▶ 高级自定义")


func _on_slider_changed(dimension: int, value: float) -> void:
	if dimension < _value_labels.size():
		_value_labels[dimension].text = "%d%%" % int(clamp(value, 0.0, 1.0) * 100)

	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			bridge.adjust_motivation(_selected_agent_id, dimension, clamp(value, 0.0, 1.0))


func _on_agent_selected(agent_id: String) -> void:
	_selected_agent_id = agent_id

	var bridge = get_node_or_null("../../../../SimulationBridge")
	if bridge:
		var data = bridge.get_agent_data(agent_id)
		var motivation: Array = data.get("motivation", [0.5, 0.5, 0.5, 0.5, 0.5, 0.5])

		for i in range(6):
			_sliders[i].value_changed.disconnect(_on_slider_changed.bind(i))

		for i in range(6):
			var v = clamp(float(motivation[i]), 0.0, 1.0)
			_sliders[i].value = v
			if i < _value_labels.size():
				_value_labels[i].text = "%d%%" % int(v * 100)

		for i in range(6):
			_sliders[i].value_changed.connect(_on_slider_changed.bind(i))


func _inject_preset(dimension: int, boost: float) -> void:
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			bridge.inject_preference(_selected_agent_id, dimension, boost, 15)


func _toggle_pause() -> void:
	var bridge = get_node_or_null("../../../../SimulationBridge")
	if bridge:
		if bridge.is_paused:
			bridge.start()
		else:
			bridge.pause()


func _reset_motivations() -> void:
	for i in range(6):
		_sliders[i].value = 0.5
		if i < _value_labels.size():
			_value_labels[i].text = "50%"

	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			for i in range(6):
				bridge.adjust_motivation(_selected_agent_id, i, 0.5)
