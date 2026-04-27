extends PanelContainer
## P2P 弹出面板
## 功能：种子地址输入、连接按钮、peer_id 展示、NAT 状态、已连接 peers 列表、订阅 topic 列表
## 通过 TopBar 的 P2P 按钮切换显示/隐藏

@onready var close_btn: Button = $VBox/CloseBtn
@onready var seed_address_input: LineEdit = $VBox/SeedAddressInput
@onready var connect_button: Button = $VBox/ConnectButton
@onready var peer_id_label: Label = $VBox/PeerIdLabel
@onready var nat_status_label: Label = $VBox/NatStatusLabel
@onready var peers_list: VBoxContainer = $VBox/PeersList
@onready var peers_label: Label = $VBox/PeersLabel
@onready var topics_list: VBoxContainer = $VBox/TopicsList
@onready var topics_label: Label = $VBox/TopicsLabel

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
		_update_nat_status_display(status, addr)
	else:
		nat_status_label.text = "NAT: 未知"

	# 获取已连接 peers 列表
	var peers_json = bridge.get_connected_peers()
	_update_peers_list(peers_json)

	# 获取订阅的 topic 列表（新增）
	if bridge.has_method("get_subscribed_topics"):
		var topics_json = bridge.get_subscribed_topics()
		_update_topics_list(topics_json)


func _update_peer_id():
	if bridge == null:
		return

	var pid = bridge.get_peer_id()
	if pid and not pid.is_empty():
		peer_id_label.text = "Peer ID: " + pid
	else:
		peer_id_label.text = "Peer ID: 未获取"


func _update_nat_status_display(status: String, addr: String):
	var text = ""
	var color = Color.WHITE

	if status == "public":
		text = "NAT: 公网可达"
		if not addr.is_empty():
			text += " (" + addr + ")"
		color = Color(0.2, 0.8, 0.2, 1.0)  # 绿色
	elif status == "private":
		text = "NAT: 内网（需要中继或打洞）"
		color = Color(0.9, 0.7, 0.2, 1.0)  # 黄色
	elif status == "unknown":
		text = "NAT: 正在探测..."
		color = Color(0.7, 0.7, 0.7, 1.0)  # 灰色
	elif status == "disabled":
		text = "NAT: 未启用（中心化模式）"
		color = Color(0.5, 0.5, 0.5, 1.0)  # 灰色
	else:
		text = "NAT: " + status
		if not addr.is_empty():
			text += " (" + addr + ")"

	nat_status_label.text = text
	nat_status_label.modulate = color


func _update_peers_list(json_str):
	# 清空现有列表
	for child in peers_list.get_children():
		child.queue_free()

	if json_str.is_empty() or json_str == "[]":
		peers_label.text = "已连接节点 (0):"
		var label = Label.new()
		label.text = "无已连接节点"
		label.modulate = Color(0.5, 0.5, 0.5, 1.0)
		peers_list.add_child(label)
		return

	# 解析 JSON
	var result = JSON.parse_string(json_str)
	if typeof(result) == TYPE_ARRAY:
		peers_label.text = "已连接节点 (" + str(result.size()) + "):"

		for peer in result:
			var peer_id = peer.get("peer_id", "unknown")
			var agent_version = peer.get("agent_version", "")
			var connection_type = peer.get("connection_type", "unknown")
			var connected_at = peer.get("connected_at", "")
			var is_relay_server = peer.get("is_relay_server", false)

			# 创建节点信息容器
			var peer_vbox = VBoxContainer.new()
			peer_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL

			# 第一行：PeerId + 类型标签
			var row1 = HBoxContainer.new()
			row1.size_flags_horizontal = Control.SIZE_EXPAND_FILL

			var peer_id_lbl = Label.new()
			peer_id_lbl.text = peer_id  # 完整 PeerId
			peer_id_lbl.size_flags_horizontal = Control.SIZE_EXPAND_FILL

			var type_lbl = Label.new()
			if is_relay_server:
				type_lbl.text = "[中继]"
				type_lbl.modulate = Color(0.9, 0.7, 0.2, 1.0)  # 黄色
			else:
				type_lbl.text = "[玩家]"
				type_lbl.modulate = Color(0.2, 0.8, 0.2, 1.0)  # 绿色

			row1.add_child(peer_id_lbl)
			row1.add_child(type_lbl)
			peer_vbox.add_child(row1)

			# 第二行：agent_version
			if not agent_version.is_empty():
				var agent_lbl = Label.new()
				agent_lbl.text = "  agent: " + agent_version
				agent_lbl.modulate = Color(0.7, 0.7, 0.7, 1.0)
				peer_vbox.add_child(agent_lbl)

			# 第三行：连接方式 + 时间
			var row3 = Label.new()
			var conn_text = "  连接方式: " + _connection_type_display(connection_type)
			if not connected_at.is_empty():
				# 提取时间部分（去掉日期）
				var time_part = connected_at
				if connected_at.contains("T"):
					time_part = connected_at.split("T")[1]
					if time_part.contains("."):
						time_part = time_part.split(".")[0]
				conn_text += " | 时间: " + time_part
			row3.text = conn_text
			row3.modulate = Color(0.7, 0.7, 0.7, 1.0)
			peer_vbox.add_child(row3)

			# 添加分隔线
			var sep = HSeparator.new()
			sep.modulate = Color(0.3, 0.3, 0.3, 1.0)
			peer_vbox.add_child(sep)

			peers_list.add_child(peer_vbox)
	else:
		peers_label.text = "已连接节点 (解析失败)"


func _connection_type_display(conn_type: String) -> String:
	if conn_type == "Direct":
		return "直连"
	elif conn_type == "Relay":
		return "中继"
	elif conn_type == "Dcutr":
		return "打洞"
	else:
		return conn_type


func _update_topics_list(json_str):
	# 清空现有列表
	for child in topics_list.get_children():
		child.queue_free()

	if json_str.is_empty() or json_str == "[]":
		topics_label.text = "订阅的 Topic (0):"
		var label = Label.new()
		label.text = "暂无订阅的 Topic"
		label.modulate = Color(0.5, 0.5, 0.5, 1.0)
		topics_list.add_child(label)
		return

	# 解析 JSON
	var result = JSON.parse_string(json_str)
	if typeof(result) == TYPE_ARRAY:
		topics_label.text = "订阅的 Topic (" + str(result.size()) + "):"

		for topic in result:
			var label = Label.new()
			label.text = "  ✅ " + str(topic)
			label.modulate = Color(0.2, 0.8, 0.2, 1.0)
			topics_list.add_child(label)
	else:
		topics_label.text = "订阅的 Topic (解析失败)"


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