# AgentDetailPanel - Agent详情面板
# 显示选中Agent的HP/饱食度/水分度条 + 背包资源列表
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
	var bridge = get_node_or_null("../../../SimulationBridge")
	if bridge:
		bridge.agent_selected.connect(_on_agent_selected)
		bridge.world_updated.connect(_on_world_updated)

	# 初始隐藏
	visible = false


func _setup_ui() -> void:
	# 状态条行（HP + 饱食 + 水分 并排）
	var bars_hbox = HBoxContainer.new()
	bars_hbox.add_theme_constant_override("separation", 4)
	bars_hbox.custom_minimum_size = Vector2(0, 32)

	var hp_result = _create_status_bar("HP", Color(0.9, 0.2, 0.2))
	_hp_bar = hp_result["bar"]
	_hp_label = hp_result["label"]
	bars_hbox.add_child(hp_result["container"])

	var satiety_result = _create_status_bar("饱", Color(0.2, 0.8, 0.2))
	_satiety_bar = satiety_result["bar"]
	_satiety_label = satiety_result["label"]
	bars_hbox.add_child(satiety_result["container"])

	var hydration_result = _create_status_bar("水", Color(0.2, 0.5, 0.9))
	_hydration_bar = hydration_result["bar"]
	_hydration_label = hydration_result["label"]
	bars_hbox.add_child(hydration_result["container"])

	_content_vbox.add_child(bars_hbox)

	# 背包内容
	var inv_hbox = HBoxContainer.new()
	inv_hbox.add_theme_constant_override("separation", 2)
	inv_hbox.custom_minimum_size = Vector2(0, 22)
	inv_hbox.size_flags_vertical = Control.SIZE_SHRINK_CENTER

	var inv_title = Label.new()
	inv_title.text = "背包:"
	inv_title.add_theme_font_size_override("font_size", 10)
	inv_title.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	inv_hbox.add_child(inv_title)

	_inventory_label = Label.new()
	_inventory_label.text = "（空）"
	_inventory_label.add_theme_font_size_override("font_size", 10)
	_inventory_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	inv_hbox.add_child(_inventory_label)

	_content_vbox.add_child(inv_hbox)


func _create_status_bar(label_text: String, color: Color) -> Dictionary:
	var vbox = VBoxContainer.new()
	vbox.custom_minimum_size = Vector2(80, 32)
	vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var hbox = HBoxContainer.new()
	hbox.add_theme_constant_override("separation", 2)

	var label = Label.new()
	label.text = label_text
	label.add_theme_font_size_override("font_size", 9)
	label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	hbox.add_child(label)

	var value_label = Label.new()
	value_label.text = "100"
	value_label.add_theme_font_size_override("font_size", 9)
	value_label.size_flags_horizontal = Control.SIZE_SHRINK_END
	hbox.add_child(value_label)

	vbox.add_child(hbox)

	var bar = ProgressBar.new()
	bar.min_value = 0
	bar.max_value = 100
	bar.value = 100
	bar.custom_minimum_size = Vector2(0, 8)
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

	var bridge = get_node_or_null("../../../SimulationBridge")
	if not bridge:
		return

	var data = bridge.get_agent_data(_selected_agent_id)
	if data.is_empty():
		return

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