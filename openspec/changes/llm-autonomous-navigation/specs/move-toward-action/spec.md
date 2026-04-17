# MoveToward 导航动作

## Purpose

定义 `ActionType::MoveToward { target: Position }` 动作的完整行为，让 LLM Agent 能够直接指定导航目标坐标，无需计算方向。

## ADDED Requirements

### Requirement: MoveToward 动作定义

系统 SHALL 提供 `ActionType::MoveToward { target: Position }` 动作类型，允许 Agent 直接指定目标位置。

#### Scenario: MoveToward 动作结构

- **WHEN** 系统定义 ActionType 枚举
- **THEN** 枚举 SHALL 包含 `MoveToward { target: Position }` 变体
- **AND** target SHALL 为有效的 Position 类型（x, y 坐标）

### Requirement: MoveToward 验证逻辑

规则引擎 SHALL 验证 MoveToward 动作的有效性。

#### Scenario: 目标在视野范围内

- **WHEN** LLM 返回 MoveToward 动作
- **AND** 目标位置在 Agent 视野范围内（当前为半径 5）
- **THEN** 验证 SHALL 通过
- **AND** 动作 SHALL 被加入候选动作列表

#### Scenario: 目标超出视野范围

- **WHEN** LLM 返回 MoveToward 动作
- **AND** 目标位置超出 Agent 视野范围
- **THEN** 验证 SHALL 失败
- **AND** 系统 SHALL 拒绝该动作
- **AND** 候选动作列表 SHALL 不包含此动作

#### Scenario: 目标位于不可通行地形

- **WHEN** LLM 返回 MoveToward 动作
- **AND** 目标位置为山地或水域等不可通行地形
- **THEN** 验证 SHALL 失败
- **AND** 系统 SHALL 拒绝该动作

#### Scenario: 目标位于建筑之上

- **WHEN** LLM 返回 MoveToward 动作
- **AND** 目标位置已有建筑
- **THEN** 验证 SHALL 失败
- **AND** 系统 SHALL 拒绝该动作

### Requirement: MoveToward 执行逻辑

World::apply_action() SHALL 正确处理 MoveToward 动作类型。

#### Scenario: 执行单步移动

- **WHEN** Agent 执行 MoveToward 动作
- **AND** 目标位置有效但非当前格
- **THEN** 系统 SHALL 计算从当前位置到目标的方向
- **AND** 系统 SHALL 调用 handle_move 执行单步移动
- **AND** 移动后 Agent 位置 SHALL 更新为新坐标
- **AND** 叙事事件 SHALL 记录移动信息

#### Scenario: 目标即当前位置

- **WHEN** Agent 执行 MoveToward 动作
- **AND** 目标位置等于当前 Agent 位置
- **THEN** 系统 SHALL 无操作
- **AND** 返回 Success 结果

#### Scenario: 移动路径被阻挡

- **WHEN** Agent 执行 MoveToward 动作
- **AND** 计算出的移动方向上有不可通行地形
- **THEN** 系统 SHALL 尝试替代方向（顺时针优先）
- **AND** 若所有方向都被阻挡，SHALL 返回 Blocked 结果
- **AND** Agent 位置 SHALL 不变

### Requirement: LLM 响应解析

决策管道 SHALL 正确解析 LLM 返回的 MoveToward 动作。

#### Scenario: 解析带坐标的 MoveToward

- **WHEN** LLM 返回 JSON 包含 action_type: "MoveToward" 或 "move_toward" 或 "移动到"
- **AND** params.target 包含有效坐标
- **THEN** 系统 SHALL 解析为 ActionType::MoveToward { target: Position }
- **AND** target 坐标 SHALL 正确提取

#### Scenario: 解析多种坐标格式

- **WHEN** LLM 返回各种坐标格式
- **AND** 格式可能为 { x: 130, y: 125 } 或 [130, 125] 或 "130,125"
- **THEN** 系统 SHALL 尝试多种解析方式
- **AND** 若解析失败，SHALL 使用当前 Agent 位置作为默认值

### Requirement: 方向计算算法

系统 SHALL 提供从当前位置到目标位置的方向计算。

#### Scenario: 计算主要方向

- **WHEN** 计算从 (x1, y1) 到 (x2, y2) 的方向
- **AND** |dx| >= |dy|
- **THEN** 方向 SHALL 为 East（dx > 0）或 West（dx < 0）

#### Scenario: 计算次要方向

- **WHEN** 计算从 (x1, y1) 到 (x2, y2) 的方向
- **AND** |dy| > |dx|
- **THEN** 方向 SHALL 为 South（dy > 0）或 North（dy < 0）

### Requirement: 与现有 Move 动作兼容

MoveToward 动作 SHALL 与现有 Move { direction } 动作保持兼容。

#### Scenario: 候选动作共存

- **WHEN** 规则引擎生成候选动作
- **THEN** MoveToward 动作和 Move 动作 SHALL 可以同时存在于候选列表
- **AND** LLM SHALL 可以选择任一动作类型

#### Scenario: MoveToward 转换为 Move

- **WHEN** MoveToward 动作通过验证
- **THEN** 执行阶段 SHALL 内部转换为 Move { direction } 执行
- **AND** 最终结果 SHALL 与 Move 动作一致