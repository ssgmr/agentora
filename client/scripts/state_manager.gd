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
signal filter_changed()  # 频道/过滤变化信号

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
var _narrative_channel: String = "nearby"  # 当前叙事频道 (local/nearby/world)，默认 nearby 显示移动/采集事件
var _narrative_agent_filter: String = ""  # 当前叙事 Agent 过滤（空=全部）

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

## 获取当前叙事频道
func get_narrative_channel() -> String:
	return _narrative_channel

## 设置叙事频道
func set_narrative_channel(channel: String) -> void:
	if channel in ["local", "nearby", "world"]:
		_narrative_channel = channel
		filter_changed.emit()

## 获取叙事 Agent 过滤
func get_narrative_agent_filter() -> String:
	return _narrative_agent_filter

## 设置叙事 Agent 过滤
func set_narrative_agent_filter(agent_id: String) -> void:
	_narrative_agent_filter = agent_id
	filter_changed.emit()

## 获取过滤后的叙事事件列表
##
## 根据当前频道和 Agent 过滤返回叙事事件
## - 世界频道忽略 Agent 过滤（显示所有世界事件）
## - 其他频道组合频道和 Agent 过滤
func get_filtered_narratives() -> Array:
	var filtered = _narrative_events.duplicate()

	# 世界频道：只显示世界事件，忽略 Agent 过滤
	if _narrative_channel == "world":
		return filtered.filter(func(e):
			return e.get("channel", "local") == "world"
		)

	# 其他频道：先按频道过滤
	if _narrative_channel != "":
		filtered = filtered.filter(func(e):
			return e.get("channel", "local") == _narrative_channel
		)

	# 然后按 Agent 过滤（可选）
	if _narrative_agent_filter != "":
		filtered = filtered.filter(func(e):
			return e.get("agent_id", "") == _narrative_agent_filter
		)

	return filtered

# ===== 内部方法 =====

func _terrain_byte_to_type(byte: int) -> String:
	match byte:
		0: return "plains"
		1: return "forest"
		2: return "mountain"
		3: return "water"
		4: return "desert"
		_: return "plains"

## 解析 position 数据（兼容 Vector2 和 Dictionary{x,y} 格式）
func _parse_position_variant(pos_variant) -> Vector2:
	if pos_variant is Vector2:
		return pos_variant
	if pos_variant is Dictionary:
		var d = pos_variant as Dictionary
		return Vector2(d.get("x", 0), d.get("y", 0))
	return Vector2.ZERO

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
## 扁平格式（与 snapshot agent_to_dict 一致）
func _on_agent_delta(delta: Dictionary) -> void:
	var event_type = delta.get("type", "")

	match event_type:
		"agent_state_changed":
			var agent_id = delta.get("agent_id", "")
			var is_alive: bool = delta.get("is_alive", true)
			var change_hint = delta.get("change_hint", "")

			# 死亡 Agent：从 _agents 移除（与 snapshot 过滤逻辑一致）
			if not is_alive:
				if _agents.has(agent_id):
					_agents.erase(agent_id)
					agent_changed.emit(agent_id, {"is_alive": false, "removed": true})
				return

			# 解析 position（兼容 Vector2 和 Dictionary{x,y} 格式）
			var pos = _parse_position_variant(delta.get("position", Vector2.ZERO))

			var agent_data = {
				"id": agent_id,
				"name": delta.get("name", ""),
				"position": pos,
				"health": delta.get("health", 100),
				"max_health": delta.get("max_health", 100),
				"satiety": delta.get("satiety", 50),
				"hydration": delta.get("hydration", 50),
				"is_alive": true,
				"age": delta.get("age", 0),
				"level": delta.get("level", 1),
				"current_action": delta.get("current_action", ""),
				"action_result": delta.get("action_result", ""),
				"reasoning": delta.get("reasoning", ""),
				"inventory_summary": delta.get("inventory_summary", {}),
				"change_hint": change_hint,
				"source_peer_id": delta.get("source_peer_id", ""),
					"icon_id": delta.get("icon_id", "default"),
					"custom_icon_path": delta.get("custom_icon_path", "")
			}
			_agents[agent_id] = agent_data
			agent_changed.emit(agent_id, agent_data)

		"world_event":
			var world_event_type = delta.get("event_type", "")
			match world_event_type:
				"structure_created":
					var pos = delta.get("position", [0, 0])
					var structure_type = delta.get("structure_type", "")
					var owner_id = delta.get("owner_id", "")
					var key = str(pos[0]) + "," + str(pos[1])
					_structures[key] = {"type": structure_type, "owner_id": owner_id}
					structure_changed.emit(pos[0], pos[1], structure_type, owner_id)

				"structure_destroyed":
					var pos = delta.get("position", [0, 0])
					var key = str(pos[0]) + "," + str(pos[1])
					_structures.erase(key)
					structure_changed.emit(pos[0], pos[1], "", "")

				"resource_changed":
					var pos = delta.get("position", [0, 0])
					var resource_type = delta.get("resource_type", "")
					var amount = delta.get("amount", 0)
					var key = str(pos[0]) + "," + str(pos[1])
					_resources[key] = {"type": resource_type, "amount": amount}
					resource_changed.emit(pos[0], pos[1], resource_type, amount)

				"milestone_reached":
					var name = delta.get("name", "")
					var display_name = delta.get("display_name", "")
					var tick = delta.get("tick", 0)
					_milestones.append({"name": name, "display_name": display_name, "tick": tick})
					milestone_reached.emit(name, display_name, tick)

				"agent_narrative":
					var event_data = {
						"tick": delta.get("narrative_tick", 0),
						"agent_id": delta.get("narrative_agent_id", ""),
						"agent_name": delta.get("narrative_agent_name", ""),
						"event_type": delta.get("narrative_event_type", ""),
						"description": delta.get("narrative_description", ""),
						"color_code": delta.get("narrative_color", "#FFFFFF"),
						"channel": delta.get("narrative_channel", "local")
					}
					_narrative_events.append(event_data)
					narrative_added.emit(event_data)

## 处理 narrative_event 信号
func _on_narrative_event(event: Dictionary) -> void:
	_narrative_events.append(event)
	narrative_added.emit(event)
