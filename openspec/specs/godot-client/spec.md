# Godot 4客户端

## Purpose

定义通过godot-rust GDExtension将Rust模拟核心桥接至Godot 4客户端的完整实现，包括2D渲染、Agent可视化、动机雷达图、叙事流面板和桌面打包。

## Requirements

### Requirement: GDExtension Bridge

系统 SHALL 通过godot-rust GDExtension将Rust模拟核心桥接至Godot 4。SimulationBridge作为Godot节点运行，在`crates/bridge/src/lib.rs`中实现，通过mpsc Channel将WorldSnapshot传递至Godot主线程。

#### Scenario: 启动模拟

- **WHEN** Godot主场景加载完成
- **THEN** SimulationBridge节点（Rust GDExtension类）SHALL 启动Tokio运行时
- **AND** 初始化World/Network/Sync/AI模块
- **AND** 开始tick循环
- **AND** autoload 配置从 `res://scripts/simulation_bridge.gd` 切换为 GDExtension 注册的类型

#### Scenario: 快照传递

- **WHEN** 模拟tick完成
- **THEN** SimulationBridge SHALL 通过mpsc channel发送WorldSnapshot
- **AND** Godot主线程的physics_process SHALL poll channel更新视图

#### Scenario: 关闭模拟

- **WHEN** Godot场景退出
- **THEN** 系统 SHALL 优雅关闭Tokio运行时
- **AND** 保存世界状态至本地存储

#### Scenario: GDExtension 加载失败回退

- **WHEN** GDExtension DLL 文件不存在或版本不兼容
- **THEN** 系统 SHALL 回退至 GDScript 模拟版 `res://scripts/simulation_bridge.gd`

### Requirement: 2D地图渲染

系统 SHALL 使用Godot TileMapLayer渲染256×256世界地图，不同地形类型对应不同Tile。支持摄像机平移和缩放浏览大地图。

#### Scenario: 地图显示

- **WHEN** 世界模型初始化完成
- **THEN** Godot SHALL 根据WorldSnapshot渲染完整地图

#### Scenario: 地图更新

- **WHEN** 资源节点状态变化或新建筑出现
- **THEN** TileMap SHALL 更新对应格子的Tile

#### Scenario: 摄像机浏览

- **WHEN** 用户拖拽地图
- **THEN** 摄像机 SHALL 平移
- **AND** 支持滚轮缩放，范围0.5x~3x

### Requirement: Agent可视化

系统 SHALL 为每个Agent创建Sprite2D+Label节点，显示位置、移动动画、名字标签。点击Agent可查看详情面板。

#### Scenario: Agent位置更新

- **WHEN** Agent移动到新位置
- **THEN** Sprite2D SHALL 平滑移动至新坐标（插值动画）

#### Scenario: Agent点击交互

- **WHEN** 用户点击Agent Sprite
- **THEN** 系统 SHALL 在右侧面板显示Agent详情
- **AND** 面板 SHALL 显示饱食度条、水分度条、HP条、库存列表
- **AND** 饱食度条颜色随数值变化：>50绿色，20-50黄色，<20红色
- **AND** 水分度条颜色随数值变化：>50蓝色，20-50黄色，<20红色

#### Scenario: Agent死亡效果

- **WHEN** Agent死亡
- **THEN** Sprite2D SHALL 播放消失动画
- **AND** 原位置 SHALL 生成遗迹标记

### Requirement: 动机雷达图

系统 SHALL 在Agent详情面板中显示6维动机向量的雷达图，使用CanvasItem自定义绘制。雷达图每tick刷新。

#### Scenario: 展示动机向量

- **WHEN** 用户选中一个Agent
- **THEN** 雷达图 SHALL 显示当前6维动机值

#### Scenario: 动机变化实时更新

- **WHEN** Agent动机向量在tick中发生变化
- **THEN** 雷达图 SHALL 在下一帧更新显示

### Requirement: 叙事流面板

系统 SHALL 在界面底部显示叙事流（RichTextLabel），滚动显示Agent动作、事件通知、环境压力等叙事文本。新事件自动滚动至底部。

#### Scenario: 叙事事件显示

- **WHEN** 新的叙事事件产生
- **THEN** 叙事流面板 SHALL 追加事件文本并自动滚动

#### Scenario: 事件类型区分

- **WHEN** 不同类型事件显示时
- **THEN** 系统 SHALL 用不同颜色标识：Agent动作=白色、交易=绿色、攻击=红色、环境压力=黄色、遗产=紫色

### Requirement: 玩家引导面板

系统 SHALL 提供引导面板，包含 6 个预设倾向按钮（生存/社交/探索/创造/征服/传承）+ 可折叠高级滑块面板。点击按钮 SHALL 对选中 Agent 注入对应维度的临时偏好（+30% 目标维度，-5% 其余维度），持续 30 tick。

#### Scenario: 预设按钮界面

- **WHEN** 玩家打开引导面板
- **THEN** 显示 6 个按钮：生存、社交、探索、创造、征服、传承
- **AND** 底部有"高级"展开/折叠切换

#### Scenario: 高级滑块折叠

- **WHEN** 高级面板折叠
- **THEN** 不显示自定义滑块

#### Scenario: 高级滑块展开

- **WHEN** 玩家点击"高级"
- **THEN** 显示 6 维动机的自定义滑块 (0%-50%)

#### Scenario: 调整动机权重

- **WHEN** 用户拖动"生存"滑块至 30%
- **THEN** Agent 的"生存与资源"动机 SHALL 在下一 tick 提升
- **AND** Agent 决策 SHALL 更倾向资源获取行为

#### Scenario: 未选中 Agent 时提示

- **WHEN** 游戏启动，玩家未选中任何 Agent
- **THEN** 面板显示选择提示"点击地图上的 Agent 以查看详情和引导"

### Requirement: 桌面打包分发

系统 SHALL 可通过Godot导出为Windows(.exe)、macOS(.app)和Linux(AppImage)单文件可执行程序。Rust动态库嵌入Godot PCK中，用户无需额外安装。

#### Scenario: Windows导出

- **WHEN** 执行Godot桌面导出
- **THEN** SHALL 生成agentora.exe单文件
- **AND** 双击可运行，自动打开世界界面

#### Scenario: macOS导出

- **WHEN** 执行Godot macOS导出
- **THEN** SHALL 生成agentora.app
- **AND** 双击可运行

### Requirement: 里程碑进度UI

界面 SHALL 在顶部或底部显示里程碑进度条（已达成数/总数），达成时弹出 2 秒提示。

#### Scenario: 进度展示

- **WHEN** 游戏运行中
- **THEN** 显示里程碑进度如 "3/7"

#### Scenario: 里程碑达成提示

- **WHEN** 收到MilestoneReached事件
- **THEN** 屏幕中央弹出达成提示，2秒后消失

### Requirement: 压力事件叙事显示

叙事流面板 SHALL 用特殊颜色显示压力事件的开始和结束。

#### Scenario: 压力事件显示

- **WHEN** 收到PressureStarted事件
- **THEN** 叙事流中用橙色显示压力事件描述

#### Scenario: 压力结束显示

- **WHEN** 收到PressureEnded事件
- **THEN** 叙事流中用灰色显示压力结束信息
