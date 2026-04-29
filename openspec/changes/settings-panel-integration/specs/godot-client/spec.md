# 功能规格说明 - Godot 客户端（增量）

## ADDED Requirements

### Requirement: TopBar 设置按钮节点

main.tscn TopBar SHALL 包含设置按钮节点。

#### Scenario: 添加 SettingsBtn 节点

- **WHEN** main.tscn 场景结构更新
- **THEN** TopBar HBoxContainer SHALL 包含 SettingsBtn 节点
- **AND** SettingsBtn 类型 SHALL 为 Button 或 MenuButton
- **AND** SettingsBtn 位置 SHALL 在 TopBar 最右侧
- **AND** SettingsBtn 文字或图标 SHALL 为 "⚙" 或 "设置"

### Requirement: main.gd settings_panel 连接

main.gd SHALL 连接 settings_panel 并处理打开/关闭逻辑。

#### Scenario: 引用 settings_panel 节点

- **WHEN** main.gd _ready() 执行
- **THEN** SHALL 获取 settings_panel 节点引用（通过 $UI/SettingsPanel 或 get_node_or_null）
- **AND** settings_panel 初始状态 SHALL 为隐藏

#### Scenario: SettingsBtn 点击处理

- **WHEN** 用户点击 SettingsBtn
- **THEN** main.gd SHALL 调用 settings_panel 显示方法
- **AND** settings_panel SHALL 加载当前配置显示

#### Scenario: ESC 键处理

- **WHEN** 用户按下 ESC 键
- **THEN** main.gd SHALL 检查 settings_panel 可见状态
- **AND** 若可见则隐藏，若不可见则显示

### Requirement: settings_panel 场景节点

main.tscn SHALL 包含 settings_panel 场景实例。

#### Scenario: 嵌入 settings_panel

- **WHEN** main.tscn 场景结构更新
- **THEN** UI CanvasLayer SHALL 包含 SettingsPanel 节点
- **AND** SettingsPanel SHALL 使用 scenes/settings_panel.tscn 实例
- **AND** SettingsPanel 初始 visible SHALL 为 false