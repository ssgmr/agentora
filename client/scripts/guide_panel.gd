# GuidePanel - 玩家引导面板
# 6×HSlider调整动机权重
extends VBoxContainer

var _sliders: Array[HSlider] = []
var _labels: Array[Label] = []
var _dimension_names: Array[String] = ["生存与资源", "社会与关系", "认知与好奇", "表达与创造", "权力与影响", "意义与传承"]
var _selected_agent_id: String = ""


func _ready() -> void:
	_setup_sliders()
	_setup_buttons()

	# 连接信号
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge:
		bridge.agent_selected.connect(_on_agent_selected)


func _setup_sliders() -> void:
	# 标题
	var title = Label.new()
	title.text = "动机权重调整"
	title.add_theme_font_size_override("font_size", 14)
	add_child(title)

	# 创建6个滑块
	for i in range(6):
		var hbox = HBoxContainer.new()

		# 维度名称标签
		var name_label = Label.new()
		name_label.text = _dimension_names[i]
		name_label.custom_minimum_size = Vector2(100, 0)
		hbox.add_child(name_label)
		_labels.append(name_label)

		# 滑块
		var slider = HSlider.new()
		slider.min_value = 0.0
		slider.max_value = 1.0
		slider.step = 0.01
		slider.value = 0.5
		slider.custom_minimum_size = Vector2(150, 0)
		slider.tooltip_text = "拖动调整%s动机权重" % _dimension_names[i]
		slider.value_changed.connect(_on_slider_changed.bind(i))
		hbox.add_child(slider)
		_sliders.append(slider)

		# 当前值显示
		var value_label = Label.new()
		value_label.name = "ValueLabel%d" % i
		value_label.text = "0.50"
		value_label.custom_minimum_size = Vector2(40, 0)
		hbox.add_child(value_label)

		add_child(hbox)


func _setup_buttons() -> void:
	# 分隔线
	var sep = HSeparator.new()
	add_child(sep)

	# 偏好按钮标题
	var pref_title = Label.new()
	pref_title.text = "临时偏好注入"
	pref_title.add_theme_font_size_override("font_size", 12)
	add_child(pref_title)

	# 偏好按钮容器
	var btn_hbox = HBoxContainer.new()

	# 探索偏好
	var explore_btn = Button.new()
	explore_btn.text = "建议探索"
	explore_btn.tooltip_text = "临时增加认知动机30%"
	explore_btn.pressed.connect(_inject_explore_preference)
	btn_hbox.add_child(explore_btn)

	# 交易偏好
	var trade_btn = Button.new()
	trade_btn.text = "建议交易"
	trade_btn.tooltip_text = "临时增加社交动机30%"
	trade_btn.pressed.connect(_inject_trade_preference)
	btn_hbox.add_child(trade_btn)

	# 建造偏好
	var build_btn = Button.new()
	build_btn.text = "建议建造"
	build_btn.tooltip_text = "临时增加表达动机30%"
	build_btn.pressed.connect(_inject_build_preference)
	btn_hbox.add_child(build_btn)

	add_child(btn_hbox)

	# 控制按钮
	var ctrl_hbox = HBoxContainer.new()

	var pause_btn = Button.new()
	pause_btn.text = "暂停"
	pause_btn.pressed.connect(_toggle_pause)
	ctrl_hbox.add_child(pause_btn)

	var reset_btn = Button.new()
	reset_btn.text = "重置"
	reset_btn.tooltip_text = "将所有动机恢复到默认值"
	reset_btn.pressed.connect(_reset_motivations)
	ctrl_hbox.add_child(reset_btn)

	add_child(ctrl_hbox)


func _on_slider_changed(dimension: int, value: float) -> void:
	# 更新值显示
	var value_label: Label = get_node_or_null("HBoxContainer%d/ValueLabel%d" % [dimension, dimension])
	if value_label == null:
		# 查找替代方式
		for child in get_children():
			if child is HBoxContainer:
				var idx = _sliders.find(child.get_child(1))
				if idx == dimension:
					value_label = child.get_child(2)
					break

	if value_label:
		value_label.text = "%.2f" % value

	# 发送到Rust
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("/root/SimulationBridge")
		if bridge:
			bridge.adjust_motivation(_selected_agent_id, dimension, value)


func _on_agent_selected(agent_id: String) -> void:
	_selected_agent_id = agent_id

	# 更新滑块显示当前Agent的动机值
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge:
		var data = bridge.get_agent_data(agent_id)
		var motivation: Array = data.get("motivation", [0.5, 0.5, 0.5, 0.5, 0.5, 0.5])

		for i in range(6):
			_sliders[i].value = motivation[i]


func _inject_explore_preference() -> void:
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("/root/SimulationBridge")
		if bridge:
			bridge.inject_preference(_selected_agent_id, 2, 0.3, 10)  # 认知维度


func _inject_trade_preference() -> void:
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("/root/SimulationBridge")
		if bridge:
			bridge.inject_preference(_selected_agent_id, 1, 0.3, 10)  # 社交维度


func _inject_build_preference() -> void:
	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("/root/SimulationBridge")
		if bridge:
			bridge.inject_preference(_selected_agent_id, 3, 0.3, 10)  # 表达维度


func _toggle_pause() -> void:
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge:
		if bridge.is_paused:
			bridge.start()
		else:
			bridge.pause()


func _reset_motivations() -> void:
	for i in range(6):
		_sliders[i].value = 0.5

	if not _selected_agent_id.is_empty():
		var bridge = get_node_or_null("/root/SimulationBridge")
		if bridge:
			for i in range(6):
				bridge.adjust_motivation(_selected_agent_id, i, 0.5)