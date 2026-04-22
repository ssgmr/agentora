## StateManager - 统一状态分发中心
##
## 作为 Autoload 加载，管理世界状态并向各组件分发更新信号。
## 所有 UI 组件订阅 StateManager 而非直接监听 Bridge。

extends Node

# ===== 信号定义 =====
signal state_updated(snapshot: Dictionary)
signal agent_changed(agent_id: String, agent_data: Dictionary)
signal terrain_changed(x: int, y: int, terrain_type: String)
signal resource_changed(x: int, y: int, resource_type: String, amount: int)
signal structure_changed(x: int, y: int, structure_type: String, owner_id: String)
signal narrative_added(event: Dictionary)
signal milestone_reached(name: String, display_name: String, tick: int)

# ===== 状态容器 =====
var _agents: Dictionary = {}  # agent_id -> agent_data
var _terrain_grid: PackedByteArray = PackedByteArray()
var _terrain_width: int = 0
var _terrain_height: int = 0
var _resources: Dictionary = {}  # (x,y) -> {type, amount}
var _structures: Dictionary = {}  # (x,y) -> {type, owner_id}
var _narrative_events: Array = []  # 叙事事件列表
var _milestones: Array = []  # 里程碑列表
var _current_tick: int = 0

# ===== 查询接口 =====

## 获取 Agent 数据
func get_agent_data(agent_id: String) -> Dictionary:
	return _agents.get(agent_id, {})

## 获取所有 Agent 数据
func get_all_agents() -> Dictionary:
	return _agents

## 获取地形类型
func get_terrain_at(x: int, y: int) -> String:
	if _terrain_width == 0 or _terrain_height == 0:
		return "plains"
	if x < 0 or x >= _terrain_width or y < 0 or y >= _terrain_height:
		return "mountain"  # 越界视为不可通行
	var idx = y * _terrain_width + x
	if idx < _terrain_grid.size():
		var terrain_byte = _terrain_grid[idx]
		return _terrain_byte_to_type(terrain_byte)
	return "plains"

## 获取资源数据
func get_resource_at(x: int, y: int) -> Dictionary:
	var key = str(x) + "," + str(y)
	return _resources.get(key, {})

## 获取建筑数据
func get_structure_at(x: int, y: int) -> Dictionary:
	var key = str(x) + "," + str(y)
	return _structures.get(key, {})

## 获取当前 tick
func get_current_tick() -> int:
	return _current_tick

## 获取叙事事件列表
func get_narrative_events() -> Array:
	return _narrative_events

## 获取里程碑列表
func get_milestones() -> Array:
	return _milestones

## 获取地形尺寸
func get_map_size() -> Vector2i:
	return Vector2i(_terrain_width, _terrain_height)

# ===== 内部方法 =====

func _terrain_byte_to_type(byte: int) -> String:
	match byte:
		0: return "plains"
		1: return "forest"
		2: return "mountain"
		3: return "water"
		4: return "desert"
		_: return "plains"

# ===== Bridge 信号处理 =====

## 处理 world_updated 信号（完整 snapshot）
func _on_world_updated(snapshot: Dictionary) -> void:
	_current_tick = snapshot.get("tick", 0)

	# 解析地形网格
	if snapshot.has("terrain_grid"):
		_terrain_grid = snapshot["terrain_grid"]
	if snapshot.has("terrain_width"):
		_terrain_width = snapshot["terrain_width"]
	if snapshot.has("terrain_height"):
		_terrain_height = snapshot["terrain_height"]

	# 解析 Agent 数据
	if snapshot.has("agents"):
		var agents_dict = snapshot["agents"]
		for agent_id in agents_dict.keys():
			var agent_data = agents_dict[agent_id]
			_agents[agent_id] = agent_data
			agent_changed.emit(agent_id, agent_data)

	# 解析地图变更
	if snapshot.has("map_changes"):
		var map_changes = snapshot["map_changes"]
		for change in map_changes:
			var x = change.get("x", 0)
			var y = change.get("y", 0)
			if change.has("terrain"):
				terrain_changed.emit(x, y, change["terrain"])
			if change.has("resource_type"):
				var key = str(x) + "," + str(y)
				_resources[key] = {
					"type": change["resource_type"],
					"amount": change.get("resource_amount", 0)
				}
				resource_changed.emit(x, y, change["resource_type"], change.get("resource_amount", 0))
			if change.has("structure"):
				var key = str(x) + "," + str(y)
				_structures[key] = {
					"type": change["structure"],
					"owner_id": change.get("owner_id", "")
				}
				structure_changed.emit(x, y, change["structure"], change.get("owner_id", ""))

	# 发送全局更新信号
	state_updated.emit(snapshot)

## 处理 agent_delta 信号（增量更新）
func _on_agent_delta(delta: Dictionary) -> void:
	var event_type = delta.get("type", "")

	match event_type:
		"agent_moved":
			var agent_id = delta.get("id", "")
			var name = delta.get("name", "")
			var pos = delta.get("position", Vector2.ZERO)
			var health = delta.get("health", 100)
			var max_health = delta.get("max_health", 100)
			var is_alive = delta.get("is_alive", true)
			var age = delta.get("age", 0)

			var agent_data = {
				"id": agent_id,
				"name": name,
				"position": pos,
				"health": health,
				"max_health": max_health,
				"is_alive": is_alive,
				"age": age
			}
			_agents[agent_id] = agent_data
			agent_changed.emit(agent_id, agent_data)

		"agent_died":
			var agent_id = delta.get("id", "")
			if _agents.has(agent_id):
				_agents[agent_id]["is_alive"] = false
				agent_changed.emit(agent_id, _agents[agent_id])

		"agent_spawned":
			var agent_id = delta.get("id", "")
			var name = delta.get("name", "")
			var pos = delta.get("position", Vector2.ZERO)
			var health = delta.get("health", 100)
			var max_health = delta.get("max_health", 100)

			var agent_data = {
				"id": agent_id,
				"name": name,
				"position": pos,
				"health": health,
				"max_health": max_health,
				"is_alive": true,
				"age": 0
			}
			_agents[agent_id] = agent_data
			agent_changed.emit(agent_id, agent_data)

		"structure_created":
			var x = delta.get("position", Vector2.ZERO).x
			var y = delta.get("position", Vector2.ZERO).y
			var structure_type = delta.get("structure_type", "")
			var owner_id = delta.get("owner_id", "")
			var key = str(int(x)) + "," + str(int(y))
			_structures[key] = {"type": structure_type, "owner_id": owner_id}
			structure_changed.emit(int(x), int(y), structure_type, owner_id)

		"resource_changed":
			var x = delta.get("position", Vector2.ZERO).x
			var y = delta.get("position", Vector2.ZERO).y
			var resource_type = delta.get("resource_type", "")
			var amount = delta.get("amount", 0)
			var key = str(int(x)) + "," + str(int(y))
			_resources[key] = {"type": resource_type, "amount": amount}
			resource_changed.emit(int(x), int(y), resource_type, amount)

		"milestone_reached":
			var name = delta.get("name", "")
			var display_name = delta.get("display_name", "")
			var tick = delta.get("tick", 0)
			_milestones.append({"name": name, "display_name": display_name, "tick": tick})
			milestone_reached.emit(name, display_name, tick)

## 处理 narrative_event 信号
func _on_narrative_event(event: Dictionary) -> void:
	_narrative_events.append(event)
	narrative_added.emit(event)