extends PanelContainer
## P2P 弹出面板
## 功能：种子地址输入、连接按钮、peer_id 展示、NAT 状态、已连接 peers 列表
## 通过 TopBar 的 P2P 按钮切换显示/隐藏

@onready var close_btn: Button = $VBox/CloseBtn
@onready var seed_address_input: LineEdit = $VBox/SeedAddressInput
@onready var connect_button: Button = $VBox/ConnectButton
@onready var peer_id_label: Label = $VBox/PeerIdLabel
@onready var nat_status_label: Label = $VBox/NatStatusLabel
@onready var peers_list: VBoxContainer = $VBox/PeersList
@onready var peers_label: Label = $VBox/PeersLabel

var bridge = null
var refresh_timer: Timer = null


func _ready():
	# 绑定关闭按钮和连接按钮
	close_btn.pressed.connect(_toggle_visibility)
	connect_button.pressed.connect(_on_connect_button_pressed)

	# 延迟获取 Bridge 引用
	await get_tree().create_timer(1.0).timeout
	bridge = _find_bridge()

	if bridge == null:
		peer_id_label.text = "P2P: 未启用"
		nat_status_label.text = "状态: 未初始化"
		connect_button.disabled = true
		seed_address_input.editable = false
		return

	# 连接 Bridge 信号
	if bridge.has_signal("peer_connected"):
		bridge.peer_connected.connect(_on_peer_connected)
	if bridge.has_signal("p2p_status_changed"):
		bridge.p2p_status_changed.connect(_on_p2p_status_changed)

	# 启动定时刷新
	refresh_timer = Timer.new()
	refresh_timer.wait_time = 3.0
	refresh_timer.autostart = true
	refresh_timer.one_shot = false
	refresh_timer.timeout.connect(_refresh_peer_info)
	add_child(refresh_timer)

	# 显示初始状态
	_update_peer_id()
	_refresh_peer_info()


signal on_closed

func _toggle_visibility():
	visible = !visible
	if not visible:
		on_closed.emit()


func toggle():
	_toggle_visibility()


func _find_bridge():
	# 尝试从场景树中找到 SimulationBridge 节点
	var main = get_tree().get_root().get_node_or_null("Main")
	if main:
		return main.get_node_or_null("SimulationBridge")
	return null


func _on_connect_button_pressed():
	if bridge == null:
		return

	var address = seed_address_input.text.strip_edges()
	if address.is_empty():
		peer_id_label.text = "P2P: 请输入种子地址"
		return

	var success = bridge.connect_to_seed(address)
	if success:
		connect_button.text = "已连接"
		connect_button.disabled = true
		peer_id_label.text = "P2P: 正在连接..."
	else:
		peer_id_label.text = "P2P: 连接请求失败"


func _refresh_peer_info():
	if bridge == null:
		return

	_update_peer_id()

	# 获取 NAT 状态
	var nat_dict = bridge.get_nat_status()
	if nat_dict and nat_dict.has("status"):
		var status = nat_dict["status"]
		var addr = nat_dict.get("address", "")
		nat_status_label.text = "NAT: " + status + (" (" + addr + ")" if not addr.is_empty() else "")
	else:
		nat_status_label.text = "NAT: 未知"

	# 获取已连接 peers 列表
	var peers_json = bridge.get_connected_peers()
	_update_peers_list(peers_json)


func _update_peer_id():
	if bridge == null:
		return

	var pid = bridge.get_peer_id()
	if pid and not pid.is_empty():
		peer_id_label.text = "Peer ID: " + pid
	else:
		peer_id_label.text = "Peer ID: 未获取"


func _update_peers_list(json_str):
	# 清空现有列表
	for child in peers_list.get_children():
		child.queue_free()

	if json_str.is_empty() or json_str == "[]":
		peers_label.text = "已连接 peers (0):"
		var label = Label.new()
		label.text = "无已连接 peer"
		label.modulate = Color(0.5, 0.5, 0.5, 1.0)
		peers_list.add_child(label)
		return

	# 简单解析 JSON（Godot 内置 JSON 解析）
	var result = JSON.parse_string(json_str)
	if typeof(result) == TYPE_ARRAY:
		peers_label.text = "已连接 peers (" + str(result.size()) + "):"
		for peer in result:
			var label = Label.new()
			var peer_id = peer.get("peer_id", "unknown")
			var conn_type = peer.get("connection_type", "unknown")
			label.text = "  - " + peer_id + " [" + conn_type + "]"
			peers_list.add_child(label)
	else:
		peers_label.text = "已连接 peers (解析失败)"


func _on_peer_connected(peer_id):
	var current = peer_id_label.text
	peer_id_label.text = current + " | 新连接: " + peer_id
	_refresh_peer_info()


func _on_p2p_status_changed(status_dict):
	if typeof(status_dict) == TYPE_DICTIONARY:
		var nat = status_dict.get("nat_status", "未知")
		var count = status_dict.get("peer_count", 0)
		var error = status_dict.get("error", "")
		nat_status_label.text = "NAT: " + nat + " | Peers: " + str(count)
		if not error.is_empty():
			nat_status_label.text += " | 错误: " + error
