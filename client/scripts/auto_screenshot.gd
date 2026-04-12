extends Node

func _ready():
	print("[AutoScreenshot] Auto screenshot loaded, waiting 30s...")
	await get_tree().create_timer(30.0).timeout
	print("[AutoScreenshot] Taking screenshot now...")
	var viewport = get_viewport()
	var img = viewport.get_texture().get_image()
	var abs_path = "D:/work/code/rust/agentora/screenshot_godot.png"
	var err = img.save_png(abs_path)
	print("[AutoScreenshot] Saved to: ", abs_path, " err=", err)
	get_tree().quit()
