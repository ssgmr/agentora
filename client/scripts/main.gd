# Main - 主场景脚本
# 负责协调 SimulationBridge 和 UI 组件
extends Node

@onready var tick_label: Label = $TopBar/TickCounter
@onready var agent_count_label: Label = $TopBar/AgentCount
@onready var world_tick_label: Label = $RightPanel/WorldInfo/TickLabel
@onready var world_agent_count_label: Label = $RightPanel/WorldInfo/AgentCount
@onready var status_label: Label = $RightPanel/AgentDetail/StatusLabel
@onready var name_label: Label = $RightPanel/AgentDetail/NameLabel
@onready var speed_control: OptionButton = $TopBar/SpeedControl


func _ready() -> void:
	print("[Main] 主场景初始化")

	# 连接 SimulationBridge 信号
	var bridge = get_node_or_null("/root/SimulationBridge")
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


func _setup_speed_control() -> void:
	if speed_control:
		speed_control.clear()
		speed_control.add_item("1x 正常", 0)
		speed_control.add_item("2x 加速", 1)
		speed_control.add_item("5x 快速", 2)
		speed_control.add_item("暂停", 3)
		speed_control.item_selected.connect(_on_speed_changed)


func _on_speed_changed(index: int) -> void:
	var bridge = get_node_or_null("/root/SimulationBridge")
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

	# 如果已选择 Agent，更新其状态
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge and not bridge.selected_agent_id.is_empty():
		var agent_data = bridge.get_agent_data(bridge.selected_agent_id)
		if not agent_data.is_empty():
			_update_agent_detail(agent_data)


func _on_agent_selected(agent_id: String) -> void:
	print("[Main] 选择了 Agent: %s" % agent_id)
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge:
		var agent_data = bridge.get_agent_data(agent_id)
		if not agent_data.is_empty():
			_update_agent_detail(agent_data)


func _update_agent_detail(data: Dictionary) -> void:
	if name_label:
		name_label.text = data.get("name", "Unknown")

	if status_label:
		var health: int = data.get("health", 100)
		var max_health: int = data.get("max_health", 100)
		var age: int = data.get("age", 0)
		var current_action: String = data.get("current_action", "等待")
		var is_alive: bool = data.get("is_alive", true)

		var status_text = "状态：%s\n" % ("活动中" if is_alive else "已死亡")
		status_text += "动作：%s\n" % current_action
		status_text += "健康：%d/%d\n" % [health, max_health]
		status_text += "年龄：%d" % age
		status_label.text = status_text


func _on_narrative_event(event: Dictionary) -> void:
	# 叙事事件由 NarrativeFeed 组件处理
	print("[Main] 叙事事件：%s" % event.get("description", ""))
