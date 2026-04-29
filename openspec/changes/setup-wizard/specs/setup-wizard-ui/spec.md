# 功能规格说明 - 引导页面 UI

## ADDED Requirements

### Requirement: 引导页面场景结构

系统 SHALL 提供 setup_wizard.tscn 场景，包含 LLM 配置、Agent 配置、P2P 配置三个区域。

#### Scenario: 场景加载

- **WHEN** Godot 启动时检测到无用户配置文件
- **THEN** SHALL 加载 setup_wizard.tscn 场景
- **AND** 场景 SHALL 包含三个配置区域：LLM/Agent/P2P
- **AND** 底部 SHALL 有"开始游戏"按钮

#### Scenario: 场景布局

- **WHEN** 引导页面渲染
- **THEN** SHALL 使用单页滚动布局
- **AND** 移动端 SHALL 支持垂直滚动浏览所有配置
- **AND** 桌面端 SHALL 在单屏内展示所有配置

### Requirement: LLM 配置 UI

LLM 配置区域 SHALL 支持三种模式切换：本地模型、远程 API、仅规则引擎。

#### Scenario: 模式选择切换

- **WHEN** 用户点击不同模式选项
- **THEN** SHALL 显示对应模式的详细配置界面
- **AND** 本地模式 SHALL 显示模型选择列表和下载按钮
- **AND** 远程 API 模式 SHALL 显示 endpoint、token、model 输入框
- **AND** 规则引擎模式 SHALL 显示说明"无需 LLM，使用规则决策"

#### Scenario: 本地模型选择

- **WHEN** 用户选择本地模型模式
- **THEN** SHALL 显示预置模型列表
- **AND** 每个模型 SHALL 显示名称、大小、描述、下载状态
- **AND** 已下载模型 SHALL 显示"已就绪"标记
- **AND** 未下载模型 SHALL 显示"下载"按钮

#### Scenario: 远程 API 配置

- **WHEN** 用户选择远程 API 模式
- **THEN** SHALL 显示 API Endpoint 输入框
- **AND** SHALL 显示 API Token 输入框（密码模式隐藏）
- **AND** SHALL 显示 Model Name 输入框
- **AND** 输入框 SHALL 有 placeholder 提示

### Requirement: Agent 配置 UI

Agent 配置区域 SHALL 支持自定义名字、系统提示词、图标选择。

#### Scenario: Agent 名字输入

- **WHEN** 用户输入 Agent 名字
- **THEN** 输入框 SHALL 有字符限制（最大 20 字符）
- **AND** 名字不能为空，空时 SHALL 显示提示
- **AND** 名字 SHALL 用于世界中的 Agent 显示

#### Scenario: 系统提示词输入

- **WHEN** 用户编辑系统提示词
- **THEN** SHALL 显示多行文本输入框
- **AND** 输入框 SHALL 有 placeholder："描述 Agent 的性格和倾向..."
- **AND** 提示词可选，为空时使用默认 Prompt

#### Scenario: 预设图标选择

- **WHEN** 用户选择 Agent 图标
- **THEN** SHALL 显示 6 个预设图标按钮（可视化）
- **AND** 图标 SHALL 包括：默认、法师、狐狸、龙、狮子、机器人
- **AND** 点击图标 SHALL 高亮选中状态
- **AND** 选中的图标 SHALL 用于游戏中 Agent 显示

#### Scenario: 自定义图标上传

- **WHEN** 用户点击"上传自定义图标"按钮
- **THEN** SHALL 打开文件选择对话框
- **AND** 支持 PNG/JPG 格式
- **AND** 上传后 SHALL 自动缩放到 32x32 尺寸
- **AND** 缩放后的图标 SHALL 显示预览

### Requirement: P2P 配置 UI

P2P 配置区域 SHALL 支持三种模式：单机、创建世界、加入世界。

#### Scenario: P2P 模式选择

- **WHEN** 用户选择不同 P2P 模式
- **THEN** SHALL 显示对应的配置界面
- **AND** 单机模式 SHALL 无需额外配置
- **AND** 创建世界 SHALL 显示本地 P2P 地址
- **AND** 加入世界 SHALL 显示地址输入框

#### Scenario: 创建世界地址显示

- **WHEN** 用户选择创建世界模式
- **THEN** SHALL 显示本地节点的 P2P 地址
- **AND** 地址格式 SHALL 为 /ip4/.../tcp/.../p2p/...
- **AND** SHALL 提供"复制地址"按钮
- **AND** SHALL 提供"分享地址"功能（移动端）

#### Scenario: 加入世界地址输入

- **WHEN** 用户选择加入世界模式
- **THEN** SHALL 显示种子节点地址输入框
- **AND** 输入框 SHALL 有 placeholder 提示
- **AND** SHALL 提供"从剪贴板粘贴"按钮（移动端）
- **AND** 可选提供二维码扫描功能

### Requirement: 移动端 UI 适配

引导页面 SHALL 针对移动端触摸操作优化。

#### Scenario: 触摸友好按钮

- **WHEN** 在移动端渲染引导页面
- **THEN** 所有按钮 SHALL 有足够大的触摸区域（至少 44x44 dp）
- **AND** 按钮 SHALL 有明显的视觉反馈（按下状态）

#### Scenario: 输入框适配

- **WHEN** 移动端用户聚焦输入框
- **THEN** SHALL 自动弹出软键盘
- **AND** 输入框 SHALL 不被键盘遮挡（滚动调整）

#### Scenario: 图标选择可视化

- **WHEN** 显示图标选择区域
- **THEN** 图标 SHALL 以网格形式展示
- **AND** 每个图标 SHALL 有足够的触摸区域
- **AND** 选中状态 SHALL 有明显的视觉标记

### Requirement: 开始游戏按钮

点击"开始游戏" SHALL 保存配置并切换到主场景。

#### Scenario: 保存配置

- **WHEN** 用户点击"开始游戏"
- **THEN** SHALL 验证必填项（Agent 名字非空）
- **AND** 验证通过后 SHALL 调用 Bridge.set_user_config()
- **AND** 配置 SHALL 保存到 config/user_config.toml

#### Scenario: 切换场景

- **WHEN** 配置保存成功
- **THEN** SHALL 切换到 main.tscn 场景
- **AND** 模拟 SHALL 使用用户配置启动

#### Scenario: 配置验证失败

- **WHEN** 必填项为空（Agent 名字）
- **THEN** SHALL 显示错误提示
- **AND** 不切换场景
- **AND** 聚焦到空输入框