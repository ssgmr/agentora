# Godot Rendering Spec

## 能力描述

Godot 客户端接收并渲染 Tier 2 新增的世界变化：建筑、资源变化、交易/联盟叙事。

## 需求

### 需求: 建筑贴图资源

**WHEN** Godot 需要渲染 Structure
**THEN** 使用 `client/assets/textures/` 下的 PNG 贴图：
- `structure_storage.png`
- `structure_campfire.png`
- `structure_fortress.png`
- `structure_watchtower.png`
- `structure_wall.png`

贴图通过 SVG 生成后转换为 PNG，尺寸 32x32 或 64x64 像素。

### 需求: Structure 渲染

**WHEN** 收到 `StructureCreated` Delta
**THEN** 在 TileMap 对应 position 放置建筑 sprite

**WHEN** 收到 `StructureDestroyed` Delta
**THEN** 移除对应位置的建筑 sprite

### 需求: Resource 视觉更新

**WHEN** 收到 `ResourceChanged` Delta
**THEN** 更新资源节点视觉表现（数量减少 → 图标变小/变暗）

### 需求: 交易/联盟叙事

**WHEN** 收到 `TradeCompleted` Delta
**THEN** 在叙事流中显示 "AgentA 与 AgentB 完成了交易"

**WHEN** 收到 `AllianceFormed` Delta
**THEN** 在叙事流中显示 "AgentA 与 AgentB 结成了联盟"

**WHEN** 收到 `AllianceBroken` Delta
**THEN** 在叙事流中显示 "AgentA 与 AgentB 的联盟破裂: reason"
