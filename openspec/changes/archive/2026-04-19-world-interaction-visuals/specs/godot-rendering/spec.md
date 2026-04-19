# 功能规格说明

## MODIFIED Requirements

### Requirement: 资源视觉呈现

系统 SHALL 在 TileMap 上绘制资源图标，并根据储量显示数量标签。

#### Scenario: 绘制资源图标

- **WHEN** 世界地图渲染可见区域
- **AND** 资源点储量 > 0
- **THEN** 系统在对应 Tile 位置绘制资源纹理图标
- **AND** 使用 _resource_textures 字典中的纹理

#### Scenario: 绘制资源数量标签（新增）

- **WHEN** 世界地图渲染可见区域内的资源点
- **AND** 资源点储量 > 0
- **THEN** 系统使用 draw_string() 在资源图标右上角绘制数量文本
- **AND** 使用 ThemeDB.fallback_font 作为字体
- **AND** 字号 10，颜色白色
- **AND** 文本右对齐，宽度 20

#### Scenario: 资源耗尽移除显示

- **WHEN** 收到 ResourceChanged delta 且 amount = 0
- **THEN** 系统从 _resources 字典移除该资源点
- **AND** 后续渲染不显示该资源图标和数量标签

#### Scenario: 资源数量变化时更新标签

- **WHEN** 收到 ResourceChanged delta 且 amount > 0
- **THEN** 系统更新 _resources 字典中对应资源点的 amount 值
- **AND** queue_redraw() 触发重绘，更新数量标签显示