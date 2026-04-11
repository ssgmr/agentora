# MotivationRadar - 动机雷达图
# CanvasItem自定义绘制6维雷达图
extends Control

@export var radar_size: float = 100.0
@export var line_color: Color = Color.WHITE
@export var fill_color: Color = Color(0.2, 0.6, 0.8, 0.5)
@export var label_color: Color = Color.WHITE

var _motivation_values: Array[float] = [0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
var _dimension_names: Array[String] = ["生存", "社交", "认知", "表达", "权力", "传承"]
var _agent_id: String = ""

const RADAR_CENTER_OFFSET: Vector2 = Vector2(50, 50)


func _ready() -> void:
	custom_minimum_size = Vector2(radar_size + 100, radar_size + 60)

	# 连接信号
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge:
		bridge.agent_selected.connect(_on_agent_selected)
		bridge.world_updated.connect(_on_world_updated)


func _draw() -> void:
	var center = Vector2(size.x / 2, size.y / 2)

	# 绘制背景网格
	draw_radar_grid(center)

	# 绘制数据区域
	draw_radar_data(center)

	# 绘制维度标签
	draw_dimension_labels(center)


func draw_radar_grid(center: Vector2) -> void:
	var num_rings: int = 5
	var ring_spacing: float = radar_size / num_rings

	for i in range(num_rings + 1):
		var radius = ring_spacing * i
		draw_circle_outline(center, radius, line_color.lerp(Color.TRANSPARENT, 0.5))

	# 绘制轴线
	for i in range(6):
		var angle = _get_angle_for_dimension(i)
		var end_point = center + Vector2(cos(angle), sin(angle)) * radar_size
		draw_line(center, end_point, line_color.lerp(Color.TRANSPARENT, 0.3), 1.0)


func draw_radar_data(center: Vector2) -> void:
	var points: Array[Vector2] = []

	for i in range(6):
		var value: float = _motivation_values[i]
		var angle = _get_angle_for_dimension(i)
		var radius = value * radar_size
		var point = center + Vector2(cos(angle), sin(angle)) * radius
		points.append(point)

	# 绘制填充区域
	var polygon_points: PackedVector2Array = []
	for p in points:
		polygon_points.append(p)
	draw_colored_polygon(polygon_points, fill_color)

	# 绘制边线和顶点
	for i in range(6):
		var next_i = (i + 1) % 6
		draw_line(points[i], points[next_i], line_color, 2.0)
		draw_circle(points[i], 3.0, line_color)


func draw_dimension_labels(center: Vector2) -> void:
	var font = get_theme_font("font")
	var font_size = get_theme_font_size("font_size")

	for i in range(6):
		var angle = _get_angle_for_dimension(i)
		var label_pos = center + Vector2(cos(angle), sin(angle)) * (radar_size + 15)

		# 调整标签位置避免重叠
		var offset = Vector2.ZERO
		if i == 0:  # 顶部
			offset = Vector2(-15, -10)
		elif i == 3:  # 底部
			offset = Vector2(-15, 5)
		elif angle > PI / 2 and angle < 3 * PI / 2:  # 左侧
			offset = Vector2(-30, -5)
		else:  # 右侧
			offset = Vector2(5, -5)

		draw_string(font, label_pos + offset, _dimension_names[i], HORIZONTAL_ALIGNMENT_CENTER, -1, font_size, label_color)


func draw_circle_outline(center: Vector2, radius: float, color: Color) -> void:
	var points: PackedVector2Array = []
	var segments: int = 64

	for i in range(segments):
		var angle = i * 2 * PI / segments
		points.append(center + Vector2(cos(angle), sin(angle)) * radius)

	draw_polyline(points, color, 1.0)


func _get_angle_for_dimension(index: int) -> float:
	# 从顶部开始，顺时针排列
	return -PI / 2 + index * PI / 3


func set_motivation_values(values: Array[float]) -> void:
	_motivation_values = values
	queue_redraw()


func _on_agent_selected(agent_id: String) -> void:
	_agent_id = agent_id
	_update_from_bridge()


func _on_world_updated(snapshot: Dictionary) -> void:
	if _agent_id.is_empty():
		return
	_update_from_bridge()


func _update_from_bridge() -> void:
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge and not _agent_id.is_empty():
		var data = bridge.get_agent_data(_agent_id)
		var motivation: Array = data.get("motivation", [0.5, 0.5, 0.5, 0.5, 0.5, 0.5])

		# 转换为Array[float]
		var float_values: Array[float] = []
		for v in motivation:
			float_values.append(float(v))

		set_motivation_values(float_values)