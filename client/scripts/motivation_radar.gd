# MotivationRadar - 动机雷达图
# CanvasItem自定义绘制6维雷达图
extends Control

@export var radar_size: float = 90.0
@export var line_color: Color = Color.WHITE
@export var fill_color: Color = Color(0.2, 0.6, 0.8, 0.5)
@export var label_color: Color = Color.WHITE
@export var label_font_size: int = 12

var _motivation_values: Array[float] = [0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
var _dimension_names: Array[String] = ["生存", "社交", "认知", "表达", "权力", "传承"]
var _agent_id: String = ""


func _ready() -> void:
	# 连接信号
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge:
		bridge.agent_selected.connect(_on_agent_selected)
		bridge.world_updated.connect(_on_world_updated)


func _draw() -> void:
	var center = Vector2(size.x / 2, size.y / 2)
	# 雷达尺寸自适应控件大小，留出标签空间
	var effective_size = min(size.x, size.y) * 0.42

	# 绘制背景网格
	draw_radar_grid(center, effective_size)

	# 绘制数据区域
	draw_radar_data(center, effective_size)

	# 绘制维度标签
	draw_dimension_labels(center, effective_size)


func draw_radar_grid(center: Vector2, eff_size: float) -> void:
	var num_rings: int = 4
	var ring_spacing: float = eff_size / num_rings

	for i in range(1, num_rings + 1):
		var radius = ring_spacing * i
		draw_circle_outline(center, radius, line_color.lerp(Color.TRANSPARENT, 0.5))

	# 绘制轴线
	for i in range(6):
		var angle = _get_angle_for_dimension(i)
		var end_point = center + Vector2(cos(angle), sin(angle)) * eff_size
		draw_line(center, end_point, line_color.lerp(Color.TRANSPARENT, 0.3), 1.0)


func draw_radar_data(center: Vector2, eff_size: float) -> void:
	var points: Array[Vector2] = []

	for i in range(6):
		var value: float = _motivation_values[i]
		var angle = _get_angle_for_dimension(i)
		var radius = value * eff_size
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


func draw_dimension_labels(center: Vector2, eff_size: float) -> void:
	var font = ThemeDB.fallback_font
	if not font:
		return
	var font_size = label_font_size

	for i in range(6):
		var angle = _get_angle_for_dimension(i)
		var label_dist = eff_size + 18.0
		var label_pos = center + Vector2(cos(angle), sin(angle)) * label_dist

		# 文字宽度
		var text_width = font.get_string_size(_dimension_names[i], HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x

		# 根据角度调整水平对齐和偏移
		var h_align: HorizontalAlignment
		var offset_x: float = 0

		# 判断标签在左侧还是右侧
		var cos_a = cos(angle)
		if cos_a < -0.3:  # 左侧
			h_align = HORIZONTAL_ALIGNMENT_RIGHT
			offset_x = -5
		elif cos_a > 0.3:  # 右侧
			h_align = HORIZONTAL_ALIGNMENT_LEFT
			offset_x = 5
		else:  # 顶部或底部
			h_align = HORIZONTAL_ALIGNMENT_CENTER
			offset_x = -text_width / 2

		draw_string(font, label_pos + Vector2(offset_x, font_size / 3), _dimension_names[i], h_align, -1, font_size, label_color)


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
