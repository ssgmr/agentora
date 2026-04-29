extends Control

# setup_wizard.gd - 引导页面脚本
# 检测用户配置，无配置显示强制 settings_panel，有配置跳转 main.tscn

@onready var bridge: Node = $SimulationBridge

func _ready():
	# 检查 Bridge 是否存在
	if not bridge:
		bridge = get_node_or_null("SimulationBridge")

	print("[SetupWizard] Bridge found: %s" % (bridge != null))

	# 检测配置并跳转
	_check_and_redirect()

func _check_and_redirect():
	if not bridge:
		print("[SetupWizard] Bridge not ready, waiting...")
		await get_tree().create_timer(0.5).timeout
		_check_and_redirect()
		return

	if bridge.has_method("has_user_config"):
		var has_config = bridge.has_user_config()
		print("[SetupWizard] has_user_config: %s" % has_config)

		if has_config:
			# 有配置，直接跳转主场景
			print("[SetupWizard] 已有配置，跳转主场景")
			get_tree().change_scene_to_file.call_deferred("res://scenes/main.tscn")
		else:
			# 无配置，显示强制 settings_panel
			print("[SetupWizard] 无配置，显示设置面板")
			_show_forced_settings_panel()
	else:
		print("[SetupWizard] Bridge 没有 has_user_config 方法")
		# 尝试直接加载配置判断
		if bridge.has_method("get_user_config"):
			var config = bridge.get_user_config()
			if config.size() > 0:
				print("[SetupWizard] 配置存在，跳转")
				get_tree().change_scene_to_file.call_deferred("res://scenes/main.tscn")
			else:
				_show_forced_settings_panel()
		else:
			# 无法判断，默认显示配置面板
			_show_forced_settings_panel()

func _show_forced_settings_panel():
	# 加载 settings_panel 场景
	var settings_scene = load("res://scenes/settings_panel.tscn")
	if not settings_scene:
		printerr("[SetupWizard] 无法加载 settings_panel.tscn")
		return

	var settings_panel_instance = settings_scene.instantiate()
	add_child(settings_panel_instance)

	# 设置强制模式
	if settings_panel_instance.has_method("set_forced_mode"):
		settings_panel_instance.set_forced_mode(true)

	# 显示面板
	settings_panel_instance.show()
	print("[SetupWizard] 强制配置面板已显示")