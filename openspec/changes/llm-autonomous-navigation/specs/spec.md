# 增量规格：行为变更

## MODIFIED Requirements

### Requirement: 硬约束过滤支持 MoveToward

规则引擎的 `filter_hard_constraints` SHALL 支持 MoveToward 动作的验证。

#### Scenario: MoveToward 通过硬约束

- **WHEN** LLM 返回 MoveToward { target: Position }
- **AND** 目标坐标在地图有效范围内
- **AND** 目标坐标在 Agent 视野范围内
- **THEN** MoveToward SHALL 被加入候选动作列表

#### Scenario: MoveToward 坐标无效

- **WHEN** LLM 返回 MoveToward { target: Position }
- **AND** 目标坐标 x 或 y 为负数或超出地图边界
- **THEN** 验证 SHALL 失败
- **AND** 候选动作列表 SHALL 不包含此动作

### Requirement: 动作执行器分发支持 MoveToward

World::apply_action() SHALL 正确分发 MoveToward 动作到对应的处理器。

#### Scenario: 分发 MoveToward 动作

- **WHEN** apply_action 收到 ActionType::MoveToward { target }
- **THEN** 系统 SHALL 调用 handle_move_toward(target) 方法
- **AND** 返回结果 SHALL 与 Move 动作使用相同的 ActionResult 类型

### Requirement: 感知摘要增强资源显示

DecisionPipeline.build_perception_summary() SHALL 在资源信息中包含方向和距离。

#### Scenario: 感知摘要包含方向

- **WHEN** 调用 build_perception_summary(world_state)
- **AND** world_state.resources_at 包含资源位置
- **THEN** 输出 SHALL 为每个资源包含:
  - 坐标 "(x, y)"
  - 类型 "Food/Water/Wood/Stone/Iron"
  - 数量 "xN"
  - 方向 "[东北方向]"
  - 距离 "距N格"

#### Scenario: 感知摘要按生存优先排序

- **WHEN** Agent 饱食度 <= 50
- **THEN** Food 资源 SHALL 排序在感知摘要最前面
- **AND** Water 资源 SHALL 排序在 Food 之后

### Requirement: 候选动作生成增强

规则引擎 SHALL 在候选动作列表中同时包含 Move 和 MoveToward 选项。

#### Scenario: 同时生成 Move 和 MoveToward

- **WHEN** 规则引擎生成候选动作
- **AND** 视野内有资源
- **THEN** 候选列表 SHALL 包含:
  - Move { direction: 朝向最近资源的方向 }
  - MoveToward { target: 最近资源的位置 }
  - Gather { resource: 当前位置资源类型 }（如果当前位置有资源）

#### Scenario: MoveToward 指向多个资源

- **WHEN** 视野内有多个资源
- **THEN** 候选列表 SHALL 包含指向最近 3 个资源的 MoveToward 动作
- **AND** 每个动作的 target SHALL 为对应资源的准确坐标

## ADDED Requirements

### Requirement: LLM 动作类型解析扩展

DecisionPipeline.parse_action_type() SHALL 支持解析 MoveToward 动作。

#### Scenario: 解析英文格式

- **WHEN** LLM 返回 action_type 为 "MoveToward" 或 "move_toward"
- **AND** params 包含 target 字段
- **THEN** 系统 SHALL 解析为 ActionType::MoveToward { target: Position }

#### Scenario: 解析中文格式

- **WHEN** LLM 返回 action_type 为 "移动到" 或 "前往"
- **AND** params 包含目标坐标
- **THEN** 系统 SHALL 解析为 ActionType::MoveToward { target: Position }

#### Scenario: 解析坐标对象格式

- **WHEN** params.target 为 { x: 130, y: 125 }
- **THEN** 系统 SHALL 提取 Position::new(130, 125)

#### Scenario: 解析坐标数组格式

- **WHEN** params.target 为 [130, 125]
- **THEN** 系统 SHALL 提取 Position::new(130, 125)

#### Scenario: 解析坐标字符串格式

- **WHEN** params.target 为 "130,125" 或 "(130, 125)"
- **THEN** 系统 SHALL 尝试解析坐标数字
- **AND** 解析失败时 SHALL 使用 Agent 当前位置作为默认值