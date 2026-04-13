# Bridge Delta Events Spec

## 能力描述

扩展 Bridge 的 Delta 推送机制，将 Structure、Resource、Trade、Alliance 等世界变化实时推送到 Godot 客户端。

## 需求

### 需求: Structure Delta

**WHEN** Structure 被创建
**THEN** 推送 `StructureCreated { position, structure_type, owner_id }` 到 Godot

**WHEN** Structure 被销毁
**THEN** 推送 `StructureDestroyed { position, structure_type }` 到 Godot

### 需求: Resource Delta

**WHEN** ResourceNode 储量发生变化（采集、再生）
**THEN** 推送 `ResourceChanged { position, resource_type, amount }` 到 Godot

### 需求: Trade Delta

**WHEN** 交易完成
**THEN** 推送 `TradeCompleted { from_id, to_id, items }` 到 Godot

### 需求: Alliance Delta

**WHEN** 联盟建立
**THEN** 推送 `AllianceFormed { id1, id2 }` 到 Godot

**WHEN** 联盟破裂
**THEN** 推送 `AllianceBroken { id1, id2, reason }` 到 Godot

### 需求: Delta 序列化

**WHEN** Delta 事件通过通道发送到 Godot
**THEN** 使用 serde 序列化为 Godot 可解析的格式（GodotVariant 兼容类型）
