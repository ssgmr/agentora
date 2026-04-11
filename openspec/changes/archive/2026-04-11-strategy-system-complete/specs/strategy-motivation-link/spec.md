# 功能规格说明：策略与动机联动

## ADDED Requirements

### Requirement: 策略成功强化动机

系统 SHALL 在策略执行成功时，按策略的 motivation_delta 调整动机向量。

#### Scenario: 成功调整动机

- **WHEN** 策略执行成功（Echo 正反馈）
- **THEN** 系统 SHALL 按策略 frontmatter 的 motivation_delta 调整动机向量
- **AND** 调整幅度 SHALL 乘以策略 success_rate 作为权重
- **AND** 调整后 SHALL 归一化到 [0.0, 1.0]

### Requirement: 策略失败弱化动机

系统 SHALL 在策略执行失败时，反向调整动机向量。

#### Scenario: 失败反向调整

- **WHEN** 策略执行失败
- **THEN** 系统 SHALL 反向调整动机向量（motivation_delta 取负）
- **AND** 调整幅度 SHALL 乘以 0.5（失败影响较小）

### Requirement: 策略创建时记录动机变化

系统 SHALL 在创建策略时记录动机变化。

#### Scenario: 提取动机 delta

- **WHEN** 创建策略
- **THEN** 系统 SHALL 从本次决策的 Action.motivation_delta 提取
- **AND** 归一化到 [-0.2, +0.2] 范围
- **AND** 记录到 frontmatter 的 motivation_delta 字段

## REMOVED Requirements

无
