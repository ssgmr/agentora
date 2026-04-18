extends Node

func _ready():
	print("[AutoScreenshot] Auto screenshot loaded, waiting 10s...")
	await get_tree().create_timer(12.0).timeout
	print("[AutoScreenshot] Taking screenshot now...")
	var viewport = get_viewport()
	var img = viewport.get_texture().get_image()
	# 使用绝对路径直接保存
	var abs_path = "D:/work/code/rust/agentora/screenshot_godot.png"
	var err = img.save_png(abs_path)
	print("[AutoScreenshot] Saved to: ", abs_path, " err=", err)
	get_tree().quit()
