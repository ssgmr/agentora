# MilestonePanel - 里程碑进度面板
# 显示已达成和未达成的文明里程碑
extends VBoxContainer

# 里程碑定义（与 Rust 端保持一致）
const MILESTONES = {
	"FirstCamp": {"name": "首次建造", "desc": "建造第一个营地", "icon": "🏕"},
	"FirstTrade": {"name": "首次交易", "desc": "完成第一次交易", "icon": "🤝"},
	"FirstFence": {"name": "首次防御", "desc": "建造第一个围栏", "icon": "🚧"},
	"FirstAttack": {"name": "首次战斗", "desc": "发起第一次攻击", "icon": "⚔"},
	"FirstLegacyInteract": {"name": "传承发现", "desc": "首次与遗产互动", "icon": "📜"},
	"CityState": {"name": "城邦时代", "desc": "3+建筑,2+盟友,仓库", "icon": "🏛"},
	"GoldenAge": {"name": "黄金时代", "desc": "达成所有基础里程碑", "icon": "👑"},
}

var _achieved_milestones: Dictionary = {}  # name -> {display_name, tick}
var _milestone_labels: Dictionary = {}  # name -> Label


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
		bridge.world_updated.connect(_on_world_updated)


func _setup_ui() -> void:
	# 标题
	var title = Label.new()
	title.text = "文明里程碑"
	title.add_theme_font_size_override("font_size", 13)
	add_child(title)

	# 创建每个里程碑的状态行
	for milestone_name in MILESTONES.keys():
		var data = MILESTONES[milestone_name]
		var hbox = HBoxContainer.new()
		hbox.add_theme_constant_override("separation", 4)

		# 图标标签
		var icon_label = Label.new()
		icon_label.text = data.icon
		icon_label.custom_minimum_size = Vector2(24, 0)
		icon_label.add_theme_font_size_override("font_size", 12)
		hbox.add_child(icon_label)

		# 名称和状态
		var status_label = Label.new()
		status_label.text = "%s [color=gray]未达成[/color]" % data.name
		status_label.add_theme_font_size_override("font_size", 10)
		status_label.tooltip_text = data.desc
		hbox.add_child(status_label)
		_milestone_labels[milestone_name] = status_label

		add_child(hbox)


func _on_world_updated(snapshot: Dictionary) -> void:
	var milestones: Array = snapshot.get("milestones", [])

	# 更新已达成的里程碑
	for milestone in milestones:
		var name_str: String = milestone.get("name", "")
		var display_name: String = milestone.get("display_name", "")
		var tick: int = milestone.get("achieved_tick", 0)

		if not _achieved_milestones.has(name_str):
			_achieved_milestones[name_str] = {
				"display_name": display_name,
				"tick": tick
			}
			_update_milestone_display(name_str, display_name, tick)
			_show_milestone_notification(name_str, display_name)


func _update_milestone_display(name_str: String, display_name: String, tick: int) -> void:
	var label: Label = _milestone_labels.get(name_str)
	if label:
		var _data = MILESTONES.get(name_str, {"icon": "✓"})
		label.text = "%s [color=green]✓ 已达成[/color] (tick %d)" % [display_name, tick]
		# 高亮颜色
		label.add_theme_color_override("font_color", Color(0.4, 0.9, 0.4))


func _show_milestone_notification(name_str: String, display_name: String) -> void:
	# 通过叙事系统显示通知（如果 NarrativeFeed 可用）
	var narrative = get_node_or_null("../../NarrativeFeed")
	if narrative and narrative.has_method("add_milestone_event"):
		narrative.add_milestone_event(name_str, display_name)