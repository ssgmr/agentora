# BridgeAccessor - SimulationBridge 统一获取器
#
# 解决问题：多个脚本需要获取 SimulationBridge 节点
# 之前有5种不同写法的路径硬编码：
# - "../../SimulationBridge"
# - "../SimulationBridge"
# - "Main/SimulationBridge"
# - "/root/Main/SimulationBridge"
# - get_node_or_null("../SimulationBridge")
#
# 使用方法：
#   var bridge = BridgeAccessor.get_bridge()
#   bridge.start()
#   bridge.pause()

extends Node

# 静态缓存，避免每次查找
static var _bridge: Node = null

## 获取 SimulationBridge 节点
## 返回：SimulationBridge 节点或 null（如果未找到）
static func get_bridge() -> Node:
	if _bridge != null:
		return _bridge

	# 尝试多种路径查找
	var paths := [
		"/root/Main/SimulationBridge",  # 绝对路径（推荐）
		"Main/SimulationBridge",        # 相对于 root
	]

	for path in paths:
		var node: Node = Engine.get_main_loop().root.get_node_or_null(path)
		if node != null:
			_bridge = node
			return node

	# 未找到
	push_warning("[BridgeAccessor] SimulationBridge not found, tried paths: " + str(paths))
	return null


## 重置缓存（场景切换时调用）
static func reset() -> void:
	_bridge = null


## 检查 Bridge 是否可用
static func is_available() -> bool:
	return get_bridge() != null