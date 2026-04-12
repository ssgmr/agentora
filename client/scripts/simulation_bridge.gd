# SimulationBridge - GDExtension 桥接层
# 负责与 Rust 模拟引擎通信
extends Node

# 信号定义
signal world_updated(snapshot: Dictionary)
signal agent_selected(agent_id: String)
signal narrative_event(event: Dictionary)
signal legacy_created(legacy: Dictionary)

# 配置
var tick_interval: float = 2.0  # 秒
var is_paused: bool = false
var selected_agent_id: String = ""

# 内部状态
var _last_snapshot: Dictionary = {}
var _agents_data: Dictionary = {}
var _events_log: Array = []
var _current_tick: int = 0
var _timer: float = 0.0
var _map_data: Dictionary = {}


func _ready() -> void:
	print("[SimulationBridge] 初始化模拟桥接")
	_initialize_map_data()


func _physics_process(delta: float) -> void:
	if is_paused:
		return

	_timer += delta
	if _timer >= tick_interval:
		_timer = 0.0
		_tick()


func _initialize_map_data() -> void:
	# 生成初始地图数据
	for x in range(256):
		for y in range(256):
			var noise = _fractal_noise(x, y, 4)
			var terrain: String
			if noise < 0.28:
				terrain = "water"
			elif noise < 0.45:
				terrain = "plains"
			elif noise < 0.65:
				terrain = "forest"
			elif noise < 0.82:
				terrain = "mountain"
			else:
				terrain = "desert"
			_map_data["%d_%d" % [x, y]] = terrain


# 多层噪声叠加，产生自然地形的斑块效果
func _fractal_noise(x: int, y: int, octaves: int) -> float:
	var value = 0.0
	var amplitude = 1.0
	var frequency = 1.0
	var max_value = 0.0

	for i in range(octaves):
		var nx = x * 0.02 * frequency
		var ny = y * 0.02 * frequency
		value += amplitude * (
			sin(nx * 1.2 + ny * 0.7 + i * 3.1) * 0.35 +
			sin(ny * 1.5 - nx * 0.5 + i * 2.3) * 0.35 +
			cos((nx + ny) * 0.8 + i * 1.7) * 0.3
		)
		max_value += amplitude
		amplitude *= 0.5
		frequency *= 2.1

	value = value / max_value
	value = value * 0.5 + 0.5
	return clampf(value, 0.0, 1.0)


func _tick() -> void:
	_current_tick += 1

	# 更新 Agent 数据
	_update_agents()

	# 生成叙事事件
	_generate_events()

	# 生成快照
	var snapshot = {
		"tick": _current_tick,
		"agents": _agents_data,
		"map_changes": [],
		"events": _events_log.slice(-10),  # 只保留最近 10 个事件
		"legacies": [],
		"pressures": []
	}

	_last_snapshot = snapshot
	_emit_world_updated(snapshot)

	print("[SimulationBridge] Tick %d, Agents: %d" % [_current_tick, _agents_data.size()])


func _update_agents() -> void:
	# 初始化或更新 Agent
	if _agents_data.is_empty():
		for i in range(5):
			var agent_id = "agent_%d" % i
			# 初始位置在地图中心附近
			_agents_data[agent_id] = {
				"id": agent_id,
				"name": "Agent %d" % i,
				"position": Vector2(128 + (i - 2) * 5, 128 + (i - 2) * 3),
				"motivation": [0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
				"health": 100 - i * 5,
				"max_health": 100,
				"age": _current_tick,
				"is_alive": true,
				"current_action": "探索"
			}
	else:
		# 移动 Agent
		for agent_id in _agents_data.keys():
			var agent = _agents_data[agent_id]
			var pos: Vector2 = agent["position"]
			# 随机移动
			var new_pos = pos + Vector2(randf_range(-2, 2), randf_range(-2, 2))
			# 限制在地图内
			new_pos.x = clampf(new_pos.x, 0, 255)
			new_pos.y = clampf(new_pos.y, 0, 255)
			agent["position"] = new_pos
			agent["age"] = _current_tick

			# 随机改变动作
			var actions = ["探索", "采集", "建造", "休息", "交易"]
			agent["current_action"] = actions[randi_range(0, actions.size() - 1)]


func _generate_events() -> void:
	var actions = ["开始探索新的区域", "发现了资源", "建造了一个设施", "与其他 Agent 交易", "休息恢复体力"]
	var agent_list = _agents_data.keys()
	if not agent_list.is_empty():
		var random_agent = agent_list[randi_range(0, agent_list.size() - 1)]
		var agent_name = _agents_data[random_agent]["name"]
		var action = actions[randi_range(0, actions.size() - 1)]

		_events_log.append({
			"tick": _current_tick,
			"agent_name": agent_name,
			"event_type": "move",
			"description": "%s: %s" % [agent_name, action]
		})


func _emit_world_updated(snapshot: Dictionary) -> void:
	world_updated.emit(snapshot)
	for event in snapshot.get("events", []):
		narrative_event.emit(event)


# === 公开 API ===

## 启动模拟
func start() -> void:
	is_paused = false
	print("[SimulationBridge] 模拟开始")


## 暂停模拟
func pause() -> void:
	is_paused = true
	print("[SimulationBridge] 模拟暂停")


## 切换暂停状态
func toggle_pause() -> void:
	is_paused = !is_paused
	print("[SimulationBridge] 切换暂停状态：%s" % ("已暂停" if is_paused else "运行中"))


## 调整动机
func adjust_motivation(agent_id: String, dimension: int, value: float) -> void:
	print("[SimulationBridge] 调整动机：agent=%s, dim=%d, value=%.2f" % [agent_id, dimension, value])
	if _agents_data.has(agent_id):
		_agents_data[agent_id]["motivation"][dimension] = value


## 注入临时偏好
func inject_preference(agent_id: String, dimension: int, boost: float, duration: int) -> void:
	print("[SimulationBridge] 注入偏好：agent=%s, dim=%d, boost=%.2f, duration=%d" % [agent_id, dimension, boost, duration])
	# TODO: 实现临时偏好系统


## 设置 Tick 间隔
func set_tick_interval(seconds: float) -> void:
	tick_interval = seconds
	print("[SimulationBridge] 设置 tick 间隔=%.1f 秒" % seconds)


## 获取当前 tick
func get_current_tick() -> int:
	return _current_tick


## 获取 Agent 数据
func get_agent_data(agent_id: String) -> Dictionary:
	return _agents_data.get(agent_id, {})


## 获取所有 Agent 列表
func get_all_agents() -> Array:
	return _agents_data.keys()


## 获取叙事日志
func get_narrative_log() -> Array:
	return _events_log


## 选择 Agent
func select_agent(agent_id: String) -> void:
	selected_agent_id = agent_id
	agent_selected.emit(agent_id)
	print("[SimulationBridge] 选择 Agent: %s" % agent_id)


## 获取快照
func get_last_snapshot() -> Dictionary:
	return _last_snapshot


## 获取地图数据
func get_map_terrain(x: int, y: int) -> String:
	return _map_data.get("%d_%d" % [x, y], "plains")
