# Main - 主场景脚本
# 负责协调 SimulationBridge 和 UI 组件
extends Node

var name_label: Label
var selected_agent_id: String = ""

@onready var tick_label: Label = $UI/TopBar/TickCounter
@onready var agent_count_label: Label = $UI/TopBar/AgentCount
@onready var world_tick_label: Label = $UI/RightPanel/WorldInfo/TickLabel
@onready var world_agent_count_label: Label = $UI/RightPanel/WorldInfo/AgentCount
@onready var status_label: Label = $UI/RightPanel/AgentDetail/VBoxContent/StatusLabel
@onready var speed_control: OptionButton = $UI/TopBar/SpeedControl


func _ready() -> void:
	# 延迟获取 NameLabel，避免节点树初始化时序问题
	call_deferred("_init_name_label")

	print("[Main] 主场景初始化")

	# 连接 SimulationBridge 信号
	var bridge = get_node_or_null("SimulationBridge")
	if bridge:
		bridge.world_updated.connect(_on_world_updated)
		bridge.agent_selected.connect(_on_agent_selected)
		bridge.narrative_event.connect(_on_narrative_event)
		print("[Main] SimulationBridge 信号已连接")
	else:
		printerr("[Main] 未找到 SimulationBridge!")

	# 初始化速度控制
	_setup_speed_control()

	print("[Main] 主场景就绪")


func _init_name_label() -> void:
	var agent_detail = get_node_or_null("UI/RightPanel/AgentDetail/VBoxContent")
	if agent_detail:
		# NameLabel 可能在场景加载时被跳过，动态创建
		name_label = agent_detail.get_node_or_null("AgentNameLabel")
		if name_label == null:
			name_label = Label.new()
			name_label.name = "AgentNameLabel"
			name_label.text = "Agent 名称"
			name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			# 直接设置锚点布局（顶部居中，不受父容器约束）
			name_label.set_anchors_preset(Control.PRESET_TOP_WIDE)
			name_label.offset_bottom = 24
			agent_detail.add_child(name_label)
			agent_detail.move_child(name_label, 0)
			print("[Main] 动态创建 NameLabel")


func _setup_speed_control() -> void:
	if speed_control:
		speed_control.clear()
		speed_control.add_item("1x 正常", 0)
		speed_control.add_item("2x 加速", 1)
		speed_control.add_item("5x 快速", 2)
		speed_control.add_item("暂停", 3)
		speed_control.item_selected.connect(_on_speed_changed)


func _on_speed_changed(index: int) -> void:
	var bridge = get_node_or_null("SimulationBridge")
	if not bridge:
		return

	match index:
		0:  # 1x
			bridge.set_tick_interval(2.0)
			bridge.start()
		1:  # 2x
			bridge.set_tick_interval(1.0)
			bridge.start()
		2:  # 5x
			bridge.set_tick_interval(0.4)
			bridge.start()
		3:  # 暂停
			bridge.pause()


func _on_world_updated(snapshot: Dictionary) -> void:
	# 更新 UI 显示
	var tick: int = snapshot.get("tick", 0)
	var agents: Dictionary = snapshot.get("agents", {})

	if tick_label:
		tick_label.text = "Tick: %d" % tick
	if world_tick_label:
		world_tick_label.text = "Tick: %d" % tick

	if agent_count_label:
		agent_count_label.text = "Agent: %d" % agents.size()
	if world_agent_count_label:
		world_agent_count_label.text = "Agent 数：%d" % agents.size()

	# 如果没有选中 Agent，自动选第一个存活的
	if selected_agent_id.is_empty():
		for agent_data in agents.values():
			if agent_data.get("is_alive", false):
				selected_agent_id = agent_data.get("id", "")
				# 触发 agent_selected 信号，让 MotivationRadar 等组件响应
				var bridge = get_node_or_null("SimulationBridge")
				if bridge:
					bridge.select_agent(selected_agent_id)
				break

	# 每次世界更新都刷新已选中 Agent 的状态
	var bridge2 = get_node_or_null("SimulationBridge")
	if bridge2 and not selected_agent_id.is_empty():
		var agent_data = bridge2.get_agent_data(selected_agent_id)
		if not agent_data.is_empty():
			_update_agent_detail(agent_data)


func _on_agent_selected(agent_id: String) -> void:
	print("[Main] 选择了 Agent: %s" % agent_id)
	selected_agent_id = agent_id


func _update_agent_detail(data: Dictionary) -> void:
	if name_label:
		name_label.text = data.get("name", "Unknown")

	if status_label:
		var age: int = data.get("age", 0)
		var current_action: String = data.get("current_action", "等待")
		var action_result: String = data.get("action_result", "")
		var is_alive: bool = data.get("is_alive", true)
		var level: int = data.get("level", 1)

		var status_text = "状态：%s  LV%d\n" % [("活动中" if is_alive else "已死亡"), level]
		status_text += "动作：%s\n" % current_action
		if action_result != "":
			status_text += "结果：%s\n" % action_result
		status_text += "年龄：%d" % age
		status_label.text = status_text


func _on_narrative_event(event: Dictionary) -> void:
	# 叙事事件由 NarrativeFeed 组件处理
	print("[Main] 叙事事件：%s" % event.get("description", ""))
