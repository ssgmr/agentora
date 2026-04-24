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
	"food": Color(0.9, 0.6, 0.2),
	"water": Color(0.3, 0.6, 0.9),
	"wood": Color(0.6, 0.4, 0.2),
	"stone": Color(0.7, 0.7, 0.7),
	"iron": Color(0.5, 0.5, 0.6),
}

# 建筑 Sprite 节点容器
var _structure_sprites: Node2D = null

# 地形颜色配置（回退用）
const TERRAIN_COLORS = {
	"plains": Color(0.3, 0.6, 0.2),
	"forest": Color(0.1, 0.4, 0.1),
	"mountain": Color(0.5, 0.5, 0.5),
	"water": Color(0.2, 0.4, 0.8),
	"desert": Color(0.8, 0.7, 0.3)
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

# 建筑显示名称映射
const STRUCTURE_DISPLAY_NAMES = {
	"Camp": "营地",
	"Campfire": "篝火",
	"Warehouse": "仓库",
	"Fortress": "堡垒",
	"WatchTower": "哨塔",
	"Fence": "围栏",
	"Wall": "城墙",
}

var _tile_size: int = 16
var _map_size: int = -1
var _map_data: Dictionary = {}
var _needs_redraw: bool = true
var _debug_print_done: bool = false
var _effect_time: float = 0.0
var _default_font: Font = null
var _debug_draw_count: int = 0
const _TERRAIN_NAMES = ["plains", "forest", "mountain", "water", "desert"]


func _ready() -> void:
	print("[WorldRenderer] 世界渲染器初始化")
	set_process(true)

	_terrain_textures["plains"] = load("res://assets/textures/terrain_plains.png")
	_terrain_textures["forest"] = load("res://assets/textures/terrain_forest.png")
	_terrain_textures["mountain"] = load("res://assets/textures/terrain_mountain.png")
	_terrain_textures["water"] = load("res://assets/textures/terrain_water.png")
	_terrain_textures["desert"] = load("res://assets/textures/terrain_desert.png")

	_load_structure_textures()
	_load_resource_textures()

	_default_font = ThemeDB.fallback_font
	if _default_font:
		print("[WorldRenderer] 默认字体加载成功")
	else:
		push_warning("[WorldRenderer] 无法加载默认字体")

	_structure_sprites = Node2D.new()
	_structure_sprites.name = "StructureSprites"
	add_child(_structure_sprites)

	StateManager.state_updated.connect(_on_state_updated)
	StateManager.terrain_changed.connect(_on_terrain_changed)
	StateManager.resource_changed.connect(_on_resource_changed)
	StateManager.structure_changed.connect(_on_structure_changed)
	print("[WorldRenderer] StateManager 信号已连接")

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
	_map_data.clear()
	print("[WorldRenderer] 等待后端 snapshot 提供地图数据...")


func _draw() -> void:
	var camera = get_node_or_null("../Camera2D")
	if camera == null:
		return

	var camera_pos = camera.position
	var zoom = camera.zoom
	var viewport_size = get_viewport().get_visible_rect().size

	var start_x = int((camera_pos.x - viewport_size.x / 2 / zoom.x) / _tile_size) - 1
	var start_y = int((camera_pos.y - viewport_size.y / 2 / zoom.y) / _tile_size) - 1
	var end_x = int((camera_pos.x + viewport_size.x / 2 / zoom.x) / _tile_size) + 1
	var end_y = int((camera_pos.y + viewport_size.y / 2 / zoom.y) / _tile_size) + 1

	start_x = maxi(start_x, 0)
	start_y = maxi(start_y, 0)
	end_x = mini(end_x, _map_size - 1)
	end_y = mini(end_y, _map_size - 1)

	for x in range(start_x, end_x + 1):
		for y in range(start_y, end_y + 1):
			var key = "%d_%d" % [x, y]
			var terrain: String = _map_data.get(key, "plains")
			var pos_x = x * _tile_size
			var pos_y = y * _tile_size
			var texture: Texture2D = _terrain_textures.get(terrain)
			if texture:
				draw_texture(texture, Vector2(pos_x, pos_y))
			else:
				var color: Color = TERRAIN_COLORS.get(terrain, Color.WHITE)
				draw_rect(Rect2(pos_x, pos_y, _tile_size, _tile_size), color)

	_draw_resources(start_x, start_y, end_x, end_y)

	if _resources.size() > 0 and _debug_print_done == false:
		_debug_print_done = true
		print("[WorldRenderer] 资源总数: %d" % _resources.size())

	_draw_structures(start_x, start_y, end_x, end_y)


func _draw_resources(start_x: int, start_y: int, end_x: int, end_y: int) -> void:
	for pos_key in _resources:
		var parts = pos_key.split("_")
		if parts.size() < 2:
			continue
		var rx = parts[0].to_int()
		var ry = parts[1].to_int()

		if rx < start_x or rx > end_x or ry < start_y or ry > end_y:
			continue

		var res_info = _resources[pos_key]
		var res_type = res_info.get("type", "food").to_lower()
		var amount = res_info.get("amount", 0)

		if amount <= 0:
			continue

		var pos_x = rx * _tile_size
		var pos_y = ry * _tile_size

		var texture: Texture2D = _resource_textures.get(res_type)
		if texture:
			draw_texture(texture, Vector2(pos_x, pos_y))
		else:
			var color: Color = RESOURCE_COLORS.get(res_type, Color.MAGENTA)
			var indicator_size = 8
			var offset = (_tile_size - indicator_size) / 2.0
			draw_rect(Rect2(pos_x + offset, pos_y + offset, indicator_size, indicator_size), color)

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

		if sx < start_x or sx > end_x or sy < start_y or sy > end_y:
			continue

		var struct_info = _structures[pos_key]
		var struct_type = struct_info.get("type", "Camp")
		var texture: Texture2D = _structure_textures.get(struct_type)
		if texture:
			draw_texture(texture, Vector2(sx * _tile_size, sy * _tile_size))

		_draw_structure_effects(sx, sy, struct_type)

		var owner_id = struct_info.get("owner_id", "")
		_draw_structure_label(sx, sy, struct_type, owner_id)


func _draw_structure_effects(x: int, y: int, struct_type: String) -> void:
	var center = Vector2(x * _tile_size + _tile_size / 2.0, y * _tile_size + _tile_size / 2.0)

	match struct_type:
		"Camp":
			var pulse = sin(_effect_time * 2.0) * 0.3 + 0.7
			var radius = _tile_size * 2.5
			var color = Color(0.2, 0.9, 0.3, 0.15 * pulse)
			draw_circle(center, radius, color)
			var inner_radius = _tile_size * 1.5
			var inner_color = Color(0.2, 0.9, 0.3, 0.25 * pulse)
			draw_circle(center, inner_radius, inner_color)

		"Warehouse":
			var ring_time = fmod(_effect_time, 3.0) / 3.0
			var radius = lerp(float(_tile_size * 0.6), float(_tile_size * 1.0), ring_time)
			var alpha = 1.0 - ring_time * 0.5
			var color = Color(0.9, 0.7, 0.2, 0.3 * alpha)
			draw_circle(center, radius, color)

		"Fence", "Wall":
			var angle = _effect_time * 0.5
			for i in range(4):
				var rot = angle + i * PI / 2
				var start = center + Vector2(cos(rot), sin(rot)) * _tile_size * 0.4
				var end = center + Vector2(cos(rot), sin(rot)) * _tile_size * 0.8
				var line_color = Color(0.8, 0.2, 0.2, 0.4)
				draw_line(start, end, line_color, 2.0)


func _draw_structure_label(x: int, y: int, struct_type: String, owner_id: String) -> void:
	if not _default_font:
		return

	var display_name = STRUCTURE_DISPLAY_NAMES.get(struct_type, struct_type)

	var owner_name = ""
	if owner_id != "":
		var agent_data = StateManager.get_agent_data(owner_id)
		if agent_data.has("name"):
			owner_name = agent_data.get("name", "")
		else:
			owner_name = owner_id.substr(0, 6)

	var lines = []
	lines.append(display_name)
	if owner_name != "":
		lines.append("[" + owner_name + "]")

	var pos_x = x * _tile_size
	var pos_y = y * _tile_size

	var font_size = 10
	var line_height = font_size + 2
	var max_line_width = 0
	for line in lines:
		var w = _default_font.get_string_size(line, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x
		max_line_width = max(max_line_width, w)

	var bg_width = max_line_width + 8
	var bg_height = lines.size() * line_height + 4

	var bg_rect = Rect2(pos_x - 2, pos_y - bg_height - 2, bg_width, bg_height)
	draw_rect(bg_rect, Color(0.0, 0.0, 0.0, 0.75))
	draw_rect(bg_rect, Color(1.0, 0.8, 0.2, 0.6), false, 1.0)

	var text_y = pos_y - bg_height + 1
	for i in range(lines.size()):
		var line = lines[i]
		var color = Color(1.0, 0.85, 0.3, 1.0) if i == 0 else Color(1.0, 1.0, 1.0, 0.9)
		draw_string(_default_font, Vector2(pos_x + 1, text_y + font_size + 1), line,
			HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, Color(0, 0, 0, 0.8))
		draw_string(_default_font, Vector2(pos_x, text_y + font_size), line,
			HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, color)
		text_y += line_height


func _process(_delta: float) -> void:
	_effect_time += _delta
	queue_redraw()


func _on_state_updated(snapshot: Dictionary) -> void:
	if snapshot.has("terrain_grid") and snapshot.has("terrain_width") and snapshot.has("terrain_height"):
		var grid: PackedByteArray = snapshot.terrain_grid
		var tw: int = snapshot.terrain_width
		var th: int = snapshot.terrain_height
		_map_size = tw
		_decode_terrain_grid(grid, tw, th)
	elif snapshot.has("map_changes"):
		for change in snapshot.map_changes:
			var key = "%d_%d" % [change.x, change.y]
			if change.has("terrain") and change.terrain != null and change.terrain != "":
				_map_data[key] = change.terrain.to_lower()

	if snapshot.has("map_changes"):
		var resource_count = 0
		var structure_count = 0
		for change in snapshot.map_changes:
			var key = "%d_%d" % [change.x, change.y]
			if change.has("structure") and change.structure != null and change.structure != "":
				_structures[key] = {"type": _normalize_structure_type(change.structure)}
				structure_count += 1
			if change.has("resource_type") and change.resource_type != null and change.resource_type != "":
				var amount = change.get("resource_amount", 0)
				_resources[key] = {"type": change.resource_type.to_lower(), "amount": amount}
				resource_count += 1
			elif change.has("resource_amount") and change.get("resource_amount", 0) <= 0:
				_resources.erase(key)
		print("[WorldRenderer] Snapshot: %d 资源, %d 建筑" % [resource_count, structure_count])

	_needs_redraw = true
	queue_redraw()


func _on_terrain_changed(x: int, y: int, terrain_type: String) -> void:
	var key = "%d_%d" % [x, y]
	_map_data[key] = terrain_type.to_lower()
	queue_redraw()


func _on_resource_changed(x: int, y: int, resource_type: String, amount: int) -> void:
	var key = "%d_%d" % [x, y]
	if amount > 0 and resource_type != "":
		_resources[key] = {"type": resource_type.to_lower(), "amount": amount}
	else:
		_resources.erase(key)
	queue_redraw()


func _on_structure_changed(x: int, y: int, structure_type: String, owner_id: String) -> void:
	var key = "%d_%d" % [x, y]
	if structure_type != "":
		_structures[key] = {"type": _normalize_structure_type(structure_type), "owner_id": owner_id}
	else:
		_structures.erase(key)
	queue_redraw()


func _decode_terrain_grid(grid: PackedByteArray, width: int, height: int) -> void:
	_map_data.clear()
	_map_size = width
	for y in range(height):
		for x in range(width):
			var idx = y * width + x
			var terrain_idx = grid[idx]
			var key = "%d_%d" % [x, y]
			if terrain_idx < _TERRAIN_NAMES.size():
				_map_data[key] = _TERRAIN_NAMES[terrain_idx]
			else:
				_map_data[key] = "plains"
	print("[WorldRenderer] 地形网格解码: %dx%d = %d 格" % [width, height, _map_data.size()])


func _normalize_structure_type(raw: String) -> String:
	if raw.contains("::"):
		return raw.split("::")[1]
	return raw