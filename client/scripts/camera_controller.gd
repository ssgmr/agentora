# CameraController - 摄像机控制
# 拖拽平移、滚轮缩放、双击Agent聚焦
extends Camera2D

@export var min_zoom: float = 0.5
@export var max_zoom: float = 3.0
@export var zoom_step: float = 0.1
@export var pan_speed: float = 500.0

var _is_panning: bool = false
var _pan_start_pos: Vector2 = Vector2.ZERO
var _camera_start_pos: Vector2 = Vector2.ZERO


func _ready() -> void:
	zoom = Vector2(1.0, 1.0)
	# 地图中心 (256*16/2 = 2048)
	position = Vector2(2048, 2048)


func _input(event: InputEvent) -> void:
	# 拖拽平移
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_RIGHT or \
		   (event.button_index == MOUSE_BUTTON_LEFT and event.shift_pressed):
			if event.pressed:
				_is_panning = true
				_pan_start_pos = event.position
				_camera_start_pos = position
			else:
				_is_panning = false

	# 滚轮缩放
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			_zoom_in()
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			_zoom_out()

	# 鼠标移动时拖拽
	if event is InputEventMouseMotion and _is_panning:
		var delta = _pan_start_pos - event.position
		position = _camera_start_pos + delta / zoom.x

	# 双击聚焦Agent
	if event is InputEventMouseButton and event.double_click and event.button_index == MOUSE_BUTTON_LEFT:
		_focus_agent_at_position(event.position)


func _zoom_in() -> void:
	var new_zoom = zoom.x + zoom_step
	if new_zoom <= max_zoom:
		zoom = Vector2(new_zoom, new_zoom)


func _zoom_out() -> void:
	var new_zoom = zoom.x - zoom_step
	if new_zoom >= min_zoom:
		zoom = Vector2(new_zoom, new_zoom)


func _focus_agent_at_position(screen_pos: Vector2) -> void:
	# 查找点击位置的Agent
	var world_pos = screen_to_world(screen_pos)

	var agent_manager = get_node_or_null("../WorldView/Agents")
	if agent_manager == null:
		agent_manager = get_node_or_null("../Agents")

	if agent_manager:
		for child in agent_manager.get_children():
			if child is Node2D and child.has_meta("agent_id"):
				var agent_pos = child.position
				var distance = agent_pos.distance_to(world_pos)

				if distance < 50:  # 点击阈值
					focus_on_agent(child.get_meta("agent_id"))
					break


func screen_to_world(screen_pos: Vector2) -> Vector2:
	# 转换屏幕坐标到世界坐标
	return (screen_pos - get_viewport_rect().size / 2) / zoom.x + position


func focus_on_agent(agent_id: String) -> void:
	# 获取Agent位置并移动摄像机
	var bridge = get_node_or_null("../../SimulationBridge")
	if bridge:
		var data = bridge.get_agent_data(agent_id)
		var agent_pos: Vector2 = data.get("position", Vector2.ZERO)

		# 世界坐标（TileMap格子 * 16像素）
		var world_pos = agent_pos * 16

		# 平滑移动到Agent位置
		var tween = create_tween()
		tween.tween_property(self, "position", world_pos, 0.5).set_ease(Tween.EASE_OUT)

		# 通知选择Agent
		bridge.select_agent(agent_id)


# 边界限制（防止摄像机超出地图范围）
func _process(_delta: float) -> void:
	# 256×256地图，每格子16像素
	var map_width = 256 * 16
	var map_height = 256 * 16

	var half_viewport = get_viewport_rect().size / 2 / zoom.x

	# 限制摄像机位置
	position.x = clamp(position.x, half_viewport.x, map_width - half_viewport.x)
	position.y = clamp(position.y, half_viewport.y, map_height - half_viewport.y)