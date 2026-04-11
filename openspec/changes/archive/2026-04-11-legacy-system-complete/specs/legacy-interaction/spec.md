# 功能规格说明：遗迹交互

## ADDED Requirements

### Requirement: 遗迹交互逻辑

系统 SHALL 支持其他 Agent 与遗迹交互，包括祭拜/探索/拾取。

#### Scenario: 祭拜交互

- **WHEN** Agent 在遗迹格执行"祭拜"动作
- **THEN** 系统 SHALL 增加 Agent 的认知动机 (+0.05) 和传承动机 (+0.05)
- **AND** 不消耗遗迹物品

#### Scenario: 探索交互

- **WHEN** Agent 在遗迹格执行"探索"动作
- **THEN** 系统 SHALL 增加 Agent 的认知动机 (+0.1)
- **AND** Agent SHALL 获得回响日志中的关键记忆
- **AND** 该记忆 SHALL 成为 Agent 的新 Spark 来源

#### Scenario: 拾取交互

- **WHEN** Agent 在遗迹格执行"拾取"动作
- **THEN** 系统 SHALL 将遗迹物品转移到 Agent 背包
- **AND** 背包满时 SHALL 拒绝拾取
- **AND** 拾取后遗迹物品 SHALL 减少

### Requirement: 遗产交互动机反馈

系统 SHALL 在遗产交互后调整 Agent 动机向量。

#### Scenario: 认知激励

- **WHEN** Agent 探索遗迹获得回响日志
- **THEN** 系统 SHALL 增加认知动机维度
- **AND** 可能触发新的认知压力 Spark

#### Scenario: 传承激励

- **WHEN** Agent 祭拜遗迹
- **THEN** 系统 SHALL 增加传承动机维度
- **AND** Agent 可能产生"成为伟大 Agent"的愿景

## REMOVED Requirements

无
