extends Node
func _ready():
    print("[AutoScreenshot] Waiting 15s...")
    await get_tree().create_timer(15.0).timeout
    print("[AutoScreenshot] Taking screenshot...")
    var viewport = get_viewport()
    var img = viewport.get_texture().get_image()
    img.save_png("D:/work/code/rust/agentora/screenshot_godot.png")
    print("[AutoScreenshot] Saved!")
    get_tree().quit()
