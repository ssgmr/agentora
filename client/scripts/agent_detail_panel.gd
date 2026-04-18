# AgentDetailPanel - Agent详情面板
# 显示选中Agent的HP/饱食度/水分度条 + 背包资源列表
extends VBoxContainer

var _selected_agent_id: String = ""
var _hp_bar: ProgressBar = null
var _satiety_bar: ProgressBar = null
var _hydration_bar: ProgressBar = null
var _hp_label: Label = null
var _satiety_label: Label = null
var _hydration_label: Label = null
var _inventory_label: Label = null
var _name_label: Label = null


func _ready() -> void:
	# 面板背景
	var panel_bg = StyleBoxFlat.new()
	panel_bg.bg_color = Color(0, 0, 0, 0.7)
	panel_bg.content_margin_left = 8
	panel_bg.content_margin_right = 8
	panel_bg.content_margin_top = 6
	panel_bg.content_margin_bottom = 6
	add_theme_stylebox_override("panel", panel_bg)

	_setup_ui()

	# 连接信号
	var bridge = get_node_or_null("../../../SimulationBridge")
	if bridge:
		bridge.agent_selected.connect(_on_agent_selected)
		bridge.world_updated.connect(_on_world_updated)

	# 初始隐藏
	visible = false


func _setup_ui() -> void:
	# Agent名称
	_name_label = Label.new()
	_name_label.text = ""
	_name_label.add_theme_font_size_override("font_size", 13)
	add_child(_name_label)

	# HP条
	var hp_result = _create_status_bar("HP", Color(0.9, 0.2, 0.2))
	_hp_bar = hp_result["bar"]
	_hp_label = hp_result["label"]
	add_child(hp_result["container"])

	# 饱食度条
	var satiety_result = _create_status_bar("饱食", Color(0.2, 0.8, 0.2))
	_satiety_bar = satiety_result["bar"]
	_satiety_label = satiety_result["label"]
	add_child(satiety_result["container"])

	# 水分度条
	var hydration_result = _create_status_bar("水分", Color(0.2, 0.5, 0.9))
	_hydration_bar = hydration_result["bar"]
	_hydration_label = hydration_result["label"]
	add_child(hydration_result["container"])

	# 分隔线
	var sep = HSeparator.new()
	add_child(sep)

	# 背包标题
	var inv_title = Label.new()
	inv_title.text = "背包"
	inv_title.add_theme_font_size_override("font_size", 11)
	add_child(inv_title)

	# 背包内容
	_inventory_label = Label.new()
	_inventory_label.text = "（空）"
	_inventory_label.add_theme_font_size_override("font_size", 10)
	_inventory_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(_inventory_label)


func _create_status_bar(label_text: String, color: Color) -> Dictionary:
	var hbox = HBoxContainer.new()
	hbox.add_theme_constant_override("separation", 4)

	var label = Label.new()
	label.text = label_text
	label.custom_minimum_size = Vector2(28, 0)
	label.add_theme_font_size_override("font_size", 10)
	hbox.add_child(label)

	var bar = ProgressBar.new()
	bar.min_value = 0
	bar.max_value = 100
	bar.value = 100
	bar.custom_minimum_size = Vector2(80, 12)
	bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	bar.show_percentage = false

	# 自定义进度条颜色
	var fill_style = StyleBoxFlat.new()
	fill_style.bg_color = color
	bar.add_theme_stylebox_override("fill", fill_style)

	var bg_style = StyleBoxFlat.new()
	bg_style.bg_color = Color(0.2, 0.2, 0.2, 0.5)
	bar.add_theme_stylebox_override("background", bg_style)

	hbox.add_child(bar)

	var value_label = Label.new()
	value_label.text = "100/100"
	value_label.custom_minimum_size = Vector2(52, 0)
	value_label.add_theme_font_size_override("font_size", 10)
	hbox.add_child(value_label)

	return {"container": hbox, "bar": bar, "label": value_label}


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

	# 名称
	_name_label.text = data.get("name", _selected_agent_id)

	# HP
	var hp: int = data.get("health", 100)
	var max_hp: int = data.get("max_health", 100)
	_hp_bar.value = hp
	_hp_label.text = "%d/%d" % [hp, max_hp]

	# 饱食度
	var satiety: int = data.get("satiety", 100)
	_satiety_bar.value = satiety
	_satiety_label.text = "%d/100" % satiety
	# 颜色变化：绿>黄>红
	_update_bar_color(_satiety_bar, satiety, Color(0.2, 0.8, 0.2), Color(0.9, 0.8, 0.1), Color(0.9, 0.2, 0.2))

	# 水分度
	var hydration: int = data.get("hydration", 100)
	_hydration_bar.value = hydration
	_hydration_label.text = "%d/100" % hydration
	_update_bar_color(_hydration_bar, hydration, Color(0.2, 0.5, 0.9), Color(0.9, 0.8, 0.1), Color(0.9, 0.2, 0.2))

	# 背包
	var inventory: Dictionary = data.get("inventory_summary", {})
	if inventory.is_empty():
		_inventory_label.text = "（空）"
	else:
		var items: Array[String] = []
		for resource_name in inventory.keys():
			items.append("%s: %d" % [resource_name, inventory[resource_name]])
		_inventory_label.text = "  ".join(items)

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