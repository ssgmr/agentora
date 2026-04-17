# WorldRenderer - 世界渲染器
# 使用 CanvasItem 直接绘制地形
extends Node2D

# 地形纹理配置
var _terrain_textures: Dictionary = {}

# 建筑结构字典 (position_key -> {type, sprite})
var _structures: Dictionary = {}

# 资源字典 (position_key -> {type, amount})
var _resources: Dictionary = {}

# 建筑纹理配置
var _structure_textures: Dictionary = {}

# 资源纹理配置
var _resource_textures: Dictionary = {}

# 资源颜色配置（回退用）
const RESOURCE_COLORS = {
	"food": Color(0.9, 0.6, 0.2),     # 橙色食物
	"water": Color(0.3, 0.6, 0.9),    # 蓝色水源
	"wood": Color(0.6, 0.4, 0.2),     # 棕色木材
	"stone": Color(0.7, 0.7, 0.7),    # 灰色石材
	"iron": Color(0.5, 0.5, 0.6),     # 深灰色铁矿
}

# 建筑 Sprite 节点容器（用于动态添加/删除）
var _structure_sprites: Node2D = null

# 地形颜色配置（回退用）
const TERRAIN_COLORS = {
	"plains": Color(0.3, 0.6, 0.2),    # 绿色草地
	"forest": Color(0.1, 0.4, 0.1),    # 深绿森林
	"mountain": Color(0.5, 0.5, 0.5),  # 灰色山脉
	"water": Color(0.2, 0.4, 0.8),     # 蓝色水域
	"desert": Color(0.8, 0.7, 0.3)     # 黄色沙漠
}

# 建筑类型到纹理的映射
const STRUCTURE_TEXTURE_MAP = {
	"Camp": "res://assets/textures/structure_camp.png",
	"Campfire": "res://assets/textures/structure_campfire.png",
	"Warehouse": "res://assets/textures/structure_warehouse.png",
	"Fortress": "res://assets/textures/structure_fortress.png",
	"WatchTower": "res://assets/textures/structure_watchtower.png",
	"Fence": "res://assets/textures/structure_wall.png",
	"Wall": "res://assets/textures/structure_wall.png",
}

var _tile_size: int = 16
var _map_size: int = 256
var _map_data: Dictionary = {}
var _visible_rect: Rect2 = Rect2(0, 0, 1280, 720)
var _needs_redraw: bool = true

# 建筑效果动画状态
var _effect_time: float = 0.0
var _active_effects: Dictionary = {}  # pos_key -> effect_type


func _ready() -> void:
	print("[WorldRenderer] 世界渲染器初始化")
	set_process(true)

	# 加载地形纹理
	_terrain_textures["plains"] = load("res://assets/textures/terrain_plains.png")
	_terrain_textures["forest"] = load("res://assets/textures/terrain_forest.png")
	_terrain_textures["mountain"] = load("res://assets/textures/terrain_mountain.png")
	_terrain_textures["water"] = load("res://assets/textures/terrain_water.png")
	_terrain_textures["desert"] = load("res://assets/textures/terrain_desert.png")

	# 加载建筑结构纹理
	_load_structure_textures()

	# 加载资源纹理
	_load_resource_textures()

	# 创建建筑 Sprite 容器
	_structure_sprites = Node2D.new()
	_structure_sprites.name = "StructureSprites"
	add_child(_structure_sprites)

	# 连接信号
	var bridge = get_node_or_null("../../SimulationBridge")
	if bridge:
		bridge.world_updated.connect(_on_world_updated)
		# 连接 delta 信号（Tier 2）
		if bridge.has_signal("agent_delta"):
			bridge.agent_delta.connect(_on_delta_received)

	# 生成地图数据
	_generate_map_data()
	print("[WorldRenderer] 地图生成完成：%d 个单元格" % _map_data.size())


func _load_structure_textures() -> void:
	for struct_type in STRUCTURE_TEXTURE_MAP:
		var path = STRUCTURE_TEXTURE_MAP[struct_type]
		var tex = load(path)
		if tex:
			_structure_textures[struct_type] = tex
			print("[WorldRenderer] 加载建筑纹理: %s" % struct_type)
		else:
			push_warning("[WorldRenderer] 无法加载建筑纹理: %s" % path)


func _load_resource_textures() -> void:
	# 尝试加载资源纹理，如果不存在则使用颜色回退
	var resource_types = ["food", "water", "wood", "stone", "iron"]
	for res_type in resource_types:
		var path = "res://assets/textures/resource_%s.png" % res_type
		var tex = load(path)
		if tex:
			_resource_textures[res_type] = tex
			print("[WorldRenderer] 加载资源纹理: %s" % res_type)
		else:
			print("[WorldRenderer] 资源纹理不存在，使用颜色回退: %s" % res_type)


func _generate_map_data() -> void:
	for x in range(_map_size):
		for y in range(_map_size):
			var terrain = _get_terrain_for_position(x, y)
			_map_data["%d_%d" % [x, y]] = terrain


func _get_terrain_for_position(x: int, y: int) -> String:
	var noise = _fractal_noise(x, y, 4)

	if noise < 0.28:
		return "water"
	elif noise < 0.45:
		return "plains"
	elif noise < 0.65:
		return "forest"
	elif noise < 0.82:
		return "mountain"
	else:
		return "desert"


# 多层噪声叠加，产生自然地形的斑块效果
func _fractal_noise(x: int, y: int, octaves: int) -> float:
	var value = 0.0
	var amplitude = 1.0
	var frequency = 1.0
	var max_value = 0.0

	for i in range(octaves):
		var nx = x * 0.02 * frequency
		var ny = y * 0.02 * frequency
		# 使用多个角度的正弦波叠加
		value += amplitude * (
			sin(nx * 1.2 + ny * 0.7 + i * 3.1) * 0.35 +
			sin(ny * 1.5 - nx * 0.5 + i * 2.3) * 0.35 +
			cos((nx + ny) * 0.8 + i * 1.7) * 0.3
		)
		max_value += amplitude
		amplitude *= 0.5
		frequency *= 2.1

	# 归一化到 [0, 1]
	value = value / max_value
	value = value * 0.5 + 0.5  # 从 [-1, 1] 映射到 [0, 1]
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

			var pos_x = x * _tile_size
			var pos_y = y * _tile_size

			# 优先使用纹理，回退到颜色
			var texture: Texture2D = _terrain_textures.get(terrain)
			if texture:
				draw_texture(texture, Vector2(pos_x, pos_y))
			else:
				var color: Color = TERRAIN_COLORS.get(terrain, Color.WHITE)
				draw_rect(Rect2(pos_x, pos_y, _tile_size, _tile_size), color)

	# 绘制资源
	_draw_resources(start_x, start_y, end_x, end_y)

	# 绘制建筑结构
	_draw_structures(start_x, start_y, end_x, end_y)


# 绘制资源
func _draw_resources(start_x: int, start_y: int, end_x: int, end_y: int) -> void:
	for pos_key in _resources:
		var parts = pos_key.split("_")
		if parts.size() < 2:
			continue
		var rx = parts[0].to_int()
		var ry = parts[1].to_int()

		# 只绘制可见区域内的资源
		if rx < start_x or rx > end_x or ry < start_y or ry > end_y:
			continue

		var res_info = _resources[pos_key]
		var res_type = res_info.get("type", "food").to_lower()
		var amount = res_info.get("amount", 0)

		if amount <= 0:
			continue  # 跳过已耗尽的资源

		var pos_x = rx * _tile_size
		var pos_y = ry * _tile_size

		# 尝试使用纹理
		var texture: Texture2D = _resource_textures.get(res_type)
		if texture:
			draw_texture(texture, Vector2(pos_x, pos_y))
		else:
			# 回退到颜色块
			var color: Color = RESOURCE_COLORS.get(res_type, Color.MAGENTA)
			# 绘制一个小的资源指示器（8x8像素，居中于格子）
			var indicator_size = 8
			var offset = (_tile_size - indicator_size) / 2
			draw_rect(Rect2(pos_x + offset, pos_y + offset, indicator_size, indicator_size), color)

			# 根据资源量调整透明度（模拟丰富度）
			var alpha = clampf(float(amount) / 100.0, 0.3, 1.0)
			var overlay_color = Color(color.r, color.g, color.b, alpha * 0.5)
			draw_rect(Rect2(pos_x + offset - 2, pos_y + offset - 2, indicator_size + 4, indicator_size + 4), overlay_color)


func _draw_structures(start_x: int, start_y: int, end_x: int, end_y: int) -> void:
	for pos_key in _structures:
		var parts = pos_key.split("_")
		if parts.size() < 2:
			continue
		var sx = parts[0].to_int()
		var sy = parts[1].to_int()

		# 只绘制可见区域内的建筑
		if sx < start_x or sx > end_x or sy < start_y or sy > end_y:
			continue

		var struct_info = _structures[pos_key]
		var struct_type = struct_info.get("type", "Camp")
		var texture: Texture2D = _structure_textures.get(struct_type)
		if texture:
			draw_texture(texture, Vector2(sx * _tile_size, sy * _tile_size))

		# 绘制建筑效果
		_draw_structure_effects(sx, sy, struct_type)


# 绘制建筑效果（Camp治疗光环、Fence阻挡等）
func _draw_structure_effects(x: int, y: int, struct_type: String) -> void:
	var center = Vector2(x * _tile_size + _tile_size / 2, y * _tile_size + _tile_size / 2)

	match struct_type:
		"Camp":
			# Camp 治疗：脉动光环效果
			var pulse = sin(_effect_time * 2.0) * 0.3 + 0.7  # 0.4 ~ 1.0
			var radius = _tile_size * 2.5  # 曼哈顿距离 ≤ 1 对应半径
			var color = Color(0.2, 0.9, 0.3, 0.15 * pulse)  # 绿色透明
			draw_circle(center, radius, color)
			# 内圈
			var inner_radius = _tile_size * 1.5
			var inner_color = Color(0.2, 0.9, 0.3, 0.25 * pulse)
			draw_circle(center, inner_radius, inner_color)

		"Warehouse":
			# Warehouse：库存容量扩展指示
			var ring_time = fmod(_effect_time, 3.0) / 3.0  # 0-1 循环
			var radius = lerp(float(_tile_size * 0.6), float(_tile_size * 1.0), ring_time)
			var alpha = 1.0 - ring_time * 0.5
			var color = Color(0.9, 0.7, 0.2, 0.3 * alpha)  # 金色
			draw_circle(center, radius, color)

		"Fence", "Wall":
			# Fence/Wall：防御指示线
			var angle = _effect_time * 0.5  # 缓慢旋转
			for i in range(4):
				var rot = angle + i * PI / 2
				var start = center + Vector2(cos(rot), sin(rot)) * _tile_size * 0.4
				var end = center + Vector2(cos(rot), sin(rot)) * _tile_size * 0.8
				var line_color = Color(0.8, 0.2, 0.2, 0.4)  # 红色防御线
				draw_line(start, end, line_color, 2.0)


func _process(_delta: float) -> void:
	# 更新效果动画时间
	_effect_time += _delta
	# 每帧重绘
	queue_redraw()


func _on_world_updated(snapshot: Dictionary) -> void:
	# 从 snapshot 加载建筑和资源信息（兜底同步）
	if snapshot.has("map_changes"):
		for change in snapshot.map_changes:
			var key = "%d_%d" % [change.x, change.y]

			# 处理建筑
			if change.has("structure") and change.structure != null and change.structure != "":
				_structures[key] = {"type": _normalize_structure_type(change.structure)}

			# 处理资源
			if change.has("resource_type") and change.resource_type != null and change.resource_type != "":
				var amount = change.get("resource_amount", 0)
				_resources[key] = {"type": change.resource_type.to_lower(), "amount": amount}
			elif change.has("resource_amount") and change.get("resource_amount", 0) <= 0:
				# 资源耗尽，移除
				_resources.erase(key)

	_needs_redraw = true
	queue_redraw()


# ===== Tier 2: Delta 事件处理 =====

func _on_delta_received(delta: Dictionary) -> void:
	var delta_type = delta.get("type", "")

	match delta_type:
		"structure_created":
			_on_structure_created(delta)
		"structure_destroyed":
			_on_structure_destroyed(delta)
		"resource_changed":
			_on_resource_changed(delta)


func _on_structure_created(delta: Dictionary) -> void:
	var pos: Vector2 = delta.get("position", Vector2.ZERO)
	var x = int(pos.x)
	var y = int(pos.y)
	var struct_type = delta.get("structure_type", "Camp")
	var key = "%d_%d" % [x, y]

	_structures[key] = {"type": _normalize_structure_type(struct_type)}
	print("[WorldRenderer] 建筑创建: %s at (%d, %d)" % [struct_type, x, y])
	queue_redraw()


func _on_structure_destroyed(delta: Dictionary) -> void:
	var pos: Vector2 = delta.get("position", Vector2.ZERO)
	var x = int(pos.x)
	var y = int(pos.y)
	var key = "%d_%d" % [x, y]

	if _structures.has(key):
		_structures.erase(key)
		print("[WorldRenderer] 建筑销毁: (%d, %d)" % [x, y])
		queue_redraw()


func _on_resource_changed(delta: Dictionary) -> void:
	# 更新资源字典并触发重绘
	var x = delta.get("x", 0)
	var y = delta.get("y", 0)
	var resource_type = delta.get("resource_type", "").to_lower()
	var amount = delta.get("amount", 0)

	var key = "%d_%d" % [x, y]

	if amount > 0 and resource_type != "":
		_resources[key] = {"type": resource_type, "amount": amount}
		print("[WorldRenderer] 资源更新: %s at (%d, %d) = %d" % [resource_type, x, y, amount])
	else:
		# 资源耗尽，移除
		if _resources.has(key):
			_resources.erase(key)
			print("[WorldRenderer] 资源耗尽: (%d, %d)" % [x, y])

	queue_redraw()


func _normalize_structure_type(raw: String) -> String:
	# 从 "Camp" / "Structure::Camp" 等格式提取类型名
	if raw.contains("::"):
		return raw.split("::")[1]
	return raw
