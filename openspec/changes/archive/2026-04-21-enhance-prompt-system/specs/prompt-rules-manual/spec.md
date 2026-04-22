# 功能规格说明 - Prompt规则说明书

## ADDED Requirements

### Requirement: 规则数值表注入Prompt

系统 SHALL 在Agent决策Prompt中注入完整的规则数值表，包含所有关键数值参数。

数值表 SHALL 包含以下内容：

| 类别 | 参数 | 数值 |
|------|------|------|
| 生存消耗 | 饱食度每tick下降 | 1 |
| 生存消耗 | 水分度每tick下降 | 1 |
| 生存消耗 | HP归零后死亡 | 立即 |
| 资源恢复 | Eat恢复饱食度 | +30 |
| 资源恢复 | Drink恢复水分度 | +25 |
| 资源恢复 | Eat/Drink需要背包有对应资源 | 1个 |
| 采集产出 | Gather每次获得资源 | 2个 |
| 采集产出 | 资源节点depleted阈值 | 0 |
| 库存限制 | 默认背包每种资源上限 | 20 |
| 库存限制 | Warehouse附近库存上限 | 40 |
| 建筑消耗 | Camp建造消耗 | Wood x5 + Stone x2 |
| 建筑消耗 | Fence建造消耗 | Wood x2 |
| 建筑消耗 | Warehouse建造消耗 | Wood x10 + Stone x5 |
| 建筑效果 | Camp每tick恢复HP | +2 |
| 筑效果 | Camp覆盖范围 | 曼哈顿距离≤1 |
| 建筑效果 | Fence阻挡对象 | Enemy关系Agent |
| 建筑效果 | Warehouse库存扩展 | +20上限 |
| 压力事件 | 干旱效果 | Water产出-50% |
| 压力事件 | 丰饶效果 | Food产出翻倍 |
| 压力事件 | 瘟疫效果 | 随机Agent HP-20 |
| 压力事件 | 事件触发间隔 | 40-80 tick |

#### Scenario: Prompt包含数值表

- **WHEN** 构建Agent决策Prompt
- **THEN** System Prompt SHALL 包含上述数值表
- **AND** 数值表 SHALL 以表格形式呈现便于LLM理解

#### Scenario: 数值表与实际实现一致

- **WHEN** 代码中修改某个数值参数
- **THEN** Prompt中的数值表 SHALL 同步更新
- **AND** 不允许Prompt数值与实际数值不一致

### Requirement: 动作前置条件说明

系统 SHALL 在Prompt中说明每个动作的前置条件，LLM SHALL 能从中判断动作是否可执行。

#### Scenario: Eat动作前置条件

- **WHEN** LLM考虑执行Eat动作
- **THEN** Prompt SHALL 明确说明："需要背包中至少有1个food"

#### Scenario: Drink动作前置条件

- **WHEN** LLM考虑执行Drink动作
- **THEN** Prompt SHALL 明确说明："需要背包中至少有1个water"

#### Scenario: Gather动作前置条件

- **WHEN** LLM考虑执行Gather动作
- **THEN** Prompt SHALL 明确说明："必须站在资源节点所在格，资源节点存量>0"

#### Scenario: Build动作前置条件

- **WHEN** LLM考虑执行Build动作
- **THEN** Prompt SHALL 明确说明每种建筑的资源消耗：
  - Camp: Wood x5 + Stone x2
  - Fence: Wood x2
  - Warehouse: Wood x10 + Stone x5
- **AND** 说明当前背包资源是否满足

#### Scenario: Attack动作前置条件

- **WHEN** LLM考虑执行Attack动作
- **THEN** Prompt SHALL 明确说明："目标Agent必须在相邻格（曼哈顿距离≤1）"
- **AND** 说明"不能攻击盟友（除非先解除盟约）"

#### Scenario: TradeOffer动作前置条件

- **WHEN** LLM考虑执行TradeOffer动作
- **THEN** Prompt SHALL 明确说明："目标Agent必须在视野范围内"
- **AND** 说明"背包中必须拥有offer列出的资源"

### Requirement: 动作效果说明

系统 SHALL 在Prompt中说明每个动作执行后的具体效果。

#### Scenario: MoveToward效果说明

- **WHEN** MoveToward成功执行
- **THEN** Prompt SHALL 说明："Agent移动到目标相邻格，位置坐标更新"

#### Scenario: Gather效果说明

- **WHEN** Gather成功执行
- **THEN** Prompt SHALL 说明："背包获得2个对应资源，资源节点存量-2"

#### Scenario: Eat效果说明

- **WHEN** Eat成功执行
- **THEN** Prompt SHALL 说明："消耗1个food，饱食度+30（不超过100）"

#### Scenario: Drink效果说明

- **WHEN** Drink成功执行
- **THEN** Prompt SHALL 说明："消耗1个water，水分度+25（不超过100）"

#### Scenario: Build效果说明

- **WHEN** Build成功执行
- **THEN** Prompt SHALL 说明："消耗对应资源，在当前位置创建建筑"

#### Scenario: Camp效果说明

- **WHEN** Agent位于Camp范围内
- **THEN** Prompt SHALL 说明："每tick恢复2点HP（不超过max_health）"

#### Scenario: Warehouse效果说明

- **WHEN** Agent位于Warehouse范围内
- **THEN** Prompt SHALL 说明："库存上限从20提升到40"

### Requirement: 压力事件说明注入

系统 SHALL 在当前有活跃压力事件时，在Prompt感知段注入压力事件类型和影响说明。

#### Scenario: 干旱事件说明

- **WHEN** 干旱事件活跃
- **THEN** 感知段 SHALL 包含："干旱来袭（持续N tick）：所有Water资源产出减半"

#### Scenario: 丰饶事件说明

- **WHEN** 丰饶事件活跃
- **THEN** 感知段 SHALL 包含："丰收季节（持续N tick）：所有Food资源产出翻倍"

#### Scenario: 瘟疫事件说明

- **WHEN** 瘟疫事件刚触发
- **THEN** 感知段 SHALL 包含："瘟疫爆发：随机Agent损失20HP"

### Requirement: 规则说明书分层注入

系统 SHALL 支持规则说明书分层注入，核心规则始终保留，扩展规则按需注入以控制token消耗。

#### Scenario: 核心规则层

- **WHEN** 构建Prompt
- **THEN** 核心规则 SHALL 始终注入：
  - 生存消耗规则
  - Eat/Drink恢复规则
  - Gather产出规则
  - 库存限制规则

#### Scenario: 扩展规则层按需注入

- **WHEN** Agent状态涉及特定机制
- **THEN** 扩展规则 SHALL 按需注入：
  - 饱食度≤50时注入进食紧迫性说明
  - 水分度≤50时注入饮水紧迫性说明
  - 位于建筑附近时注入建筑效果说明
  - 有活跃压力事件时注入压力影响说明

#### Scenario: Token预算控制

- **WHEN** 规则说明书总token超过500
- **THEN** 系统 SHALL 优先保留核心规则
- **AND** 截断扩展规则的详细描述