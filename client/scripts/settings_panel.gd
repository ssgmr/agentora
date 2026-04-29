extends PanelContainer

# settings_panel.gd - 游戏内设置面板（合并 setup_wizard）
# 使用静态节点引用，支持完整配置项修改

# Bridge 引用
var bridge: Node

# === UI 节点引用（@onready） ===
@onready var local_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/LLMHBox/LocalBtn
@onready var remote_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/LLMHBox/RemoteBtn
@onready var rule_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/LLMHBox/RuleBtn

# 远程配置区
@onready var remote_config_container: VBoxContainer = $PanelContainer/MarginContainer/ScrollContainer/VBox/RemoteConfigContainer
@onready var endpoint_input: LineEdit = $PanelContainer/MarginContainer/ScrollContainer/VBox/RemoteConfigContainer/EndpointInput
@onready var token_input: LineEdit = $PanelContainer/MarginContainer/ScrollContainer/VBox/RemoteConfigContainer/TokenInput
@onready var model_input: LineEdit = $PanelContainer/MarginContainer/ScrollContainer/VBox/RemoteConfigContainer/ModelInput

# 本地模型配置区
@onready var local_model_container: VBoxContainer = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer
@onready var qwen2b_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/ModelsGrid/Qwen2BBtn
@onready var qwen4b_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/ModelsGrid/Qwen4BBtn
@onready var gemma2b_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/ModelsGrid/Gemma2BBtn
@onready var local_file_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/ModelsGrid/LocalFileBtn

# 下载进度 UI
@onready var download_progress_container: VBoxContainer = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/DownloadProgressContainer
@onready var download_status_label: Label = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/DownloadProgressContainer/DownloadStatusLabel
@onready var download_progress_bar: ProgressBar = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/DownloadProgressContainer/DownloadProgressBar
@onready var cancel_download_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/DownloadProgressContainer/CancelDownloadBtn

# 模型加载状态 UI
@onready var load_status_container: VBoxContainer = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/LoadStatusContainer
@onready var load_status_label: Label = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/LoadStatusContainer/LoadStatusLabel
@onready var load_progress_bar: ProgressBar = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/LoadStatusContainer/LoadProgressBar
@onready var backend_info_label: Label = $PanelContainer/MarginContainer/ScrollContainer/VBox/LocalModelContainer/LoadStatusContainer/BackendInfoLabel

@onready var agent_name_input: LineEdit = $PanelContainer/MarginContainer/ScrollContainer/VBox/AgentNameInput
@onready var prompt_input: TextEdit = $PanelContainer/MarginContainer/ScrollContainer/VBox/PromptInput

@onready var icon_default: TextureButton = $PanelContainer/MarginContainer/ScrollContainer/VBox/IconGrid/IconDefault
@onready var icon_wizard: TextureButton = $PanelContainer/MarginContainer/ScrollContainer/VBox/IconGrid/IconWizard
@onready var icon_fox: TextureButton = $PanelContainer/MarginContainer/ScrollContainer/VBox/IconGrid/IconFox
@onready var icon_dragon: TextureButton = $PanelContainer/MarginContainer/ScrollContainer/VBox/IconGrid/IconDragon
@onready var icon_lion: TextureButton = $PanelContainer/MarginContainer/ScrollContainer/VBox/IconGrid/IconLion
@onready var icon_robot: TextureButton = $PanelContainer/MarginContainer/ScrollContainer/VBox/IconGrid/IconRobot

@onready var single_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/P2PHBox/SingleBtn
@onready var create_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/P2PHBox/CreateBtn
@onready var join_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/P2PHBox/JoinBtn
@onready var seed_address_input: LineEdit = $PanelContainer/MarginContainer/ScrollContainer/VBox/SeedAddressInput

@onready var restart_label: Label = $PanelContainer/MarginContainer/ScrollContainer/VBox/RestartLabel
@onready var save_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/BtnHBox/SaveBtn
@onready var close_btn: Button = $PanelContainer/MarginContainer/ScrollContainer/VBox/BtnHBox/CloseBtn

# === 配置状态 ===
var current_llm_mode: String = "rule_only"
var current_icon_id: String = "default"
var current_p2p_mode: String = "single"
var restart_required: bool = false
var is_forced_mode: bool = false  # 强制模式（首次配置）

# === 下载/加载状态 ===
var current_download_model: String = ""  # 当前下载的模型名称
var is_downloading: bool = false
var is_loading: bool = false

# 图标 ID 映射
var icon_ids: Array = ["default", "wizard", "fox", "dragon", "lion", "robot"]
var icon_buttons: Array = []

func _ready():
	# 初始化图标按钮数组
	icon_buttons = [icon_default, icon_wizard, icon_fox, icon_dragon, icon_lion, icon_robot]

	# 获取 Bridge 引用
	bridge = get_node_or_null("../SimulationBridge")
	if not bridge:
		var setup = get_tree().root.get_node_or_null("SetupWizard")
		if setup:
			bridge = setup.get_node_or_null("SimulationBridge")

	# 应用共享样式
	_apply_styles()

	# 加载图标纹理
	_load_icon_textures()

	# 连接按钮事件
	_connect_signals()

	# 加载当前配置
	load_config()

func _apply_styles():
	# LLM 模式按钮
	SharedUIScripts.apply_toggle_button_style(local_btn)
	SharedUIScripts.apply_toggle_button_style(remote_btn)
	SharedUIScripts.apply_toggle_button_style(rule_btn)

	# 远程配置输入框
	SharedUIScripts.apply_input_style(endpoint_input)
	SharedUIScripts.apply_input_style(token_input)
	SharedUIScripts.apply_input_style(model_input)

	# 本地模型按钮
	for btn in [qwen2b_btn, qwen4b_btn, gemma2b_btn, local_file_btn]:
		SharedUIScripts.apply_button_style(btn)

	# 下载进度 UI
	SharedUIScripts.apply_button_style(cancel_download_btn, "danger")
	# ProgressBar 通常使用主题默认样式

	# Agent 配置输入框
	SharedUIScripts.apply_input_style(agent_name_input)
	SharedUIScripts.apply_textedit_style(prompt_input)

	# 图标按钮
	for btn in icon_buttons:
		SharedUIScripts.apply_icon_button_style(btn)

	# P2P 模式按钮
	SharedUIScripts.apply_toggle_button_style(single_btn)
	SharedUIScripts.apply_toggle_button_style(create_btn)
	SharedUIScripts.apply_toggle_button_style(join_btn)
	SharedUIScripts.apply_input_style(seed_address_input)

	# 保存/关闭按钮
	SharedUIScripts.apply_button_style(save_btn, "success")
	SharedUIScripts.apply_button_style(close_btn)

func _load_icon_textures():
	var icon_paths := {
		"default": "res://assets/textures/agents/default.png",
		"wizard": "res://assets/textures/agents/wizard.png",
		"fox": "res://assets/textures/agents/fox.png",
		"dragon": "res://assets/textures/agents/dragon.png",
		"lion": "res://assets/textures/agents/lion.png",
		"robot": "res://assets/textures/agents/robot.png",
	}

	for i in range(icon_ids.size()):
		var icon_id = icon_ids[i]
		var btn = icon_buttons[i]
		var path = icon_paths.get(icon_id, "")
		if ResourceLoader.exists(path):
			btn.texture_normal = load(path)

func _connect_signals():
	# LLM 模式按钮
	local_btn.pressed.connect(_on_llm_mode_changed.bind("local"))
	remote_btn.pressed.connect(_on_llm_mode_changed.bind("remote"))
	rule_btn.pressed.connect(_on_llm_mode_changed.bind("rule_only"))

	# 本地模型下载按钮
	qwen2b_btn.pressed.connect(_on_model_download_requested.bind(
		"https://www.modelscope.cn/models/unsloth/Qwen3.5-2B-GGUF/resolve/master/Qwen3.5-2B-Q4_K_M.gguf",
		"Qwen3.5-2B"
	))
	qwen4b_btn.pressed.connect(_on_model_download_requested.bind(
		"https://www.modelscope.cn/models/unsloth/Qwen3.5-4B-GGUF/resolve/master/Qwen3.5-4B-Q4_K_M.gguf",
		"Qwen3.5-4B"
	))
	gemma2b_btn.pressed.connect(_on_model_download_requested.bind(
		"https://www.modelscope.cn/models/AI-ModelScope/gemma-2-2b-it-GGUF/resolve/master/gemma-2-2b-it-q4_k_m.gguf",
		"Gemma-2-2B"
	))
	local_file_btn.pressed.connect(_on_select_local_model)

	# 图标按钮
	for i in range(icon_ids.size()):
		var btn = icon_buttons[i]
		var icon_id = icon_ids[i]
		btn.pressed.connect(_on_icon_selected.bind(icon_id))

	# P2P 模式按钮
	single_btn.pressed.connect(_on_p2p_mode_changed.bind("single"))
	create_btn.pressed.connect(_on_p2p_mode_changed.bind("create"))
	join_btn.pressed.connect(_on_p2p_mode_changed.bind("join"))

	# 保存/关闭按钮
	save_btn.pressed.connect(_on_save_pressed)
	close_btn.pressed.connect(_on_close_pressed)

	# 取消下载按钮
	cancel_download_btn.pressed.connect(_on_cancel_download_pressed)

	# 连接 Bridge 信号
	_connect_bridge_signals()

func load_config():
	if not bridge or not bridge.has_method("get_user_config"):
		print("[SettingsPanel] Bridge not available")
		return

	var config = bridge.get_user_config()
	print("[SettingsPanel] Loaded config: %s" % str(config.keys()))

	# LLM 模式
	current_llm_mode = config.get("llm_mode", "rule_only")
	_update_llm_mode_ui()

	# 远程配置
	endpoint_input.text = config.get("llm_api_endpoint", "")
	token_input.text = config.get("llm_api_token", "")
	model_input.text = config.get("llm_model_name", "")

	# Agent 名字
	agent_name_input.text = config.get("agent_name", "智行者")

	# 自定义提示词
	prompt_input.text = config.get("agent_custom_prompt", "")

	# 图标
	current_icon_id = config.get("agent_icon_id", "default")
	_update_icon_ui()

	# P2P 模式
	current_p2p_mode = config.get("p2p_mode", "single")
	seed_address_input.text = config.get("p2p_seed_address", "")
	_update_p2p_mode_ui()

	# 清除重启提示
	restart_required = false
	restart_label.text = ""

func set_forced_mode(forced: bool):
	"""设置强制模式（首次配置，不可关闭）"""
	is_forced_mode = forced
	if forced:
		close_btn.hide()
		# 可选：修改标题
		var title_label = $PanelContainer/MarginContainer/ScrollContainer/VBox/TitleLabel
		if title_label:
			title_label.text = "首次配置"

func _on_llm_mode_changed(mode: String):
	if mode != current_llm_mode:
		restart_required = true
		restart_label.text = "更改 LLM 模式需要重启生效"
		restart_label.modulate = Color.YELLOW
	current_llm_mode = mode
	_update_llm_mode_ui()

func _update_llm_mode_ui():
	local_btn.button_pressed = (current_llm_mode == "local")
	remote_btn.button_pressed = (current_llm_mode == "remote")
	rule_btn.button_pressed = (current_llm_mode == "rule_only")

	# 条件显示配置区域
	remote_config_container.visible = (current_llm_mode == "remote")
	local_model_container.visible = (current_llm_mode == "local")

func _on_model_download_requested(url: String, name: String):
	print("[SettingsPanel] Model download requested: %s" % name)

	# 设置当前下载模型名称
	current_download_model = name

	# 计算下载路径
	var models_dir := "models/"
	var dest_path := models_dir + name + ".gguf"

	# 调用 Bridge.download_model()
	if bridge and bridge.has_method("download_model"):
		var success = bridge.download_model(name, url, dest_path)
		if success:
			# 显示进度条容器
			download_progress_container.visible = true
			download_status_label.text = "开始下载: %s..." % name
			download_progress_bar.value = 0.0
			is_downloading = true
		else:
			_show_message("下载启动失败")
	else:
		_show_message("Bridge.download_model() 方法不可用")

func _on_select_local_model():
	print("[SettingsPanel] Select local model")
	_show_message("请手动将模型文件放入 models/ 目录")

func _on_icon_selected(icon_id: String):
	current_icon_id = icon_id
	_update_icon_ui()

func _update_icon_ui():
	for i in range(icon_ids.size()):
		var btn = icon_buttons[i]
		var icon_id = icon_ids[i]
		if icon_id == current_icon_id:
			btn.modulate = Color(1.0, 1.0, 1.0, 1.0)
		else:
			btn.modulate = Color(0.6, 0.6, 0.6, 1.0)

func _on_p2p_mode_changed(mode: String):
	current_p2p_mode = mode
	_update_p2p_mode_ui()

func _update_p2p_mode_ui():
	single_btn.button_pressed = (current_p2p_mode == "single")
	create_btn.button_pressed = (current_p2p_mode == "create")
	join_btn.button_pressed = (current_p2p_mode == "join")

	# 显示/隐藏种子地址输入框
	seed_address_input.visible = (current_p2p_mode == "join")

func _on_save_pressed():
	var agent_name = agent_name_input.text.strip_edges()

	# 验证 Agent 名字
	if agent_name.is_empty():
		restart_label.text = "Agent 名字不能为空！"
		restart_label.modulate = Color.RED
		return

	# remote 模式验证
	if current_llm_mode == "remote":
		var endpoint = endpoint_input.text.strip_edges()
		if endpoint.is_empty():
			restart_label.text = "远程模式需要输入 API Endpoint！"
			restart_label.modulate = Color.RED
			return

	# join 模式验证
	if current_p2p_mode == "join":
		var seed_address = seed_address_input.text.strip_edges()
		if seed_address.is_empty():
			restart_label.text = "加入模式需要输入种子节点地址！"
			restart_label.modulate = Color.RED
			return

	# 收集配置
	var config = {
		"llm_mode": current_llm_mode,
		"llm_api_endpoint": endpoint_input.text.strip_edges() if current_llm_mode == "remote" else "",
		"llm_api_token": token_input.text.strip_edges() if current_llm_mode == "remote" else "",
		"llm_model_name": model_input.text.strip_edges() if current_llm_mode == "remote" else "",
		"llm_local_model_path": "",
		"agent_name": agent_name,
		"agent_custom_prompt": prompt_input.text.strip_edges(),
		"agent_icon_id": current_icon_id,
		"agent_custom_icon_path": "",
		"p2p_mode": current_p2p_mode,
		"p2p_seed_address": seed_address_input.text.strip_edges()
	}

	# 保存配置
	if bridge and bridge.has_method("set_user_config"):
		var success = bridge.set_user_config(config)
		if success:
			print("[SettingsPanel] Config saved successfully")
			if is_forced_mode:
				# 强制模式：跳转到主场景
				get_tree().change_scene_to_file.call_deferred("res://scenes/main.tscn")
			else:
				if restart_required:
					restart_label.text = "配置已保存，请重启游戏"
					restart_label.modulate = Color.YELLOW
				else:
					restart_label.text = "配置已保存"
					restart_label.modulate = Color.GREEN
		else:
			restart_label.text = "配置保存失败"
			restart_label.modulate = Color.RED
	else:
		restart_label.text = "Bridge 未就绪"
		restart_label.modulate = Color.RED

func _on_close_pressed():
	hide()

func show_panel():
	show()
	load_config()

func _show_message(text: String):
	var dialog = AcceptDialog.new()
	dialog.dialog_text = text
	dialog.title = "提示"
	add_child(dialog)
	dialog.popup_centered()

# === Bridge 信号连接 ===

func _connect_bridge_signals():
	if not bridge:
		return

	# 下载进度信号
	if bridge.has_signal("download_progress"):
		bridge.download_progress.connect(_on_download_progress)

	# 下载完成信号
	if bridge.has_signal("model_download_complete"):
		bridge.model_download_complete.connect(_on_download_complete)

	# 下载失败信号
	if bridge.has_signal("model_download_failed"):
		bridge.model_download_failed.connect(_on_download_failed)

	# 模型加载开始信号
	if bridge.has_signal("model_load_start"):
		bridge.model_load_start.connect(_on_model_load_start)

	# 模型加载进度信号
	if bridge.has_signal("model_load_progress"):
		bridge.model_load_progress.connect(_on_model_load_progress)

	# 模型加载完成信号
	if bridge.has_signal("model_load_complete"):
		bridge.model_load_complete.connect(_on_model_load_complete)

	# 模型加载失败信号
	if bridge.has_signal("model_load_failed"):
		bridge.model_load_failed.connect(_on_model_load_failed)

# === 下载信号处理 ===

func _on_download_progress(model_name: String, downloaded_mb: float, total_mb: float, speed_mbps: float):
	if model_name != current_download_model:
		return

	is_downloading = true
	download_progress_container.visible = true

	var percent = (downloaded_mb / total_mb) * 100.0 if total_mb > 0 else 0.0
	download_progress_bar.value = percent
	download_status_label.text = "下载中: %.1f MB / %.1f MB (%.1%%) %.1f MB/s" % [downloaded_mb, total_mb, percent, speed_mbps]

func _on_download_complete(path: String):
	is_downloading = false
	download_progress_container.visible = false
	current_download_model = ""

	_show_message("模型下载完成！路径: %s\n点击保存后重启生效。" % path)

func _on_download_failed(error: String):
	is_downloading = false
	download_progress_container.visible = false
	current_download_model = ""

	_show_message("模型下载失败: %s" % error)

# === 加载信号处理 ===

func _on_model_load_start(model_name: String, estimated_time_ms: int):
	is_loading = true
	load_status_container.visible = true
	load_status_label.text = "模型加载中: %s (估算 %.1f 秒)" % [model_name, estimated_time_ms / 1000.0]
	load_progress_bar.value = 0.0
	backend_info_label.text = "检测 GPU 后端..."

func _on_model_load_progress(phase: String, progress: float, model_name: String):
	if not is_loading:
		return

	load_progress_bar.value = progress

	var phase_text := ""
	if phase == "reading":
		phase_text = "读取模型文件..."
	elif phase == "parsing":
		phase_text = "解析模型权重..."
	elif phase == "gpu_upload":
		phase_text = "上传到 GPU..."

	load_status_label.text = "加载中: %s (%.0f%%) %s" % [model_name, progress, phase_text]

func _on_model_load_complete(model_name: String, backend: String, memory_mb: int):
	is_loading = false
	load_status_container.visible = false

	var backend_text := ""
	if backend == "metal":
		backend_text = "Apple Metal GPU"
	elif backend == "vulkan":
		backend_text = "Vulkan GPU"
	elif backend == "cuda":
		backend_text = "NVIDIA CUDA GPU"
	elif backend == "cpu":
		backend_text = "CPU (无 GPU 加速)"
	else:
		backend_text = backend

	_show_message("模型加载完成！\n后端: %s\n内存占用: %d MB" % [backend_text, memory_mb])

func _on_model_load_failed(model_name: String, error: String):
	is_loading = false
	load_status_container.visible = false

	_show_message("模型加载失败: %s\n将使用规则引擎作为兜底。" % error)

# === 取消下载 ===

func _on_cancel_download_pressed():
	if bridge and bridge.has_method("cancel_download"):
		bridge.cancel_download()

	is_downloading = false
	download_progress_container.visible = false
	current_download_model = ""
	download_status_label.text = "下载已取消"