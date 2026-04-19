# Civilization Milestones Spec

## Purpose

定义文明里程碑检测系统，自动追踪文明演进的七个关键阶段，并推送至 Godot 客户端展示。

## Requirements

### Requirement: 里程碑检测系统

系统 SHALL 维护一个里程碑列表，自动检测 7 个文明里程碑的达成条件。每个里程碑只能达成一次。

#### Scenario: 初始化里程碑列表

- **WHEN** 世界创建
- **THEN** milestones 列表为空，7 个里程碑均未达成

### Requirement: 里程碑定义

系统 SHALL 支持以下 7 个里程碑，按大致时间顺序排列：

1. **第一座营地** — 任意 Agent 建成第 1 个 Camp
2. **贸易萌芽** — 第 1 次 TradeAccept 成功执行
3. **领地意识** — 第 1 个 Fence 建成
4. **冲突爆发** — 第 1 次 Attack 动作执行
5. **首次传承** — 第 1 个 Legacy 被 Interact (Worship/Explore/Pickup)
6. **城邦雏形** — 同时满足：存活建筑 ≥3 个、盟友对数 ≥2、存在至少 1 个 Warehouse
7. **文明黄金期** — 前 6 个里程碑全部达成

#### Scenario: 检测第一座营地

- **WHEN** Agent 执行 Build Camp 成功
- **AND** "第一座营地"里程碑未达成
- **THEN** 标记"第一座营地"达成，生成里程碑事件

#### Scenario: 检测贸易萌芽

- **WHEN** TradeAccept 动作成功执行
- **AND** "贸易萌芽"里程碑未达成
- **THEN** 标记"贸易萌芽"达成

#### Scenario: 检测城邦雏形

- **WHEN** 世界中有存活建筑 ≥3 个、至少 2 对盟友关系、存在至少 1 个 Warehouse
- **AND** "城邦雏形"里程碑未达成
- **THEN** 标记"城邦雏形"达成

#### Scenario: 检测文明黄金期

- **WHEN** 前 6 个里程碑全部达成
- **AND** "文明黄金期"里程碑未达成
- **THEN** 标记"文明黄金期"达成

#### Scenario: 里程碑不可重复达成

- **WHEN** "第一座营地"已达成
- **AND** 另一个 Agent 建成 Camp
- **THEN** 不再触发"第一座营地"里程碑

### Requirement: 里程碑推送至 Godot

里程碑达成时 SHALL 推送 MilestoneReached 事件到 Godot 客户端，并在叙事流中记录。

#### Scenario: 里程碑达成推送

- **WHEN** 任意里程碑达成
- **THEN** 推送 MilestoneReached Delta 事件（含里程碑名称和达成 tick）
- **AND** 生成 NarrativeEvent 记录到叙事流

### Requirement: 里程碑进度显示

Godot 客户端 SHALL 在界面上显示里程碑进度（已达成/总数），达成时弹出短暂提示。

#### Scenario: 进度展示

- **WHEN** 游戏运行中
- **THEN** 界面上显示里程碑进度如 "3/7"

#### Scenario: 达成提示

- **WHEN** 里程碑达成
- **THEN** 界面弹出达成提示（2 秒后自动消失），图标高亮

### Requirement: 里程碑序列化

里程碑状态 SHALL 包含在 WorldSnapshot 中，通过 Bridge 推送到 Godot。

#### Scenario: 快照包含里程碑

- **WHEN** 生成 WorldSnapshot
- **THEN** 包含已达成里程碑的列表和达成 tick
