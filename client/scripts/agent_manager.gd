# AgentManager - Agent 管理器
# 创建和管理 Agent 节点
extends Node2D

const AGENT_COLOR = Color(0.2, 0.6, 0.9)
const AGENT_SIZE = 24
const SELECTION_COLOR = Color.YELLOW
const LABEL_FONT_SIZE = 11

var _agent_idle_texture: Texture2D
var _agent_selected_texture: Texture2D
var _agent_nodes: Dictionary = {}
var _selected_agent_id: String = ""


func _ready() -> void:
	print("[AgentManager] Agent 管理器初始化")

	# 加载 Agent 纹理（可选）
	_agent_idle_texture = load("res://assets/sprites/agent_idle.png")
	_agent_selected_texture = load("res://assets/sprites/agent_selected.png")
	if _agent_idle_texture:
		print("[AgentManager] Agent 纹理加载成功")
	else:
		print("[AgentManager] Agent 纹理加载失败，使用颜色回退")

	# 连接信号
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge:
		bridge.world_updated.connect(_on_world_updated)
		bridge.agent_selected.connect(_on_agent_selected)


func _on_world_updated(snapshot: Dictionary) -> void:
	var agents: Dictionary = snapshot.get("agents", {})

	# 更新或创建 Agent 节点
	for agent_id in agents.keys():
		var agent_data: Dictionary = agents[agent_id]
		_update_agent(agent_id, agent_data)

	# 删除不存在的 Agent
	for existing_id in _agent_nodes.keys():
		if not agents.has(existing_id):
			_remove_agent(existing_id)


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
	bg.position = Vector2(-(AGENT_SIZE + 4) / 2, -(AGENT_SIZE + 4) / 2)
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
	label.position = Vector2(-30, -AGENT_SIZE / 2 - 14)
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
				var bridge = get_node_or_null("/root/SimulationBridge")
				if bridge:
					bridge.select_agent(agent_id)
				break
