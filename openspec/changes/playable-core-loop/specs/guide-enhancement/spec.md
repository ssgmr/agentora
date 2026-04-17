# 功能规格说明 — guide-enhancement

## ADDED Requirements

### Requirement: 预设倾向按钮

引导面板 SHALL 提供6个预设倾向按钮：生存、社交、探索、创造、征服、传承。点击某个按钮 SHALL 对选中Agent注入对应维度的临时偏好（+30%目标维度，-5%其余维度），持续30 tick。

#### Scenario: 点击生存按钮

- **WHEN** 玩家选中Agent并点击"生存"按钮
- **THEN** Agent生存维度(0) +0.3, 其余维度各 -0.05, 持续30 tick

#### Scenario: 点击社交按钮

- **WHEN** 玩家选中Agent并点击"社交"按钮
- **THEN** Agent社交维度(1) +0.3, 其余维度各 -0.05, 持续30 tick

#### Scenario: 点击探索按钮

- **WHEN** 玩家选中Agent并点击"探索"按钮
- **THEN** Agent认知维度(2) +0.3, 其余维度各 -0.05, 持续30 tick

#### Scenario: 点击创造按钮

- **WHEN** 玩家选中Agent并点击"创造"按钮
- **THEN** Agent表达维度(3) +0.3, 其余维度各 -0.05, 持续30 tick

#### Scenario: 点击征服按钮

- **WHEN** 玩家选中Agent并点击"征服"按钮
- **THEN** Agent权力维度(4) +0.3, 其余维度各 -0.05, 持续30 tick

#### Scenario: 点击传承按钮

- **WHEN** 玩家选中Agent并点击"传承"按钮
- **THEN** Agent传承维度(5) +0.3, 其余维度各 -0.05, 持续30 tick

### Requirement: 自定义高级滑块

引导面板 SHALL 提供6个自定义滑块（对应6维动机），允许玩家手动微调每个维度的注入值（0%-50%），持续时间30 tick。此为高级选项，默认折叠。

#### Scenario: 展开高级滑块

- **WHEN** 玩家点击"高级"展开按钮
- **THEN** 显示6个维度的自定义滑块

#### Scenario: 调整单个滑块

- **WHEN** 玩家将"生存"滑块调到30%
- **THEN** 对选中Agent注入生存维度+0.3的临时偏好

### Requirement: Agent详情面板增强

选中Agent时 SHALL 显示饱食度条、水分度条、HP条、库存列表和当前状态。

#### Scenario: 显示生存状态条

- **WHEN** 玩家点击选中Agent
- **THEN** 面板显示3个状态条：饱食度(绿/黄/红)、水分度(蓝/黄/红)、HP(红)

#### Scenario: 颜色随数值变化

- **WHEN** 饱食度/水分度 > 50
- **THEN** 条为绿色/蓝色
- **WHEN** 饱食度/水分度 20-50
- **THEN** 条变为黄色
- **WHEN** 饱食度/水分度 < 20
- **THEN** 条变为红色

#### Scenario: 显示库存列表

- **WHEN** 玩家选中Agent
- **THEN** 面板显示背包中所有资源类型及数量（如 Food: 5, Water: 3）

### Requirement: 未选中Agent时的提示

当无Agent被选中时，引导面板 SHALL 显示提示文字"点击地图上的Agent以查看详情和引导"。

#### Scenario: 初始无选中提示

- **WHEN** 游戏启动，玩家未选中任何Agent
- **THEN** 面板显示选择提示