# Main - 主场景脚本
# 负责协调 SimulationBridge 和 UI 组件
extends Node

var selected_agent_id: String = ""
var _map_bounds_set: bool = false  # 标记是否已设置地图边界

@onready var tick_label: Label = $UI/TopBar/TickCounter
@onready var agent_count_label: Label = $UI/TopBar/AgentCount
@onready var speed_control: OptionButton = $UI/TopBar/SpeedControl


func _ready() -> void:
	print("[Main] 主场景初始化")

	# 连接 SimulationBridge 信号到 StateManager（统一状态分发）
	var bridge = BridgeAccessor.get_bridge()
	if bridge:
		# Bridge 信号 → StateManager 处理
		bridge.world_updated.connect(StateManager._on_world_updated)
		bridge.agent_selected.connect(_on_agent_selected)
		bridge.narrative_event.connect(StateManager._on_narrative_event)
		# 连接 agent_delta 信号（增量更新）
		if bridge.has_signal("agent_delta"):
			bridge.agent_delta.connect(StateManager._on_agent_delta)
		print("[Main] SimulationBridge 信号已连接到 StateManager")
	else:
		printerr("[Main] 未找到 SimulationBridge!")

	# 订阅 StateManager 的全局更新信号（用于 UI 更新）
	StateManager.state_updated.connect(_on_state_updated)
	StateManager.milestone_reached.connect(_on_milestone_reached)

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
	var bridge = BridgeAccessor.get_bridge()
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


func _on_state_updated(snapshot: Dictionary) -> void:
	# 从 StateManager 获取当前状态更新 UI 显示
	var tick: int = StateManager.get_current_tick()
	var agents: Dictionary = StateManager.get_all_agents()

	if tick_label:
		tick_label.text = "Tick: %d" % tick

	if agent_count_label:
		agent_count_label.text = "Agent: %d" % agents.size()

	# 设置相机边界（从 StateManager 获取地图尺寸）
	if not _map_bounds_set:
		var map_size: Vector2i = StateManager.get_map_size()
		if map_size.x > 0 and map_size.y > 0:
			var camera = get_node_or_null("Camera2D")
			if camera and camera.has_method("set_map_bounds"):
				camera.set_map_bounds(map_size.x, map_size.y, 16)
				_map_bounds_set = true
				print("[Main] 已设置相机边界: %dx%d" % [map_size.x, map_size.y])

	# 如果没有选中 Agent，自动选第一个存活的
	if selected_agent_id.is_empty():
		for agent_id in agents.keys():
			var agent_data = agents[agent_id]
			if agent_data.get("is_alive", false):
				selected_agent_id = agent_data.get("id", "")
				# 触发 agent_selected 信号
				var bridge = BridgeAccessor.get_bridge()
				if bridge:
					bridge.select_agent(selected_agent_id)
				break


func _on_milestone_reached(name: String, display_name: String, tick: int) -> void:
	print("[Main] 里程碑达成: %s (%s) at tick %d" % [name, display_name, tick])


func _on_agent_selected(agent_id: String) -> void:
	print("[Main] 选择了 Agent: %s" % agent_id)
	selected_agent_id = agent_id


func _on_narrative_event(event: Dictionary) -> void:
	# 叙事事件由 NarrativeFeed 组件处理
	print("[Main] 叙事事件：%s" % event.get("description", ""))