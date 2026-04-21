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
var _map_size: int = -1  # 未知，等待 snapshot 到来
# 地图数据字典 (key "x_y" -> terrain string)，由后端 snapshot 填充
var _map_data: Dictionary = {}
var _needs_redraw: bool = true
var _debug_print_done: bool = false

# 建筑效果动画状态
var _effect_time: float = 0.0

# 默认字体（用于资源数量标签）
var _default_font: Font = null


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

	# 加载默认字体
	_default_font = ThemeDB.fallback_font
	if _default_font:
		print("[WorldRenderer] 默认字体加载成功")
	else:
		push_warning("[WorldRenderer] 无法加载默认字体，资源数量标签将无法显示")

	# 创建建筑 Sprite 容器
	_structure_sprites = Node2D.new()
	_structure_sprites.name = "StructureSprites"
	add_child(_structure_sprites)

	# 连接信号
	var bridge = BridgeAccessor.get_bridge()
	if bridge:
		print("[WorldRenderer] 找到 SimulationBridge 节点")
		bridge.world_updated.connect(_on_world_updated)
		print("[WorldRenderer] world_updated 信号已连接")
		# 连接 delta 信号（Tier 2）
		if bridge.has_signal("agent_delta"):
			bridge.agent_delta.connect(_on_delta_received)
	else:
		push_error("[WorldRenderer] 未找到 SimulationBridge 节点！")

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


# 生成地图数据（初始为空，等待后端 snapshot 填充真实地形）
func _generate_map_data() -> void:
	# 不再预生成，等待 snapshot 提供真实地图尺寸
	_map_data.clear()
	print("[WorldRenderer] 等待后端 snapshot 提供地图数据...")


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

	# 一次性打印绘制状态
	if _resources.size() > 0 and _debug_print_done == false:
		_debug_print_done = true
		print("[WorldRenderer] 资源总数: %d, 首项: %s" % [_resources.size(), _resources.keys()[0]])

	# 绘制建筑结构
	_draw_structures(start_x, start_y, end_x, end_y)


# 绘制资源
var _debug_draw_count: int = 0
func _draw_resources(start_x: int, start_y: int, end_x: int, end_y: int) -> void:
	_debug_draw_count = 0
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
			var offset = (_tile_size - indicator_size) / 2.0
			draw_rect(Rect2(pos_x + offset, pos_y + offset, indicator_size, indicator_size), color)

			# 根据资源量调整透明度（模拟丰富度）
			var alpha = clampf(float(amount) / 100.0, 0.3, 1.0)
			var overlay_color = Color(color.r, color.g, color.b, alpha * 0.5)
			draw_rect(Rect2(pos_x + offset - 2, pos_y + offset - 2, indicator_size + 4, indicator_size + 4), overlay_color)

		# 绘制资源数量标签（右上角）
		if amount > 0 and _default_font:
			var label_pos = Vector2(pos_x + _tile_size - 2, pos_y + 2)
			draw_string(_default_font, label_pos, str(amount),
				HORIZONTAL_ALIGNMENT_RIGHT, 20, 10, Color.WHITE)


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
	var center = Vector2(x * _tile_size + _tile_size / 2.0, y * _tile_size + _tile_size / 2.0)

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
	# 从 snapshot 加载地形、建筑和资源信息（兜底同步）

	# 处理地形网格数据（优先使用，包含全图地形）
	if snapshot.has("terrain_grid") and snapshot.has("terrain_width") and snapshot.has("terrain_height"):
		var grid: PackedByteArray = snapshot.terrain_grid
		var tw: int = snapshot.terrain_width
		var th: int = snapshot.terrain_height
		# 更新地图尺寸（从后端获取）
		_map_size = tw
		_decode_terrain_grid(grid, tw, th)
	elif snapshot.has("map_changes"):
		# 回退：从 map_changes 解析地形（只包含有资源/建筑的格子）
		for change in snapshot.map_changes:
			var key = "%d_%d" % [change.x, change.y]
			if change.has("terrain") and change.terrain != null and change.terrain != "":
				_map_data[key] = change.terrain.to_lower()

	# 处理建筑和资源
	if snapshot.has("map_changes"):
		var resource_count = 0
		var structure_count = 0
		for change in snapshot.map_changes:
			var key = "%d_%d" % [change.x, change.y]

			# 处理建筑
			if change.has("structure") and change.structure != null and change.structure != "":
				_structures[key] = {"type": _normalize_structure_type(change.structure)}
				structure_count += 1

			# 处理资源
			if change.has("resource_type") and change.resource_type != null and change.resource_type != "":
				var amount = change.get("resource_amount", 0)
				_resources[key] = {"type": change.resource_type.to_lower(), "amount": amount}
				resource_count += 1
			elif change.has("resource_amount") and change.get("resource_amount", 0) <= 0:
				# 资源耗尽，移除
				_resources.erase(key)

		print("[WorldRenderer] Snapshot 处理: %d 个资源, %d 个建筑 (总计 %d 个 map_changes)" % [resource_count, structure_count, snapshot.map_changes.size()])

	print("[WorldRenderer] _resources 字典大小: %d" % _resources.size())

	_needs_redraw = true
	queue_redraw()


# 解码地形网格数据（0=plains, 1=forest, 2=mountain, 3=water, 4=desert）
const _TERRAIN_NAMES = ["plains", "forest", "mountain", "water", "desert"]

func _decode_terrain_grid(grid: PackedByteArray, width: int, height: int) -> void:
	_map_data.clear()
	_map_size = width  # 更新地图尺寸
	for y in range(height):
		for x in range(width):
			var idx = y * width + x
			var terrain_idx = grid[idx]
			var key = "%d_%d" % [x, y]
			if terrain_idx < _TERRAIN_NAMES.size():
				_map_data[key] = _TERRAIN_NAMES[terrain_idx]
			else:
				_map_data[key] = "plains"
	print("[WorldRenderer] 地形网格解码: %dx%d = %d 格 (地图尺寸已更新)" % [width, height, _map_data.size()])


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
