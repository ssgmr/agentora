## ADDED Requirements

### Requirement: SVG矢量资源覆盖核心视觉元素
系统 SHALL 提供SVG矢量图资源，覆盖Agent、地形、建筑、遗迹等核心视觉元素。

#### Scenario: Agent图标可用
- **GIVEN** 游戏已启动
- **THEN** Agent以矢量圆形图标显示，大小不小于16px，普通状态为蓝色，选中状态为黄色高亮

#### Scenario: 地形纹理清晰可辨
- **GIVEN** 世界已生成
- **THEN** 5种地形（草原/森林/山脉/水域/沙漠）有对应的纹理图标，风格统一为矢量扁平风

#### Scenario: 建筑和遗迹图标可用
- **WHEN** Agent建造结构
- **THEN** 建筑以小方块+屋顶样式显示在地图对应位置

#### Scenario: Agent遗迹可见
- **WHEN** Agent死亡
- **THEN** 遗迹以灰色墓碑形图标显示在原位置

### Requirement: 资源文件组织规范
系统 SHALL 将资源文件按类型组织在 `client/assets/` 目录下。

#### Scenario: 资源目录结构清晰
- **GIVEN** 资源已创建
- **THEN** `assets/sprites/` 存放Agent/建筑/遗迹PNG，`assets/textures/` 存地形PNG，`assets/svg/` 存原始SVG源文件

### Requirement: SVG到PNG导出流程
系统 SHALL 提供SVG→PNG的导出脚本，将矢量图批量导出为Godot可用的PNG格式。

#### Scenario: 批量导出
- **WHEN** 运行导出脚本
- **THEN** 所有SVG源文件导出为对应尺寸（16×16或32×32）的PNG，透明背景

## MODIFIED Requirements

### Requirement: 地形呈现自然斑块分布
系统 SHALL 使用多层分形噪声生成地形，而非简单正弦函数。

#### Scenario: 地形斑块自然
- **GIVEN** 世界已生成
- **WHEN** 观察地图
- **THEN** 地形呈现不规则的斑块分布，森林/草原/山脉/水域/沙漠边界自然过渡，无同心圆图案

#### Scenario: 地形类型比例合理
- **GIVEN** 默认世界种子
- **THEN** 草原和森林占比最大，水域和山脉适中，沙漠较少
