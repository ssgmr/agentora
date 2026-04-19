# 功能规格说明

## ADDED Requirements

### Requirement: Agent 动作闪烁效果

系统 SHALL 在 Agent 执行采集动作时产生短暂闪烁效果，提示当前行为。

#### Scenario: 采集动作触发闪烁

- **WHEN** Agent 执行 Gather 类型的动作
- **THEN** Agent 节点产生绿色闪烁效果
- **AND** 闪烁持续约 0.3 秒
- **AND** 闪烁效果为透明度脉动（alpha 在 0.4~1.0 之间波动）

#### Scenario: 闪烁效果实现方式

- **WHEN** Agent 需要显示闪烁效果
- **THEN** 系统使用 sin(_effect_time * 8) 计算透明度
- **AND** 通过 sprite.modulate.a 设置透明度
- **AND** 在 _physics_process 中持续更新

#### Scenario: 多次采集连续闪烁

- **WHEN** Agent 连续执行多次采集动作
- **THEN** 每次采集独立触发闪烁
- **AND** 闪烁效果不叠加，每次重置计时器

#### Scenario: Agent 未执行动作时无闪烁

- **WHEN** Agent 当前动作非 Gather 类型
- **THEN** Agent 无闪烁效果
- **AND** sprite.modulate.a 保持默认值 1.0

#### Scenario: 闪烁结束恢复正常

- **WHEN** 闪烁计时器归零
- **THEN** Agent 恢复正常显示状态
- **AND** sprite.modulate.a 设为 1.0