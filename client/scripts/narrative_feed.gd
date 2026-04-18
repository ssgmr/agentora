# NarrativeFeed - 叙事流面板
# RichTextLabel滚动显示Agent决策叙事
extends PanelContainer

@export var max_events: int = 100
@export var auto_scroll: bool = true

var _text_box: RichTextLabel
var _scroll_container: ScrollContainer

# 事件颜色编码
const EVENT_COLORS = {
	"move": "#FFFFFF",      # 白色 - 动作
	"gather": "#FFFFFF",
	"wait": "#FFFFFF",
	"trade": "#4CAF50",     # 绿色 - 交易
	"trade_accept": "#4CAF50",
	"talk": "#9E9E9E",      # 灰色 - 对话
	"attack": "#F44336",    # 红色 - 攻击
	"alliance": "#2196F3",  # 蓝色 - 结盟
	"pressure": "#FFC107",  # 黄色 - 压力事件
	"pressure_start": "#FF9800",  # 橙色 - 压力开始
	"pressure_end": "#8BC34A",    # 浅绿 - 压力结束
	"milestone": "#FFD700",  # 金色 - 里程碑
	"legacy": "#9C27B0",    # 紫色 - 遗产
	"death": "#9C27B0",
	"healed": "#4CAF50",    # 绿色 - 治愈（营地效果）
	"survival": "#E91E63",  # 粉色 - 生存警告
}


func _ready() -> void:
	# 给 PanelContainer 加半透明深色背景
	var panel_bg = StyleBoxFlat.new()
	panel_bg.bg_color = Color(0, 0, 0, 0.6)
	add_theme_stylebox_override("panel", panel_bg)

	# 使用 tscn 中预定义的节点
	_scroll_container = get_node_or_null("ScrollContainer")
	_text_box = get_node_or_null("ScrollContainer/EventText")

	# 如果 tscn 节点不存在，回退到动态创建
	if not _scroll_container or not _text_box:
		_setup_ui_fallback()
	else:
		_setup_styling()

	# 连接信号
	# NarrativeFeed 在 UI 下，SimulationBridge 在 Main 下，需要上两层
	var bridge = get_node_or_null("../../SimulationBridge")
	if bridge:
		print("[NarrativeFeed] 找到 bridge，检查信号...")
		print("[NarrativeFeed] 有 narrative_event 信号: ", bridge.has_signal("narrative_event"))
		# 只连接 narrative_event，不连接 world_updated（避免重复）
		bridge.narrative_event.connect(_on_narrative_event)
		print("[NarrativeFeed] narrative_event 信号已连接")
	else:
		printerr("[NarrativeFeed] 未找到 SimulationBridge! 当前路径: ", get_path())


func _setup_styling() -> void:
	_text_box.add_theme_color_override("default_color", Color.WHITE)
	_text_box.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART  # 启用智能换行
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
	_text_box.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART  # 启用智能换行
	_text_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_text_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll_container.add_child(_text_box)

	_setup_styling()


func _on_narrative_event(event: Variant) -> void:
	print("[NarrativeFeed] 收到事件: type=%s agent=%s desc=%s" % [
		event.get("event_type", "?"), event.get("agent_name", "?"), event.get("description", "?")
	])
	add_event(event)


func add_event(event: Dictionary) -> void:
	var tick: int = event.get("tick", 0)
	var agent_name: String = event.get("agent_name", "Unknown")
	var event_type: String = event.get("event_type", "unknown")
	var description: String = event.get("description", "")

	# 获取颜色
	var color: String = EVENT_COLORS.get(event_type, "#FFFFFF")

	# 格式化事件文本（description 已包含 agent 名字）
	var formatted: String = "[color=%s][tick %d] %s[/color]\n" % [color, tick, description]

	# 添加到文本框
	var current_text = _text_box.text
	if current_text.begins_with("[i]等待"):
		current_text = ""

	_text_box.text = current_text + formatted

	# 自动滚动到底部
	if auto_scroll:
		await get_tree().process_frame
		_scroll_container.scroll_vertical = _scroll_container.get_v_scroll_bar().max_value

	# 限制最大事件数
	_limit_events()


func _limit_events() -> void:
	var lines = _text_box.text.split("\n")
	if lines.size() > max_events:
		# 移除最旧的事件
		lines = lines.slice(-max_events)
		_text_box.text = "\n".join(lines)


func clear_log() -> void:
	_text_box.text = "[i]日志已清空[/i]"


# 添加压力事件
func add_pressure_event(description: String) -> void:
	add_event({
		"tick": 0,
		"agent_name": "[系统]",
		"event_type": "pressure",
		"description": description
	})


# 添加遗产事件
func add_legacy_event(legacy_id: String, agent_name: String) -> void:
	add_event({
		"tick": 0,
		"agent_name": agent_name,
		"event_type": "legacy",
		"description": "已死亡，留下遗迹 #%s" % legacy_id
	})


# 添加里程碑事件
func add_milestone_event(name_str: String, display_name: String) -> void:
	var milestone_icons = {
		"FirstCamp": "🏕",
		"FirstTrade": "🤝",
		"FirstFence": "🚧",
		"FirstAttack": "⚔",
		"FirstLegacyInteract": "📜",
		"CityState": "🏛",
		"GoldenAge": "👑",
	}
	var icon = milestone_icons.get(name_str, "🏆")
	add_event({
		"tick": 0,
		"agent_name": "[文明]",
		"event_type": "milestone",
		"description": "%s 达成：【%s】" % [icon, display_name]
	})


# 添加压力开始事件
func add_pressure_start(pressure_type: String, description: String, duration: int) -> void:
	var icons = {
		"drought": "☀️",
		"abundance": "🌾",
		"plague": "☠️",
	}
	var icon = icons.get(pressure_type, "⚠️")
	add_event({
		"tick": 0,
		"agent_name": "[世界]",
		"event_type": "pressure_start",
		"description": "%s %s（持续%d ticks）" % [icon, description, duration]
	})


# 添加压力结束事件
func add_pressure_end(pressure_type: String, description: String) -> void:
	var icons = {
		"drought": "🌧️",
		"abundance": "🍃",
		"plague": "💚",
	}
	var icon = icons.get(pressure_type, "✓")
	add_event({
		"tick": 0,
		"agent_name": "[世界]",
		"event_type": "pressure_end",
		"description": "%s %s 已结束" % [icon, description]
	})


# 添加治愈事件（营地效果）
func add_healed_event(agent_name: String, hp_restored: int) -> void:
	add_event({
		"tick": 0,
		"agent_name": agent_name,
		"event_type": "healed",
		"description": "在营地休息，恢复 %d HP" % hp_restored
	})


# 添加生存警告
func add_survival_warning(agent_name: String, satiety: int, hydration: int) -> void:
	var warnings = []
	if satiety <= 30:
		warnings.append("饥饿")
	if hydration <= 30:
		warnings.append("口渴")
	if satiety == 0 or hydration == 0:
		warnings.append("危急")
	add_event({
		"tick": 0,
		"agent_name": agent_name,
		"event_type": "survival",
		"description": "⚠️ %s（饱食:%d 水分:%d）" % [" ".join(warnings), satiety, hydration]
	})