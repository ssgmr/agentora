# 功能规格说明 - Action 反馈 Schema

## ADDED Requirements

### Requirement: ActionResult 结构化格式

系统 SHALL 定义结构化的 ActionResult 格式，替代当前的字符串格式。

ActionResult SHALL 包含：
- **ActionSuccess**: 动作类型、数值变化列表、位置变化
- **ActionBlocked**: 错误类型、原因描述、建议动作

每个 ActionResult 变体 SHALL 可序列化为 JSON，供前端展示。

#### Scenario: Gather 成功反馈结构化

- **WHEN** Gather 成功
- **THEN** ActionResult SHALL 包含：
  ```json
  {
    "type": "success",
    "action": "Gather",
    "changes": [
      {"field": "inventory.food", "before": 3, "after": 5},
      {"field": "node.amount", "before": 100, "after": 98}
    ]
  }
  ```

#### Scenario: MoveToward 失败反馈结构化

- **WHEN** MoveToward 目标不相邻
- **THEN** ActionResult SHALL 包含：
  ```json
  {
    "type": "blocked",
    "error_code": "invalid_target",
    "reason": "目标距离3格，只能移动到相邻格",
    "suggestion": {"action": "MoveToward", "direction": "east"}
  }
  ```

### Requirement: 反馈生成器统一

系统 SHALL 创建 ActionFeedbackGenerator 模块：
- 接收 ActionResult
- 生成 LLM 可理解的文本反馈
- 生成前端 UI 显示格式

#### Scenario: 反馈生成器处理成功

- **WHEN** ActionFeedbackGenerator.generate(action_result)
- **THEN** 返回文本："采集成功：获得 food x2，节点剩余 98，背包 3→5"

#### Scenario: 反馈生成器处理失败

- **WHEN** ActionResult 是 Blocked
- **THEN** 返回文本包含原因和建议
- **AND** 前端显示红色提示

### Requirement: handler 使用结构化格式

所有动作 handler SHALL 返回结构化 ActionResult，不再使用字符串。

#### Scenario: handle_gather 返回结构化

- **WHEN** handle_gather() 成功
- **THEN** 返回 ActionResult::Success { changes: [...] }
- **AND** 不返回 "gather:foodx2,node_remain:98" 字符串