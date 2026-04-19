# AgentManager - Agent 管理器
# 支持增量 delta 更新 + snapshot 一致性校验
extends Node2D

const AGENT_COLOR = Color(0.2, 0.6, 0.9)
const AGENT_SIZE = 24
const SELECTION_COLOR = Color.YELLOW
const LABEL_FONT_SIZE = 11
const MAX_DELTA_PER_FRAME = 100

var _agent_idle_texture: Texture2D
var _agent_selected_texture: Texture2D
var _agent_nodes: Dictionary = {}
var _selected_agent_id: String = ""
var _pending_deltas: Array = []

# Agent 闪烁效果系统
var _flash_agents: Dictionary = {}  # agent_id -> {"duration": float}
var _effect_time: float = 0.0


func _ready() -> void:
	print("[AgentManager] Agent 管理器初始化（增量渲染模式）")

	# 加载 Agent 纹理（可选）
	_agent_idle_texture = load("res://assets/sprites/agent_idle.png")
	_agent_selected_texture = load("res://assets/sprites/agent_selected.png")
	if _agent_idle_texture:
		print("[AgentManager] Agent 纹理加载成功")
	else:
		print("[AgentManager] Agent 纹理加载失败，使用颜色回退")

	# 连接信号
	var bridge = get_node_or_null("../../SimulationBridge")
	if bridge:
		bridge.world_updated.connect(_on_world_updated)
		bridge.agent_selected.connect(_on_agent_selected)
		# 新增：连接 delta 事件
		if bridge.has_signal("agent_delta"):
			bridge.agent_delta.connect(_on_agent_delta)
			print("[AgentManager] agent_delta 信号已连接")


func _physics_process(_delta: float) -> void:
	# 累加效果时间
	_effect_time += _delta

	# 每帧最多处理 MAX_DELTA_PER_FRAME 个 delta，剩余留给下一帧
	var processed = 0
	while _pending_deltas.size() > 0 and processed < MAX_DELTA_PER_FRAME:
		var delta_data = _pending_deltas.pop_front()
		_process_delta(delta_data)
		processed += 1

	# 更新 Agent 闪烁效果
	_update_flash_effects(_delta)


func _on_agent_delta(delta_data: Dictionary) -> void:
	_pending_deltas.append(delta_data)


func _process_delta(delta_data: Dictionary) -> void:
	var event_type = delta_data.get("type", "")

	match event_type:
		"agent_moved":
			var agent_id = delta_data.get("id", "")
			_update_or_create_agent(agent_id, delta_data)
			# 检查是否触发闪烁（采集动作）
			_check_and_trigger_flash(agent_id)

		"agent_died":
			var agent_id = delta_data.get("id", "")
			_remove_agent(agent_id)

		"agent_spawned":
			var agent_id = delta_data.get("id", "")
			_create_agent_node(agent_id, delta_data)


func _check_and_trigger_flash(agent_id: String) -> void:
	"""检查 Agent 动作并触发闪烁效果"""
	var bridge = get_node_or_null("../../SimulationBridge")
	if not bridge:
		return

	# 获取 Agent 详细数据
	var agent_data = bridge.get_agent_data(agent_id)
	if agent_data.is_empty():
		return

	# 检查当前动作是否包含 Gather
	var current_action = agent_data.get("current_action", "")
	if current_action.contains("Gather"):
		flash_agent(agent_id)


func _update_or_create_agent(agent_id: String, data: Dictionary) -> void:
	var agent_node: Node2D = _agent_nodes.get(agent_id)

	if agent_node == null:
		# Agent 不存在，创建新的
		agent_node = _create_agent_node(agent_id, data)
		add_child(agent_node)
		_agent_nodes[agent_id] = agent_node
	else:
		# 增量更新：只改位置
		var pos: Vector2 = data.get("position", Vector2.ZERO)
		agent_node.position = pos * 16  # 转换为像素坐标

		# 更新健康值（通过 modulate 调整透明度）- 但闪烁期间跳过
		if not _flash_agents.has(agent_id):
			var sprite: Sprite2D = agent_node.get_node_or_null("Sprite")
			if sprite:
				var health_ratio: float = float(data.get("health", 100)) / float(data.get("max_health", 100))
				sprite.modulate.a = health_ratio

		# 更新 Alive 状态
		var is_alive: bool = data.get("is_alive", true)
		agent_node.visible = is_alive


func _on_world_updated(snapshot: Dictionary) -> void:
	var agents: Dictionary = snapshot.get("agents", {})

	# 一致性校验：创建 snapshot 中有但本地缺失的 agent
	for agent_id in agents.keys():
		if not _agent_nodes.has(agent_id):
			var agent_data = agents[agent_id]
			var agent_node = _create_agent_node(agent_id, agent_data)
			add_child(agent_node)
			_agent_nodes[agent_id] = agent_node
			var pos: Vector2 = agent_data.get("position", Vector2.ZERO)
			print("[AgentManager] 一致性修复：创建缺失的 Agent %s 在 (%.0f, %.0f)" % [agent_id, pos.x, pos.y])

	# 一致性校验：删除本地有但 snapshot 中不存在的 agent（幽灵 agent）
	var to_remove = []
	for existing_id in _agent_nodes.keys():
		if not agents.has(existing_id):
			to_remove.append(existing_id)

	for agent_id in to_remove:
		_remove_agent(agent_id)
		print("[AgentManager] 一致性修复：移除幽灵 Agent ", agent_id)


func _update_agent(agent_id: String, data: Dictionary) -> void:
	var agent_node: Node2D = _agent_nodes.get(agent_id)

	if agent_node == null:
		# 创建新的 Agent 节点
		agent_node = _create_agent_node(agent_id, data)
		add_child(agent_node)
		_agent_nodes[agent_id] = agent_node
	else:
		# 更新现有 Agent
		var pos: Vector2 = data.get("position", Vector2.ZERO)
		agent_node.position = pos * 16  # 转换为像素坐标

		# 更新健康值（通过 modulate 调整透明度）
		var sprite: Sprite2D = agent_node.get_node_or_null("Sprite")
		if sprite:
			var health_ratio: float = float(data.get("health", 100)) / float(data.get("max_health", 100))
			sprite.modulate.a = health_ratio

		# 更新标签
		var label: Label = agent_node.get_node_or_null("Label")
		if label:
			label.text = data.get("name", agent_id)

		# 更新 Alive 状态
		var is_alive: bool = data.get("is_alive", true)
		agent_node.visible = is_alive


func _create_agent_node(agent_id: String, data: Dictionary) -> Node2D:
	# 创建容器节点
	var container = Node2D.new()
	container.name = agent_id

	# 半透明黑色背景（增强对比）
	var bg = ColorRect.new()
	bg.name = "Bg"
	bg.custom_minimum_size = Vector2(AGENT_SIZE + 4, AGENT_SIZE + 4)
	bg.position = Vector2(-(AGENT_SIZE + 4) / 2.0, -(AGENT_SIZE + 4) / 2.0)
	bg.color = Color(0, 0, 0, 0.3)
	container.add_child(bg)

	# Agent 主体 - 使用 PNG 纹理
	var sprite = Sprite2D.new()
	sprite.name = "Sprite"
	sprite.texture = _agent_idle_texture
	sprite.centered = true
	sprite.scale = Vector2(AGENT_SIZE / 32.0, AGENT_SIZE / 32.0)
	container.add_child(sprite)

	# 创建标签（带阴影）
	var label = Label.new()
	label.name = "Label"
	label.position = Vector2(-30, -AGENT_SIZE / 2.0 - 14)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.add_theme_font_size_override("font_size", LABEL_FONT_SIZE)
	label.add_theme_color_override("font_shadow_color", Color.BLACK)
	label.add_theme_constant_override("shadow_offset_x", 1)
	label.add_theme_constant_override("shadow_offset_y", 1)
	label.text = data.get("name", agent_id)
	container.add_child(label)

	# 设置初始位置
	var pos: Vector2 = data.get("position", Vector2.ZERO)
	container.position = pos * 16

	return container


func _remove_agent(agent_id: String) -> void:
	var agent_node: Node2D = _agent_nodes.get(agent_id)
	if agent_node:
		agent_node.queue_free()
		_agent_nodes.erase(agent_id)


func _on_agent_selected(agent_id: String) -> void:
	# 清除之前的选择
	if not _selected_agent_id.is_empty():
		_clear_selection()

	_selected_agent_id = agent_id

	# 高亮选中的 Agent
	var agent_node: Node2D = _agent_nodes.get(agent_id)
	if agent_node:
		_highlight_agent(agent_node)


func _clear_selection() -> void:
	var prev_node: Node2D = _agent_nodes.get(_selected_agent_id)
	if prev_node:
		var sprite: Sprite2D = prev_node.get_node_or_null("Sprite")
		if sprite:
			sprite.texture = _agent_idle_texture
	_selected_agent_id = ""


func _highlight_agent(agent_node: Node2D) -> void:
	var sprite: Sprite2D = agent_node.get_node_or_null("Sprite")
	if sprite:
		sprite.texture = _agent_selected_texture


# 点击检测
func _input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var mouse_pos = get_local_mouse_position()

		for agent_id in _agent_nodes.keys():
			var agent_node: Node2D = _agent_nodes[agent_id]
			var distance = agent_node.position.distance_to(mouse_pos)

			if distance < AGENT_SIZE:
				var bridge = get_node_or_null("../../SimulationBridge")
				if bridge:
					bridge.select_agent(agent_id)
				break


# ===== Agent 闪烁效果系统 =====

func flash_agent(agent_id: String, duration: float = 0.3) -> void:
	"""触发 Agent 闪烁效果"""
	_flash_agents[agent_id] = {"duration": duration}


func _update_flash_effects(delta: float) -> void:
	"""更新所有 Agent 的闪烁效果"""
	for agent_id in _flash_agents.keys():
		var flash_data = _flash_agents[agent_id]
		flash_data["duration"] -= delta

		if flash_data["duration"] <= 0:
			# 闪烁结束，恢复正常
			_flash_agents.erase(agent_id)
			if _agent_nodes.has(agent_id):
				var sprite: Sprite2D = _agent_nodes[agent_id].get_node_or_null("Sprite")
				if sprite:
					sprite.modulate.a = 1.0
		else:
			# 闪烁中：透明度脉动（0.4~1.0）
			if _agent_nodes.has(agent_id):
				var sprite: Sprite2D = _agent_nodes[agent_id].get_node_or_null("Sprite")
				if sprite:
					sprite.modulate.a = sin(_effect_time * 8.0) * 0.3 + 0.7
