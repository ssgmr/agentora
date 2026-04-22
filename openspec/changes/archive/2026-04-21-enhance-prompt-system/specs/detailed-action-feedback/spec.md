# 功能规格说明 - 详细动作反馈

## ADDED Requirements

### Requirement: 动作执行失败详细反馈

动作执行失败时，系统 SHALL 返回详细的失败原因，包含具体数值差异和条件检查结果。

#### Scenario: Build失败反馈资源差异

- **WHEN** Agent尝试Build Camp
- **AND** 背包资源不足（只有Wood x2，需要Wood x5 + Stone x2）
- **THEN** feedback SHALL 返回："Build失败：资源不足。需要Wood x5 + Stone x2，背包中只有Wood x2 + Stone x0"

#### Scenario: Eat失败反馈资源缺失

- **WHEN** Agent尝试Eat
- **AND** 背包中无food
- **THEN** feedback SHALL 返回："Eat失败：背包中没有food。当前背包：wood x3, stone x2"

#### Scenario: Drink失败反馈资源缺失

- **WHEN** Agent尝试Drink
- **AND** 背包中无water
- **THEN** feedback SHALL 返回："Drink失败：背包中没有water"

#### Scenario: Gather失败反馈位置错误

- **WHEN** Agent尝试Gather wood
- **AND** 当前位置无wood资源节点
- **THEN** feedback SHALL 返回："Gather失败：当前位置(120,115)没有wood资源。请先MoveToward到资源位置"

#### Scenario: Gather失败反馈资源耗尽

- **WHEN** Agent尝试Gather food
- **AND** 当前位置food资源节点存量=0
- **THEN** feedback SHALL 返回："Gather失败：当前位置food资源已耗尽。请寻找其他资源节点"

#### Scenario: Attack失败反馈距离限制

- **WHEN** Agent尝试Attack目标Agent
- **AND** 目标Agent不在相邻格（曼哈顿距离>1）
- **THEN** feedback SHALL 返回："Attack失败：目标Agent距离过远（距离3格）。Attack只能对相邻格Agent执行"

#### Scenario: Attack失败反馈盟友限制

- **WHEN** Agent尝试Attack目标Agent
- **AND** 目标Agent是盟友关系
- **THEN** feedback SHALL 返回："Attack失败：不能攻击盟友Agent。若要攻击，需先解除盟约"

#### Scenario: TradeOffer失败反馈资源不足

- **WHEN** Agent尝试TradeOffer
- **AND** 背包中offer的资源数量不足
- **THEN** feedback SHALL 返回："TradeOffer失败：背包资源不足。Offer需要wood x5，背包只有wood x3"

### Requirement: 动作执行成功详细反馈

动作执行成功时，系统 SHALL 返回详细的效果描述，包含数值变化。

#### Scenario: Gather成功反馈

- **WHEN** Agent成功Gather wood
- **THEN** feedback SHALL 返回："Gather成功：获得wood x2。当前位置wood资源剩余x48。背包wood增至x5"

#### Scenario: Eat成功反馈

- **WHEN** Agent成功Eat
- **THEN** feedback SHALL 返回："Eat成功：消耗food x1，饱食度+30（从45增至75）。背包food剩余x2"

#### Scenario: Drink成功反馈

- **WHEN** Agent成功Drink
- **THEN** feedback SHALL 返回："Drink成功：消耗water x1，水分度+25（从60增至85）。背包water剩余x1"

#### Scenario: Build成功反馈

- **WHEN** Agent成功Build Camp
- **THEN** feedback SHALL 返回："Build成功：消耗Wood x5 + Stone x2，在(125,120)创建Camp。Camp效果：每tick恢复2HP"

#### Scenario: MoveToward成功反馈

- **WHEN** Agent成功MoveToward向东
- **THEN** feedback SHALL 返回："MoveToward成功：从(120,115)移动到(121,115)。新位置地形：Forest"

#### Scenario: Camp回血反馈

- **WHEN** Agent位于Camp范围内
- **AND** Agent HP < max_health
- **THEN** feedback SHALL 在tick结束时返回："Camp效果：恢复HP x2（从85增至87）"

### Requirement: 反馈信息注入下一轮Prompt

动作执行反馈 SHALL 作为 `action_feedback` 参数注入下一轮决策Prompt。

#### Scenario: 反馈注入Prompt

- **WHEN** Agent上一轮执行动作
- **THEN** 本轮Prompt SHALL 包含"上次动作结果：{feedback}"
- **AND** feedback SHALL 位于感知段之前，优先级高于其他信息

#### Scenario: 连续失败反馈

- **WHEN** Agent连续多次动作失败
- **THEN** 每轮Prompt SHALL 包含上一轮的失败反馈
- **AND** LLM SHALL 能从失败中学习规则和修正决策

#### Scenario: 无动作时无反馈

- **WHEN** Agent上一轮未执行动作（如Wait）
- **THEN** 本轮Prompt SHALL 不包含action_feedback段

### Requirement: 规则引擎校验失败反馈

规则引擎校验动作时，若校验失败 SHALL 返回具体的校验失败原因。

#### Scenario: 移动边界校验失败

- **WHEN** Agent尝试MoveToward向北
- **AND** 目标位置超出地图边界
- **THEN** 规则引擎 SHALL 返回："MoveToward校验失败：目标位置(120,-1)超出地图边界（0-255）"

#### Scenario: 库存上限校验失败

- **WHEN** Agent尝试Gather wood
- **AND** 背包wood已达上限20
- **THEN** 规则引擎 SHALL 返回："Gather校验失败：背包wood已达上限x20。请先消耗或存储"

#### Scenario: 资源类型校验失败

- **WHEN** Agent尝试Gather gold（不存在资源类型）
- **THEN** 规则引擎 SHALL 返回："Gather校验失败：无效资源类型'gold'。有效类型：food, water, wood, stone, iron"

### Requirement: 反馈信息格式化

反馈信息 SHALL 使用统一格式，便于LLM理解和处理。

#### Scenario: 失败反馈格式

- **WHEN** 动作执行失败
- **THEN** feedback SHALL 使用格式："{动作}失败：{具体原因}。{补充说明}"

#### Scenario: 成功反馈格式

- **WHEN** 动作执行成功
- **THEN** feedback SHALL 使用格式："{动作}成功：{效果描述}。{补充说明}"

#### Scenario: 补充说明可选

- **WHEN** 反馈信息需要补充说明
- **THEN** SHALL 添加当前状态或建议行动
- **AND** 若无需补充说明 SHALL 略去补充部分