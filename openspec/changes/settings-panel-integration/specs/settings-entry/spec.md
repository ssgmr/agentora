# 功能规格说明 - 设置入口

## ADDED Requirements

### Requirement: 设置按钮入口

游戏 TopBar SHALL 提供设置按钮入口，用户点击后打开 settings_panel 弹窗。

#### Scenario: 显示设置按钮

- **WHEN** main.tscn 场景加载完成
- **THEN** TopBar SHALL 显示设置按钮（齿轮图标或"设置"文字）
- **AND** 按钮位置在 TopBar 右侧

#### Scenario: 点击打开设置面板

- **WHEN** 用户点击设置按钮
- **THEN** 系统 SHALL 显示 settings_panel 弹窗
- **AND** settings_panel SHALL 加载当前用户配置显示

#### Scenario: 点击关闭设置面板

- **WHEN** 用户点击设置面板的关闭按钮
- **THEN** 系统 SHALL 隐藏 settings_panel 弹窗
- **AND** 恢复游戏正常交互

### Requirement: ESC 快捷键

系统 SHALL 支持 ESC 快捷键打开/关闭设置面板。

#### Scenario: ESC 打开设置

- **WHEN** 用户按下 ESC 键
- **AND** settings_panel 当前不可见
- **THEN** 系统 SHALL 显示 settings_panel

#### Scenario: ESC 关闭设置

- **WHEN** 用户按下 ESC 键
- **AND** settings_panel 当前可见
- **THEN** 系统 SHALL 隐藏 settings_panel

#### Scenario: 游戏暂停时 ESC 行为

- **WHEN** 游戏处于暂停状态
- **AND** 用户按下 ESC 键
- **THEN** 系统 SHALL 优先关闭 settings_panel（如果打开）
- **AND** 若 settings_panel 已关闭，则恢复游戏运行

### Requirement: 设置面板弹窗样式

settings_panel SHALL 以弹窗形式显示，具有合适的尺寸和样式。

#### Scenario: 弹窗尺寸

- **WHEN** settings_panel 显示
- **THEN** 弹窗尺寸 SHALL 覆盖屏幕中央区域（约 400x500 px）
- **AND** 弹窗背景 SHALL 使用半透明深色遮罩
- **AND** 弹窗内容 SHALL 使用圆角面板样式

#### Scenario: 弹窗层级

- **WHEN** settings_panel 显示
- **THEN** 弹窗 SHALL 位于 CanvasLayer 上层
- **AND** 阻挡下层 UI 的交互事件