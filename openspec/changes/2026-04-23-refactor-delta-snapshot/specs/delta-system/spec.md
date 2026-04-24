# Delta System Spec (Modified)

## Purpose

Delta 机制从14种变体简化为 AgentStateChanged + WorldEvent 两类，实现清晰的事件分类和统一的数据构建。

## MODIFIED Requirements

### Requirement: Delta 序列化

Delta 事件通过通道发送到 Godot 时，SHALL 使用新的结构格式。

#### Scenario: AgentStateChanged 序列化

- **WHEN** 发送 AgentStateChanged 事件
- **THEN** conversion.rs SHALL 转换为 Godot Dictionary：
  - type: "agent_state_changed"
  - agent_id: String
  - state: Dictionary (包含所有 AgentState 字段)
  - change_hint: String ("spawned"|"moved"|"died"|"healed"|"survival_low")

#### Scenario: WorldEvent 序列化

- **WHEN** 发送 WorldEvent 事件
- **THEN** conversion.rs SHALL 转换为 Godot Dictionary：
  - type: 根据事件类型映射（"structure_created"|"milestone_reached"|...）
  - 其他字段 SHALL 根据具体事件类型设置

### Requirement: Structure Delta (Modified)

**WHEN** Structure 被创建
**THEN** 发送 `WorldEvent(StructureCreated { pos, structure_type, owner_id })` 到 Godot
**AND** 不再发送 `AgentDelta::StructureCreated`

**WHEN** Structure 被销毁
**THEN** 发送 `WorldEvent(StructureDestroyed { pos, structure_type })` 到 Godot

### Requirement: Resource Delta (Modified)

**WHEN** ResourceNode 储量发生变化
**THEN** 发送 `WorldEvent(ResourceChanged { pos, resource_type, amount })` 到 Godot

### Requirement: Trade Delta (Modified)

**WHEN** 交易完成
**THEN** 发送 `WorldEvent(TradeCompleted { from_id, to_id, items })` 到 Godot

### Requirement: Alliance Delta (Modified)

**WHEN** 联盟建立
**THEN** 发送 `WorldEvent(AllianceFormed { id1, id2 })` 到 Godot

**WHEN** 联盟破裂
**THEN** 发送 `WorldEvent(AllianceBroken { id1, id2, reason })` 到 Godot

## REMOVED Requirements

### Requirement: AgentMoved Delta 变体

**原因**：被 AgentStateChanged 替代，字段完全重复
**迁移方案**：使用 `Delta::AgentStateChanged { state, change_hint: Moved }`

### Requirement: AgentDied Delta 变体

**原因**：被 AgentStateChanged 替代，AgentState.is_alive=false 即表示死亡
**迁移方案**：使用 `Delta::AgentStateChanged { state { is_alive: false }, change_hint: Died }`

### Requirement: AgentSpawned Delta 变体

**原因**：被 AgentStateChanged 替代，change_hint=Spawned 表示首次出现
**迁移方案**：使用 `Delta::AgentStateChanged { state, change_hint: Spawned }`

### Requirement: HealedByCamp Delta 变体

**原因**：被 AgentStateChanged 替代，change_hint=Healed 表示营地治愈
**迁移方案**：使用 `Delta::AgentStateChanged { state, change_hint: Healed }`

### Requirement: SurvivalWarning Delta 变体

**原因**：被 AgentStateChanged 替代，change_hint=SurvivalLow 表示生存警告
**迁移方案**：使用 `Delta::AgentStateChanged { state, change_hint: SurvivalLow }`