# 实施任务清单

## 1. 资源数量标签实现

实现资源点数量文本显示功能。

- [x] 1.1 在 world_renderer.gd 中添加字体变量
  - 文件: `client/scripts/world_renderer.gd`
  - 在 `_ready()` 中初始化 `var _default_font: Font = ThemeDB.fallback_font`

- [x] 1.2 在 _draw_resources() 中绘制数量标签
  - 文件: `client/scripts/world_renderer.gd`
  - 在纹理绘制后调用 `draw_string(_default_font, label_pos, str(amount), ...)`
  - 标签位置: 资源图标右上角，字号 10

## 2. Agent闪烁效果实现

实现 Agent 采集动作时的闪烁视觉效果。

- [x] 2.1 在 agent_manager.gd 中添加闪烁系统变量
  - 文件: `client/scripts/agent_manager.gd`
  - 新增变量: `_flash_agents: Dictionary`, `_effect_time: float`

- [x] 2.2 实现 flash_agent() 方法
  - 文件: `client/scripts/agent_manager.gd`
  - 参数: agent_id, duration=0.3
  - 将 agent_id 加入 _flash_agents 字典

- [x] 2.3 在 _physics_process() 中更新闪烁效果
  - 文件: `client/scripts/agent_manager.gd`
  - 累加 _effect_time
  - 遍历 _flash_agents，用 sin() 更新 modulate.a
  - 闪烁结束后恢复 modulate.a = 1.0

- [x] 2.4 在 _process_delta() 中触发闪烁
  - 文件: `client/scripts/agent_manager.gd`
  - 处理 agent_moved 时调用 bridge.get_agent_data()
  - 若 current_action 包含 "Gather"，调用 flash_agent()

## 3. Agent状态面板增强

扩展 AgentDetailPanel 显示更多状态信息。

- [x] 3.1 添加动作标签显示
  - 文件: `client/scripts/agent_detail_panel.gd`
  - 新增变量: `_action_label: Label`
  - 在 _setup_ui() 中创建动作行 UI

- [x] 3.2 添加结果标签显示
  - 文件: `client/scripts/agent_detail_panel.gd`
  - 新增变量: `_result_label: Label`
  - 在 _setup_ui() 中创建结果行 UI
  - 根据结果内容设置颜色（成功绿色，失败红色）

- [x] 3.3 添加等级标签显示
  - 文件: `client/scripts/agent_detail_panel.gd`
  - 新增变量: `_level_label: Label`
  - 在 _setup_ui() 中创建等级行 UI（"Lv.X" 格式）

- [x] 3.4 在 _update_display() 中更新新增标签
  - 文件: `client/scripts/agent_detail_panel.gd`
  - 从 agent_data 获取 current_action、action_result、level
  - 更新对应标签文本和颜色

## 4. 测试与验证

验证视觉效果正常渲染。

- [x] 4.1 启动 Godot 客户端验证资源数量标签
  - 运行: `godot --path client`
  - 观察资源点是否显示具体数量

- [x] 4.2 验证 Agent 采集闪烁效果
  - 观察 Agent 执行 Gather 时是否产生绿色闪烁

- [x] 4.3 验证 AgentDetailPanel 状态显示
  - 点击 Agent 检查面板是否显示动作、结果、等级

- [x] 4.4 使用 Godot MCP 截图验证
  - 使用 game_screenshot 工具获取渲染结果
  - 检查视觉效果是否符合预期

- [x] 4.5 合并重复的 AgentDetail 面板
  - 删除 main.tscn 中的 AgentDetail 节点
  - 统一使用 AgentDetailPanel
  - 清理 main.gd 中对已删除节点的引用

## 任务依赖关系

```
1.x (资源标签) ─┬─→ 4.x (验证)
2.x (Agent闪烁)─┤
3.x (状态面板)─┴─
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.1-1.2 | 资源数量标签（最简单，直接绘制） |
| 阶段二 | 2.1-2.4 | Agent闪烁效果（需要新增动画系统） |
| 阶段三 | 3.1-3.4 | 状态面板增强（纯UI扩展） |
| 阶段四 | 4.1-4.4 | 测试验证 |

## 文件结构总览

```
client/scripts/
├── world_renderer.gd    # 修改：添加数量标签绘制
├── agent_manager.gd     # 修改：添加闪烁系统
└── agent_detail_panel.gd # 修改：添加状态显示
```

预计改动量：约 80-100 行 GDScript 代码。