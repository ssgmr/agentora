extends Node

func _ready():
	print("[AutoScreenshot] Auto screenshot loaded, waiting 10s...")
	await get_tree().create_timer(12.0).timeout
	print("[AutoScreenshot] Taking screenshot now...")
	var viewport = get_viewport()
	var img = viewport.get_texture().get_image()
	# 动态获取项目根目录路径（跨平台兼容）
	var project_root = ProjectSettings.globalize_path("res://").rstrip("/")
	var screenshot_path = project_root + "/screenshot_godot.png"
	var err = img.save_png(screenshot_path)
	print("[AutoScreenshot] Saved to: ", screenshot_path, " err=", err)
	get_tree().quit()
