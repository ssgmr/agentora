# 功能规格说明

## Requirements

### Requirement: 资源数量标签显示

系统 SHALL 在世界地图资源点旁绘制具体数量文本，让玩家直观了解资源储量。

#### Scenario: 显示资源数量

- **WHEN** 世界地图渲染可见区域内的资源点
- **AND** 资源点储量 > 0
- **THEN** 系统在资源图标旁绘制数量文本（如 "50"、"120"）
- **AND** 文本位置在资源图标右上角
- **AND** 文本使用白色字体，字号 10

#### Scenario: 资源耗尽不显示

- **WHEN** 资源点储量 = 0 或已耗尽
- **THEN** 系统不绘制数量标签

#### Scenario: 资源数量变化时更新

- **WHEN** 收到 ResourceChanged delta 事件
- **THEN** 系统更新对应资源点的数量标签显示
- **AND** 使用 queue_redraw() 触发重绘

#### Scenario: 使用默认字体

- **WHEN** 绘制资源数量标签
- **THEN** 系统使用 ThemeDB.fallback_font 作为默认字体
- **AND** 无需加载额外字体资源