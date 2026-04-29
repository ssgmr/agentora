# 功能规格说明 - Godot 客户端（增量）

## ADDED Requirements

### Requirement: 启动流程改造

Godot 启动流程 SHALL 改为检测配置文件，决定加载引导页面或主场景。

#### Scenario: 检测配置文件

- **WHEN** main.gd 的 _ready() 执行
- **THEN** SHALL 调用 Bridge.has_user_config()
- **AND** 根据返回值决定后续流程

#### Scenario: 无配置加载引导页面

- **WHEN** has_user_config() 返回 false
- **THEN** SHALL 加载 setup_wizard.tscn 场景
- **AND** 替换当前场景（get_tree().change_scene_to_file）
- **AND** 不初始化 Simulation

#### Scenario: 有配置加载主场景

- **WHEN** has_user_config() 返回 true
- **THEN** SHALL 继续当前 main.tscn 场景
- **AND** 正常初始化 SimulationBridge
- **AND** 从配置启动模拟

### Requirement: 引导页面场景

系统 SHALL 新增 setup_wizard.tscn 场景，包含完整的引导配置 UI。

#### Scenario: 场景文件位置

- **WHEN** 创建引导页面
- **THEN** SHALL 位于 client/scenes/setup_wizard.tscn
- **AND** 场景根节点为 Control 类型

#### Scenario: 引导脚本关联

- **WHEN** 引导页面场景定义
- **THEN** SHALL 挂载 setup_wizard.gd 脚本
- **AND** 脚本 SHALL 处理配置逻辑和场景切换

### Requirement: 场景切换逻辑

引导页面完成配置后 SHALL 切换到主场景并启动模拟。

#### Scenario: 配置完成切换

- **WHEN** 用户在引导页面点击"开始游戏"
- **AND** 配置验证通过
- **THEN** SHALL 调用 Bridge.set_user_config()
- **AND** 保存成功后 SHALL 调用 get_tree().change_scene_to_file("res://scenes/main.tscn")

#### Scenario: 主场景启动模拟

- **WHEN** 从引导页面切换到 main.tscn
- **THEN** main.gd SHALL 加载用户配置
- **AND** 使用配置初始化模拟（LLM Provider、Agent 名字、P2P 模式）

### Requirement: 游戏内设置面板

游戏运行时 SHALL 提供设置面板修改配置。

#### Scenario: 设置面板入口

- **WHEN** 用户在游戏中打开设置菜单
- **THEN** SHALL 显示设置面板（可复用引导页面部分 UI）
- **AND** 加载当前配置显示

#### Scenario: 配置修改保存

- **WHEN** 用户修改配置并点击保存
- **THEN** SHALL 调用 Bridge.set_user_config()
- **AND** 显示提示"配置已保存，重启生效"

#### Scenario: 重启提示

- **WHEN** 用户修改了需要重启生效的配置（如 LLM 模式）
- **THEN** SHALL 显示重启提示
- **AND** 提供立即重启按钮