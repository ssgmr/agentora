# ChannelSelector - 叙事频道选择器
# 支持切换 Local/Nearby/World 三个频道
extends HBoxContainer

# 信号
signal channel_changed(channel: String)

# UI 组件
var _local_btn: Button
var _nearby_btn: Button
var _world_btn: Button

# 当前频道（默认本地，Centralized 模式兼容）
var _current_channel: String = "local"


func _ready() -> void:
	_setup_ui()
	_update_button_states()

	# 订阅 StateManager 信号
	StateManager.filter_changed.connect(_on_filter_changed)


func _setup_ui() -> void:
	# 创建频道标签
	var label = Label.new()
	label.text = "频道:"
	add_child(label)

	# 创建三个频道按钮
	_local_btn = Button.new()
	_local_btn.text = "📋本地"
	_local_btn.toggle_mode = true
	_local_btn.custom_minimum_size = Vector2(70, 28)
	add_child(_local_btn)
	_local_btn.pressed.connect(_on_local_pressed)

	_nearby_btn = Button.new()
	_nearby_btn.text = "📍附近"
	_nearby_btn.toggle_mode = true
	_nearby_btn.custom_minimum_size = Vector2(70, 28)
	add_child(_nearby_btn)
	_nearby_btn.pressed.connect(_on_nearby_pressed)

	_world_btn = Button.new()
	_world_btn.text = "🌍世界"
	_world_btn.toggle_mode = true
	_world_btn.custom_minimum_size = Vector2(70, 28)
	add_child(_world_btn)
	_world_btn.pressed.connect(_on_world_pressed)


func _update_button_states() -> void:
	_local_btn.button_pressed = (_current_channel == "local")
	_nearby_btn.button_pressed = (_current_channel == "nearby")
	_world_btn.button_pressed = (_current_channel == "world")


func _set_channel(channel: String) -> void:
	_current_channel = channel
	_update_button_states()
	StateManager.set_narrative_channel(channel)
	channel_changed.emit(channel)


func _on_local_pressed() -> void:
	_set_channel("local")


func _on_nearby_pressed() -> void:
	_set_channel("nearby")


func _on_world_pressed() -> void:
	_set_channel("world")


func _on_filter_changed() -> void:
	var manager_channel = StateManager.get_narrative_channel()
	if manager_channel != _current_channel:
		_current_channel = manager_channel
		_update_button_states()


## 外部调用：设置频道
func set_channel(channel: String) -> void:
	if channel in ["local", "nearby", "world"]:
		_set_channel(channel)