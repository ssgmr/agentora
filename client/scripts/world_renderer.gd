# WorldRenderer - 世界渲染器
# 使用 CanvasItem 直接绘制地形
extends Node2D

# 地形颜色配置
const TERRAIN_COLORS = {
	"plains": Color(0.3, 0.6, 0.2),    # 绿色草地
	"forest": Color(0.1, 0.4, 0.1),    # 深绿森林
	"mountain": Color(0.5, 0.5, 0.5),  # 灰色山脉
	"water": Color(0.2, 0.4, 0.8),     # 蓝色水域
	"desert": Color(0.8, 0.7, 0.3)     # 黄色沙漠
}

var _tile_size: int = 16
var _map_size: int = 256
var _map_data: Dictionary = {}
var _visible_rect: Rect2 = Rect2(0, 0, 1280, 720)
var _needs_redraw: bool = true


func _ready() -> void:
	print("[WorldRenderer] 世界渲染器初始化")
	set_process(true)

	# 连接信号
	var bridge = get_node_or_null("/root/SimulationBridge")
	if bridge:
		bridge.world_updated.connect(_on_world_updated)

	# 生成地图数据
	_generate_map_data()
	print("[WorldRenderer] 地图生成完成：%d 个单元格" % _map_data.size())


func _generate_map_data() -> void:
	for x in range(_map_size):
		for y in range(_map_size):
			var terrain = _get_terrain_for_position(x, y)
			_map_data["%d_%d" % [x, y]] = terrain


func _get_terrain_for_position(x: int, y: int) -> String:
	var noise = _simple_noise(x * 0.05, y * 0.05)

	if noise < 0.3:
		return "water"
	elif noise < 0.5:
		return "plains"
	elif noise < 0.7:
		return "forest"
	elif noise < 0.85:
		return "mountain"
	else:
		return "desert"


func _simple_noise(x: float, y: float) -> float:
	var value = sin(x * 1.5) * cos(y * 1.5) * 0.5 + 0.5
	return clampf(value, 0.0, 1.0)


func _draw() -> void:
	# 获取摄像机位置
	var camera = get_node_or_null("../Camera2D")
	if camera == null:
		return

	var camera_pos = camera.position
	var zoom = camera.zoom
	var viewport_size = get_viewport().get_visible_rect().size

	# 计算可见范围（世界坐标）
	var start_x = int((camera_pos.x - viewport_size.x / 2 / zoom.x) / _tile_size) - 1
	var start_y = int((camera_pos.y - viewport_size.y / 2 / zoom.y) / _tile_size) - 1
	var end_x = int((camera_pos.x + viewport_size.x / 2 / zoom.x) / _tile_size) + 1
	var end_y = int((camera_pos.y + viewport_size.y / 2 / zoom.y) / _tile_size) + 1

	# 限制在地图范围内
	start_x = maxi(start_x, 0)
	start_y = maxi(start_y, 0)
	end_x = mini(end_x, _map_size - 1)
	end_y = mini(end_y, _map_size - 1)

	# 绘制可见区域内的地形
	for x in range(start_x, end_x + 1):
		for y in range(start_y, end_y + 1):
			var key = "%d_%d" % [x, y]
			var terrain: String = _map_data.get(key, "plains")
			var color: Color = TERRAIN_COLORS.get(terrain, Color.WHITE)

			var pos_x = x * _tile_size
			var pos_y = y * _tile_size

			# 绘制地形矩形
			draw_rect(Rect2(pos_x, pos_y, _tile_size, _tile_size), color)


func _process(_delta: float) -> void:
	# 每帧重绘
	queue_redraw()


func _on_world_updated(snapshot: Dictionary) -> void:
	# 地图变化时重绘
	_needs_redraw = true
	queue_redraw()
