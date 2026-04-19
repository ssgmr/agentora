# Godot Rendering Spec

## Purpose

Godot 客户端接收并渲染 Tier 2 新增的世界变化：建筑、资源变化、交易/联盟叙事。

## Requirements

### Requirement: 建筑贴图资源

**WHEN** Godot 需要渲染 Structure
**THEN** 使用 `client/assets/textures/` 下的 PNG 贎图：
- `structure_storage.png`
- `structure_campfire.png`
- `structure_fortress.png`
- `structure_watchtower.png`
- `structure_wall.png`

贴图通过 SVG 生成后转换为 PNG，尺寸 32x32 或 64x64 像素。

### Requirement: Structure 渲染

**WHEN** 收到 `StructureCreated` Delta
**THEN** 在 TileMap 对应 position 放置建筑 sprite

**WHEN** 收到 `StructureDestroyed` Delta
**THEN** 移除对应位置的建筑 sprite

### Requirement: Resource 视觉更新

**WHEN** 收到 `ResourceChanged` Delta
**THEN** 更新资源节点视觉表现（数量减少 → 图标变小/变暗）

### Requirement: Resource 数量标签绘制

**WHEN** 世界地图渲染可见区域内的资源点
**AND** 资源点储量 > 0
**THEN** 系统使用 draw_string() 在资源图标右上角绘制数量文本
**AND** 使用 ThemeDB.fallback_font 作为字体
**AND** 字号 10，颜色白色
**AND** 文本右对齐，宽度 20

**WHEN** 收到 ResourceChanged delta 且 amount = 0
**THEN** 系统从 _resources 字典移除该资源点
**AND** 后续渲染不显示该资源图标和数量标签

**WHEN** 收到 ResourceChanged delta 且 amount > 0
**THEN** 系统更新 _resources 字典中对应资源点的 amount 值
**AND** queue_redraw() 触发重绘，更新数量标签显示

### Requirement: 交易/联盟叙事

**WHEN** 收到 `TradeCompleted` Delta
**THEN** 在叙事流中显示 "AgentA 与 AgentB 完成了交易"

**WHEN** 收到 `AllianceFormed` Delta
**THEN** 在叙事流中显示 "AgentA 与 AgentB 结成了联盟"

**WHEN** 收到 `AllianceBroken` Delta
**THEN** 在叙事流中显示 "AgentA 与 AgentB 的联盟破裂: reason"