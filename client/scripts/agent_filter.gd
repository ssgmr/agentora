# AgentFilter - Agent 选择器
# 支持下拉选择 Agent 和点击 Agent Sprite 自动选中
extends HBoxContainer

# 信号
signal agent_selected(agent_id: String)

# UI 组件
var _option_button: OptionButton
var _clear_button: Button

# 当前选中的 Agent ID
var selected_agent_id: String = ""

# 可用 Agent 列表缓存
var _available_agents: Dictionary = {}


func _ready() -> void:
	_setup_ui()

	# 订阅 StateManager 信号
	StateManager.state_updated.connect(_on_state_updated)
	StateManager.agent_changed.connect(_on_agent_changed)
	StateManager.filter_changed.connect(_on_filter_changed)

	# 初始化 Agent 列表
	_update_agent_list()


func _setup_ui() -> void:
	# 创建下拉选择器
	_option_button = OptionButton.new()
	_option_button.custom_minimum_size = Vector2(150, 30)
	add_child(_option_button)

	# 连接选择信号
	_option_button.item_selected.connect(_on_option_selected)

	# 创建清除按钮
	_clear_button = Button.new()
	_clear_button.text = "X"
	_clear_button.custom_minimum_size = Vector2(30, 30)
	_clear_button.tooltip_text = "清除过滤"
	add_child(_clear_button)

	# 连接清除信号
	_clear_button.pressed.connect(_clear_filter)


func _update_agent_list() -> void:
	# 清空现有选项
	_option_button.clear()

	# 添加"全部Agent"选项（默认）
	_option_button.add_item("全部Agent")
	_option_button.set_item_metadata(0, "")

	# 获取所有 Agent
	var agents = StateManager.get_all_agents()
	_available_agents = agents

	# 添加每个 Agent 选项
	var idx = 1
	for agent_id in agents.keys():
		var agent_data = agents[agent_id]
		var name = agent_data.get("name", agent_id)
		var is_alive = agent_data.get("is_alive", true)

		# 来源标记（本地/远程）
		var source_icon = "📋"  # 本地

		# 显示名称（带来源标记）
		var display_name = source_icon + " " + name

		# 如果已死亡，添加标记
		if not is_alive:
			display_name = "💀 " + display_name

		_option_button.add_item(display_name)
		_option_button.set_item_metadata(idx, agent_id)
		idx += 1

	# 恢复选中状态
	_restore_selection()


func _restore_selection() -> void:
	if selected_agent_id == "":
		_option_button.select(0)
		return

	# 查找对应的索引
	for i in range(_option_button.get_item_count()):
		var meta = _option_button.get_item_metadata(i)
		if meta == selected_agent_id:
			_option_button.select(i)
			return

	# 如果未找到，选择"全部"
	_option_button.select(0)


func _on_option_selected(index: int) -> void:
	var agent_id = _option_button.get_item_metadata(index)

	selected_agent_id = agent_id
	StateManager.set_narrative_agent_filter(agent_id)
	agent_selected.emit(agent_id)


func _clear_filter() -> void:
	selected_agent_id = ""
	_option_button.select(0)
	StateManager.set_narrative_agent_filter("")
	agent_selected.emit("")


func _on_state_updated(_snapshot: Dictionary) -> void:
	_update_agent_list()


func _on_agent_changed(_agent_id: String, _agent_data: Dictionary) -> void:
	_update_agent_list()


func _on_filter_changed() -> void:
	# 同步 StateManager 的过滤状态
	var current_filter = StateManager.get_narrative_agent_filter()
	if current_filter != selected_agent_id:
		selected_agent_id = current_filter
		_restore_selection()


## 外部调用：设置选中的 Agent（用于点击 Agent Sprite 时）
func select_agent(agent_id: String) -> void:
	selected_agent_id = agent_id
	StateManager.set_narrative_agent_filter(agent_id)
	_restore_selection()
	agent_selected.emit(agent_id)