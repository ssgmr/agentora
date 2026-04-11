# Godot 4客户端

## Purpose

定义通过godot-rust GDExtension将Rust模拟核心桥接至Godot 4客户端的完整实现，包括2D渲染、Agent可视化、动机雷达图、叙事流面板和桌面打包。

## Requirements

### Requirement: GDExtension Bridge

系统 SHALL 通过godot-rust GDExtension将Rust模拟核心桥接至Godot 4。SimulationBridge作为Godot节点运行，在单独的Tokio运行时中驱动模拟，通过mpsc Channel将WorldSnapshot传递至Godot主线程。

#### Scenario: 启动模拟

- **WHEN** Godot主场景加载完成
- **THEN** SimulationBridge节点 SHALL 启动Tokio运行时
- **AND** 初始化World/Network/Sync/AI模块
- **AND** 开始tick循环

#### Scenario: 快照传递

- **WHEN** 模拟tick完成
- **THEN** SimulationBridge SHALL 通过mpsc channel发送WorldSnapshot
- **AND** Godot主线程的physics_process SHALL poll channel更新视图

#### Scenario: 关闭模拟

- **WHEN** Godot场景退出
- **THEN** 系统 SHALL 优雅关闭Tokio运行时
- **AND** 保存世界状态至本地存储

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

系统 SHALL 提供引导面板，允许玩家通过HSlider调整Agent的6维动机权重。调整后的权重在下一tick生效，影响Agent决策。

#### Scenario: 调整动机权重

- **WHEN** 用户拖动"生存与资源"滑块至0.9
- **THEN** Agent的"生存与资源"动机 SHALL 在下一tick提升
- **AND** Agent决策 SHALL 更倾向资源获取行为

#### Scenario: 注入偏好

- **WHEN** 用户点击"建议探索"按钮
- **THEN** 系统 SHALL 向Agent注入一个临时偏好（"认知与好奇"维度临时+0.3）
- **AND** 该偏好 SHALL 在3个tick后衰减

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
