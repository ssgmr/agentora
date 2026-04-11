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
	"legacy": "#9C27B0",    # 紫色 - 遗产
	"death": "#9C27B0"
}


func _ready() -> void:
	_setup_ui()

	# 连接信号
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge:
		bridge.narrative_event.connect(_on_narrative_event)
		bridge.world_updated.connect(_on_world_updated)


func _setup_ui() -> void:
	_scroll_container = ScrollContainer.new()
	_scroll_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	add_child(_scroll_container)

	_text_box = RichTextLabel.new()
	_text_box.bbcode_enabled = true
	_text_box.fit_content = true
	_text_box.scroll_active = false  # 使用外部ScrollContainer
	_text_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_scroll_container.add_child(_text_box)

	# 初始提示
	_text_box.text = "[i]等待模拟开始...[/i]"


func _on_narrative_event(event: Dictionary) -> void:
	add_event(event)


func _on_world_updated(snapshot: Dictionary) -> void:
	# 处理WorldSnapshot中的events列表
	var events: Array = snapshot.get("events", [])
	for event in events:
		add_event(event)


func add_event(event: Dictionary) -> void:
	var tick: int = event.get("tick", 0)
	var agent_name: String = event.get("agent_name", "Unknown")
	var event_type: String = event.get("event_type", "unknown")
	var description: String = event.get("description", "")

	# 获取颜色
	var color: String = EVENT_COLORS.get(event_type, "#FFFFFF")

	# 格式化事件文本
	var formatted: String = "[color=%s][tick %d] %s: %s[/color]\n" % [color, tick, agent_name, description]

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