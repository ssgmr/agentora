# 功能规格说明 - 设置面板 UI

## ADDED Requirements

### Requirement: settings_panel 静态 UI 定义

settings_panel.tscn SHALL 使用静态节点定义完整 UI 结构，而非动态构建。

#### Scenario: 删除动态构建代码

- **WHEN** settings_panel.gd 重构完成
- **THEN** SHALL 删除 _build_ui() 动态构建函数
- **AND** SHALL 删除动态创建的样式函数（如 _create_label、_create_spacer）
- **AND** SHALL 直接使用 .tscn 中定义的节点引用

#### Scenario: 静态节点结构

- **WHEN** settings_panel.tscn 编辑完成
- **THEN** SHALL 包含以下节点结构：
  - PanelContainer (根节点)
    - Bg (ColorRect, 深色背景)
    - MarginContainer
      - VBoxContainer
        - TitleLabel ("设置")
        - LLM 配置区
        - Agent 配置区
        - P2P 配置区
        - RestartLabel
        - BtnHBox (保存/关闭按钮)

### Requirement: settings_panel 完整配置项

settings_panel SHALL 支持所有用户配置项的显示和修改。

#### Scenario: LLM 模式配置

- **WHEN** settings_panel 显示
- **THEN** SHALL 显示 LLM 模式切换按钮组（本地/远程/规则引擎）
- **AND** 远程模式 SHALL 显示 endpoint/token/model 输入框
- **AND** 本地模式 SHALL 显示模型路径输入（或选择按钮）

#### Scenario: Agent 名字配置

- **WHEN** settings_panel 显示
- **THEN** SHALL 显示 Agent 名字 LineEdit
- **AND** 名字不能为空（保存时验证）

#### Scenario: Agent custom_prompt 配置

- **WHEN** settings_panel 显示
- **THEN** SHALL 显示系统提示词 TextEdit
- **AND** 提示词为可选（允许为空）
- **AND** placeholder_text SHALL 为 "描述智能体性格..."

#### Scenario: Agent icon 配置

- **WHEN** settings_panel 显示
- **THEN** SHALL 显示图标选择 GridContainer
- **AND** 包含 6 个预设图标 TextureButton（default/wizard/fox/dragon/lion/robot）
- **AND** 选中图标 SHALL 高亮显示（其他图标变灰）
- **AND** 当前选中图标 SHALL 从配置加载

#### Scenario: P2P 模式配置

- **WHEN** settings_panel 显示
- **THEN** SHALL 显示 P2P 模式切换按钮组（单机/创建/加入）
- **AND** 创建模式 SHALL 显示本地地址 Label
- **AND** 加入模式 SHALL 显示种子地址输入 LineEdit

### Requirement: 配置加载和保存

settings_panel SHALL 正确加载当前配置并保存修改。

#### Scenario: 加载当前配置

- **WHEN** settings_panel 打开
- **THEN** SHALL 调用 Bridge.get_user_config()
- **AND** 将配置值填充到各输入控件
- **AND** LLM 模式按钮 SHALL 正确选中当前模式
- **AND** 图标 SHALL 正确选中当前 icon_id
- **AND** P2P 模式按钮 SHALL 正确选中当前模式

#### Scenario: 保存配置

- **WHEN** 用户点击保存按钮
- **THEN** SHALL 收集所有输入控件值
- **AND** 验证 Agent 名字非空
- **AND** 构建 config Dictionary
- **AND** 调用 Bridge.set_user_config()
- **AND** 显示保存结果提示

#### Scenario: 配置验证失败

- **WHEN** Agent 名字为空
- **THEN** SHALL 显示错误提示 "Agent 名字不能为空"
- **AND** 聚焦到名字输入框
- **AND** 不调用保存

### Requirement: 重启生效提示

settings_panel SHALL 在修改需要重启的配置时显示提示。

#### Scenario: 检测重启需求

- **WHEN** 用户修改 LLM 模式
- **THEN** SHALL 标记 restart_required = true
- **AND** 显示 "更改 LLM 模式需要重启生效"

#### Scenario: 保存成功提示

- **WHEN** 配置保存成功
- **AND** restart_required 为 true
- **THEN** SHALL 显示 "配置已保存，请重启游戏"

#### Scenario: 重启按钮（可选）

- **WHEN** 显示重启提示
- **THEN** 可选提供 "立即重启" 按钮
- **AND** 点击后 SHALL 调用 get_tree().reload_current_scene()

### Requirement: 样式一致性

settings_panel 样式 SHALL 与 setup_wizard 保持一致。

#### Scenario: 使用共享样式

- **WHEN** settings_panel 构建/显示
- **THEN** SHALL 使用 shared_ui_styles.gd 的样式常量
- **AND** 深色背景色 SHALL 为 COLOR_BG_PANEL
- **AND** 输入框样式 SHALL 为 create_input_style()
- **AND** 按钮样式 SHALL 为 create_button_style()