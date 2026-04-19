# NPC Rule Engine Spec

## Purpose

`RuleEngine` 扩展为 NPC Agent 提供全套复杂动作的决策能力，包括 Build、Trade、Ally 等。

## Requirements

### Requirement: select_target

**WHEN** NPC 需要选择动作目标
**THEN** 基于以下策略选择：

| 动作 | 选择策略 |
|------|----------|
| Attack | 最近的 Agent 或 HP 最低的 Agent |
| Build | 基于最高动机类型选择建筑类型 |
| Ally | 信任度最高或最近的非敌对 Agent |
| Trade | 库存互补的 Agent（我有多的资源，对方有需要的） |
| Talk | 最近的 Agent |

### Requirement: fallback_decision 扩展

**WHEN** NPC 通过 RuleEngine 做决策（不经过 LLM）
**THEN** 根据 6 维动机权重返回对应动作：

| 最高动机 | 可能动作 |
|----------|----------|
| 生存(Survival) | Move / Gather / Wait / Build(Storage) |
| 社交(Social) | Talk / AllyPropose / TradeOffer |
| 认知(Cognitive) | Explore |
| 表达(Expressive) | Build(Campfire) |
| 权力(Power) | Attack / AllyPropose / Build(Fortress) |
| 传承(Legacy) | InteractLegacy |

### Requirement: Build 类型选择

**WHEN** NPC 决定执行 Build
**THEN** 根据最高动机选择建筑类型：
- 生存 → Storage
- 社交 → Campfire
- 权力 → Fortress
- 其他 → Wall

### Requirement: 动作校验

**WHEN** NPC 决定执行动作
**THEN** 在执行前进行与 LLM Agent 相同的校验：
- 资源是否足够（Build）
- 目标是否存在且可达
- 位置是否合法
- 校验失败时降级为 Wait
