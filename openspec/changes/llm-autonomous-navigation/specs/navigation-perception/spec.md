# 导航感知增强

## Purpose

增强感知摘要，在资源信息中显示相对方向和曼哈顿距离，让 LLM 无需计算即可理解导航信息。

## ADDED Requirements

### Requirement: 资源方向提示

感知摘要 SHALL 为每个检测到的资源显示相对于 Agent 的方向。

#### Scenario: 显示单一资源方向

- **WHEN** Agent 位于 (128, 130)
- **AND** 视野范围内有资源位于 (130, 125)
- **THEN** 感知摘要 SHALL 显示: "(130, 125): Food x100 [东北方向，距5格]"

#### Scenario: 显示多个资源方向

- **WHEN** Agent 视野范围内有多个资源
- **THEN** 每个资源条目 SHALL 独立显示方向和距离
- **AND** 方向 SHALL 按照距离排序（最近的优先）

### Requirement: 方向计算规则

方向文字 SHALL 基于曼哈顿距离的坐标差计算。

#### Scenario: 东西方向优先

- **WHEN** |dx| >= |dy|
- **THEN** 方向 SHALL 为 "东" 或 "西"
- **AND** 若 dy != 0，SHALL 附加次要方向，如"东北"或"西南"

#### Scenario: 南北方向优先

- **WHEN** |dy| > |dx|
- **THEN** 方向 SHALL 为 "南" 或 "北"
- **AND** 若 dx != 0，SHALL 附加次要方向，如"东南"或"西北"

#### Scenario: 精确方向映射

- **WHEN** dx > 0 且 dy < 0，方向 SHALL 为 "东北"
- **WHEN** dx > 0 且 dy > 0，方向 SHALL 为 "东南"
- **WHEN** dx < 0 且 dy < 0，方向 SHALL 为 "西北"
- **WHEN** dx < 0 且 dy > 0，方向 SHALL 为 "西南"
- **WHEN** dx > 0 且 dy == 0，方向 SHALL 为 "东"
- **WHEN** dx < 0 且 dy == 0，方向 SHALL 为 "西"
- **WHEN** dx == 0 且 dy > 0，方向 SHALL 为 "南"
- **WHEN** dx == 0 且 dy < 0，方向 SHALL 为 "北"

### Requirement: 距离显示格式

距离 SHALL 使用曼哈顿距离，以简单格式显示。

#### Scenario: 显示曼哈顿距离

- **WHEN** Agent 位于 (x1, y1)
- **AND** 资源位于 (x2, y2)
- **THEN** 距离 SHALL 显示为 "|dx| + |dy|"
- **AND** 格式 SHALL 为 "距N格"

### Requirement: 资源优先级排序

感知摘要中的资源 SHALL 按照优先级排序显示。

#### Scenario: 按生存需求排序

- **WHEN** Agent 饱食度 <= 50
- **THEN** Food 类型资源 SHALL 排序在第一
- **AND** Water 类型资源 SHALL 排序在第二

#### Scenario: 按距离排序

- **WHEN** 有多个相同类型资源
- **THEN** 距离近的资源 SHALL 排序在前

#### Scenario: 按类型优先级

- **WHEN** Agent 没有明确的生存压力
- **THEN** 资源排序 SHALL 为: Food > Water > Wood > Stone > Iron

### Requirement: 感知摘要格式

感知摘要 SHALL 使用清晰的格式显示导航信息。

#### Scenario: 完整感知格式

- **WHEN** 生成感知摘要
- **THEN** 格式 SHALL 遵循以下模板:
```
当前状态：
  饱食度: 50/100 [饥饿中！需要寻找食物]
  水分度: 80/100
位置: (128, 130)
资源分布:
  (130, 125): Food x100 [东北方向，距5格] ← 最近的食物
  (135, 128): Water x50 [东方，距7格]
  (129, 140): Wood x75 [南方，距10格]
```

#### Scenario: 感知摘要 token 限制

- **WHEN** 感知摘要超过 token 预算
- **THEN** 系统 SHALL 按优先级截断资源列表
- **AND** 生存相关资源 SHALL 最后被截断

### Requirement: Agent 方向辅助信息

感知摘要 SHALL 包含 Agent 当前面朝方向的提示（可选）。

#### Scenario: 显示上次移动方向

- **WHEN** Agent 上一次动作为 Move 或 MoveToward
- **THEN** 感知摘要 SHALL 包含"面朝方向: X"
- **AND** 这有助于 LLM 理解"继续前进"或"转身"

### Requirement: 资源数量提示

感知摘要 SHALL 为资源数量提供语义化提示。

#### Scenario: 资源丰富度描述

- **WHEN** 资源数量 >= 100
- **THEN** 描述 SHALL 为 "大量"
- **WHEN** 资源数量 >= 50 且 < 100
- **THEN** 描述 SHALL 为 "中等"
- **WHEN** 资源数量 < 50
- **THEN** 描述 SHALL 为 "少量"

#### Scenario: 显示丰富度

- **WHEN** 资源数量为 150
- **THEN** 显示 SHALL 为 "Food x150 (大量)"