# GuidePanel - 玩家引导面板
# 6×HSlider调整动机权重
extends VBoxContainer

var _sliders: Array[HSlider] = []
var _value_labels: Array[Label] = []
var _dimension_names: Array[String] = ["生存", "社交", "认知", "表达", "权力", "传承"]
var _selected_agent_id: String = ""


func _ready() -> void:
	_setup_sliders()
	_setup_buttons()

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
					# 通知 bridge 同步 selected_agent_id
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
					# 临时断开信号，避免代码设置滑块时触发 adjust_motivation
					for i in range(6):
						_sliders[i].value_changed.disconnect(_on_slider_changed.bind(i))

					for i in range(6):
						var v = clamp(float(motivation[i]), 0.0, 1.0)
						_sliders[i].value = v
						if i < _value_labels.size():
							_value_labels[i].text = "%.2f" % v

					# 重新连接信号
					for i in range(6):
						_sliders[i].value_changed.connect(_on_slider_changed.bind(i))


func _setup_sliders() -> void:
	# 标题
	var title = Label.new()
	title.text = "动机权重"
	title.add_theme_font_size_override("font_size", 13)
	add_child(title)

	# 创建6个滑块
	for i in range(6):
		var hbox = HBoxContainer.new()
		hbox.add_theme_constant_override("separation", 2)

		# 维度名称标签
		var name_label = Label.new()
		name_label.text = _dimension_names[i]
		name_label.custom_minimum_size = Vector2(32, 0)
		name_label.add_theme_font_size_override("font_size", 10)
		hbox.add_child(name_label)

		# 滑块
		var slider = HSlider.new()
		slider.min_value = 0.0
		slider.max_value = 1.0
		slider.step = 0.01
		slider.value = 0.5
		slider.custom_minimum_size = Vector2(100, 0)
		slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		slider.value_changed.connect(_on_slider_changed.bind(i))
		hbox.add_child(slider)
		_sliders.append(slider)

		# 当前值显示
		var value_label = Label.new()
		value_label.name = "ValueLabel"
		value_label.text = "0.50"
		value_label.custom_minimum_size = Vector2(32, 0)
		value_label.add_theme_font_size_override("font_size", 11)
		hbox.add_child(value_label)
		_value_labels.append(value_label)

		add_child(hbox)


func _setup_buttons() -> void:
	# 分隔线
	var sep = HSeparator.new()
	add_child(sep)

	# 偏好按钮标题
	var pref_title = Label.new()
	pref_title.text = "临时偏好"
	pref_title.add_theme_font_size_override("font_size", 11)
	add_child(pref_title)

	# 偏好按钮容器
	var btn_hbox = HBoxContainer.new()
	btn_hbox.add_theme_constant_override("separation", 4)

	var explore_btn = Button.new()
	explore_btn.text = "探索"
	explore_btn.tooltip_text = "临时增加认知动机30%"
	explore_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	explore_btn.pressed.connect(_inject_explore_preference)
	btn_hbox.add_child(explore_btn)

	var trade_btn = Button.new()
	trade_btn.text = "交易"
	trade_btn.tooltip_text = "临时增加社交动机30%"
	trade_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	trade_btn.pressed.connect(_inject_trade_preference)
	btn_hbox.add_child(trade_btn)

	var build_btn = Button.new()
	build_btn.text = "建造"
	build_btn.tooltip_text = "临时增加表达动机30%"
	build_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	build_btn.pressed.connect(_inject_build_preference)
	btn_hbox.add_child(build_btn)

	add_child(btn_hbox)

	# 控制按钮
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


func _on_slider_changed(dimension: int, value: float) -> void:
	# 更新值显示
	if dimension < _value_labels.size():
		_value_labels[dimension].text = "%.2f" % clamp(value, 0.0, 1.0)

	# 发送到Rust
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			bridge.adjust_motivation(_selected_agent_id, dimension, clamp(value, 0.0, 1.0))


func _on_agent_selected(agent_id: String) -> void:
	_selected_agent_id = agent_id

	# 更新滑块显示当前Agent的动机值
	var bridge = get_node_or_null("../../../../SimulationBridge")
	if bridge:
		var data = bridge.get_agent_data(agent_id)
		var motivation: Array = data.get("motivation", [0.5, 0.5, 0.5, 0.5, 0.5, 0.5])
		printerr("[GuidePanel] motivation: %s" % str(motivation))

		# 临时断开信号，避免代码设置滑块时触发 adjust_motivation
		for i in range(6):
			_sliders[i].value_changed.disconnect(_on_slider_changed.bind(i))

		for i in range(6):
			var v = clamp(float(motivation[i]), 0.0, 1.0)
			_sliders[i].value = v
			if i < _value_labels.size():
				_value_labels[i].text = "%.2f" % v

		# 重新连接信号
		for i in range(6):
			_sliders[i].value_changed.connect(_on_slider_changed.bind(i))


func _inject_explore_preference() -> void:
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			bridge.inject_preference(_selected_agent_id, 2, 0.3, 10)  # 认知维度


func _inject_trade_preference() -> void:
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			bridge.inject_preference(_selected_agent_id, 1, 0.3, 10)  # 社交维度


func _inject_build_preference() -> void:
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			bridge.inject_preference(_selected_agent_id, 3, 0.3, 10)  # 表达维度


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
			_value_labels[i].text = "0.50"

	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("../../../../SimulationBridge")
		if bridge:
			for i in range(6):
				bridge.adjust_motivation(_selected_agent_id, i, 0.5)
