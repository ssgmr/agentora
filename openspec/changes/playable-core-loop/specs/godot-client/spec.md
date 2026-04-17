# 功能规格说明 — godot-client (修改)

## MODIFIED Requirements

### Requirement: 引导面板重设计

guide_panel.gd SHALL 从6个抽象滑块改为6个预设倾向按钮 + 可折叠高级滑块面板。

#### Scenario: 预设按钮界面

- **WHEN** 玩家打开引导面板
- **THEN** 显示6个按钮：生存、社交、探索、创造、征服、传承
- **AND** 底部有"高级"展开/折叠切换

#### Scenario: 高级滑块折叠

- **WHEN** 高级面板折叠
- **THEN** 不显示自定义滑块

#### Scenario: 高级滑块展开

- **WHEN** 玩家点击"高级"
- **THEN** 显示6维动机的自定义滑块(0%-50%)

### Requirement: Agent详情面板增强

选中Agent时详情面板 SHALL 显示饱食度条、水分度条、HP条、库存列表。

#### Scenario: 显示生存指标

- **WHEN** 玩家选中Agent
- **THEN** 面板顶部显示3个状态条：饱食度(绿→黄→红)、水分度(蓝→黄→红)、HP(红)

#### Scenario: 显示库存

- **WHEN** 玩家选中Agent
- **THEN** 面板中段显示背包资源列表（如 "Food: 5, Wood: 3"）

## ADDED Requirements

### Requirement: 里程碑进度UI

界面 SHALL 在顶部或底部显示里程碑进度条（已达成数/总数），达成时弹出2秒提示。

#### Scenario: 进度展示

- **WHEN** 游戏运行中
- **THEN** 显示 "🏆 3/7" 进度指示

#### Scenario: 里程碑达成提示

- **WHEN** 收到MilestoneReached事件
- **THEN** 屏幕中央弹出"🏆 第一座营地 已达成！"提示，2秒后消失

### Requirement: 压力事件叙事显示

叙事流面板 SHALL 用特殊颜色显示压力事件的开始和结束。

#### Scenario: 压力事件显示

- **WHEN** 收到PressureStarted事件
- **THEN** 叙事流中用橙色显示"☀️ 干旱来袭，水源产出减半"

#### Scenario: 压力结束显示

- **WHEN** 收到PressureEnded事件
- **THEN** 叙事流中用灰色显示"干旱已结束"