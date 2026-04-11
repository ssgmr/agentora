# 功能规格说明：策略库系统

> **设计借鉴**: 本规格借鉴 Hermes Agent 的 Skills 自我改进机制，适配 Agentora 的动机向量决策场景。

## ADDED Requirements

### Requirement: 策略库架构

系统 SHALL 为每个Agent维护决策策略库（StrategyHub），存储成功决策的可复用策略。策略库使用 Markdown + YAML frontmatter 格式，支持 progressive disclosure 分级披露。

```
┌─────────────────────────────────────────────────────────────────────────┐
│              StrategyHub 策略库架构                                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   目录结构:                                                              │
│   ~/.agentora/agents/<agent_id>/strategies/                              │
│   ├── resource_pressure/                                                 │
│   │   └── STRATEGY.md                                                    │
│   │       ├── references/ (条件说明文档)                                  │
│   │       └── logs/ (执行记录案例)                                        │
│   ├── social_pressure/                                                   │
│   │   └── STRATEGY.md                                                    │
│   ├── explore_pressure/                                                  │
│   │   └── STRATEGY.md                                                    │
│   └── ...                                                                │
│                                                                         │
│   STRATEGY.md 格式:                                                      │
│   ---                                                                    │
│   spark_type: resource_pressure                                          │
│   success_rate: 0.85                                                     │
│   use_count: 12                                                          │
│   last_used_tick: 1250                                                   │
│   created_tick: 800                                                      │
│   deprecated: false                                                      │
│   motivation_delta: [+0.1, -0.05, +0.02, 0, 0, 0]                        │
│   ---                                                                    │
│   # 策略内容...                                                           │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Scenario: 策略目录命名

- **WHEN** 创建新策略
- **THEN** 目录名 SHALL 使用 Spark 类型命名（如 resource_pressure、social_pressure）
- **AND** 目录名 SHALL 仅使用小写字母、数字、下划线
- **AND** 目录名 SHALL 不超过 64 字符

#### Scenario: STRATEGY.md YAML Frontmatter

- **WHEN** 创建或更新 STRATEGY.md
- **THEN** 文件 SHALL 以 YAML frontmatter 开头（--- 包裹）
- **AND** frontmatter SHALL 包含必填字段:
  - `spark_type`: 策略适用的 Spark 类型
  - `success_rate`: 成功率（0.0-1.0）
  - `use_count`: 使用次数
  - `last_used_tick`: 最后使用的 tick
- **AND** frontmatter MAY 包含可选字段:
  - `created_tick`: 创建时间
  - `deprecated`: 是否已废弃
  - `motivation_delta`: 预期动机变化（6维数组）
  - `conditions`: 适用条件（JSON）

#### Scenario: 策略内容结构

- **WHEN** 编写 STRATEGY.md 正文
- **THEN** 内容 SHALL 包含:
  1. 策略概述（何时触发、预期结果）
  2. 决策步骤（推荐动作序列）
  3. 成功条件（什么情况下策略有效）
  4. 失败应对（策略失效时的备选方案）

### Requirement: Progressive Disclosure 分级披露

系统 SHALL 对策略库实施三级披露机制，控制 Prompt token 消耗。

#### Scenario: Tier 1 策略列表

- **WHEN** 构建决策 Prompt 需要扫描策略库
- **THEN** 系统 SHALL 仅加载策略 metadata（YAML frontmatter）
- **AND** 返回格式为: `[spark_type] success_rate=X.XX, uses=N`
- **AND** Tier 1 内容 SHALL 不超过 200 chars

#### Scenario: Tier 2 策略详情

- **WHEN** Spark 类型匹配某策略且需要详情
- **THEN** 系统 SHALL 加载完整 STRATEGY.md 内容
- **AND** 不加载 references/ 和 logs/ 子目录
- **AND** Tier 2 内容 SHALL 不超过 400 chars

#### Scenario: Tier 3 执行案例

- **WHEN** 需要参考历史执行案例
- **THEN** 系统 MAY 加载 logs/ 目录中的案例文件
- **AND** 仅加载最近 1-2 个案例
- **AND** Tier 3 内容 SHALL 不超过 300 chars

### Requirement: 策略创建触发

系统 SHALL 在以下条件触发策略创建，将成功决策转化为可复用策略。

#### Scenario: 成功决策触发创建

- **WHEN** Agent 执行决策后 Echo 反馈为"成功"
- **AND** 决策涉及 ≥3 个候选动作筛选
- **AND** 动机对齐度 > 0.7（决策与动机向量高度对齐）
- **THEN** 系统 SHALL 自动创建策略
- **AND** 策略名 SHALL 使用本次 Spark 类型

#### Scenario: 探索发现触发创建

- **WHEN** Agent 探索发现新工作流或有效策略
- **AND** 发现的事件 importance > 0.8
- **THEN** 系统 SHALL 创建策略记录该发现

#### Scenario: 策略创建工具

- **WHEN** 触发策略创建
- **THEN** 系统 SHALL 使用 strategy 工具:
```
strategy(
  action="create",
  name="resource_pressure_trade_nearby",
  content="---
spark_type: resource_pressure
success_rate: 1.0
use_count: 1
---
# 策略: 与邻近高信任Agent交易

触发条件: 资源库存 < 阈值
推荐动作:
1. 感知视野内Agent
2. 选择 trust > 0.5 的Agent
3. 发起交易提议
...",
)
```

#### Scenario: 策略内容安全扫描

- **WHEN** 创建策略内容
- **THEN** 系统 SHALL 执行安全扫描（与 ChronicleStore 相同规则）
- **AND** 扫描威胁模式: prompt injection、role hijack、rule bypass、invisible unicode
- **AND** 检测到威胁 SHALL 拒绝创建

### Requirement: 策略自我改进（Patch）

系统 SHALL 支持策略使用中发现问题时的即时修正机制，不需要等待用户请求。

#### Scenario: 策略问题检测

- **WHEN** 使用策略后 Echo 反馈为"失败"或"后悔"
- **THEN** 系统 SHALL 检测策略问题类型:
  - `outdated`: 策略条件已变化（如信任值已下降）
  - `incomplete`: 策略步骤有遗漏
  - `wrong`: 策略导致负面结果

#### Scenario: 策略立即修正

- **WHEN** 检测到策略问题
- **THEN** 系统 SHALL 立即执行 patch（不等待下次决策）
- **AND** 使用 strategy 工具:
```
strategy(
  action="patch",
  name="resource_pressure_trade_nearby",
  find="选择 trust > 0.5 的Agent",
  replace="选择 trust > 0.5 且距离 < 3 的Agent",
)
```

#### Scenario: Patch 更新 Frontmatter

- **WHEN** 执行策略 patch
- **THEN** 系统 SHALL 同时更新 YAML frontmatter:
  - `success_rate`: 根据最近使用情况重新计算
  - `last_used_tick`: 更新为当前 tick

#### Scenario: Patch 记录到执行日志

- **WHEN** 策略 patch 完成
- **THEN** 系统 SHALL 将修正记录写入 logs/ 目录
- **AND** 文件名: `<tick>_patch.md`
- **AND** 内容: 修正原因、修正内容、修正后预期

### Requirement: 策略衰减机制

系统 SHALL 对长期不适用或成功率下降的策略执行衰减，防止未维护策略成为决策负担。

#### Scenario: 成功率衰减

- **WHEN** 每 50 tick 到达
- **THEN** 系统 SHALL 对所有策略执行:
  - `success_rate = success_rate * 0.95`
- **AND** 衰减仅在策略未被使用时生效

#### Scenario: 策略使用时成功率更新

- **WHEN** 策略被使用后 Echo 反馈为"成功"
- **THEN** 系统 SHALL 更新:
  - `success_rate = (success_rate * use_count + 1.0) / (use_count + 1)`
  - `use_count += 1`
- **AND** 这会抵消衰减效果

#### Scenario: 策略废弃标记

- **WHEN** 策略 success_rate < 0.3
- **THEN** 系统 SHALL 标记 `deprecated: true`
- **AND** deprecated 策略 SHALL 不出现在 Tier 1 列表
- **AND** deprecated 策略 SHALL 仍可被 strategy_view 查询

#### Scenario: 策略删除

- **WHEN** 策略 deprecated=true 且连续 100 tick 未使用
- **THEN** 系统 MAY 自动删除策略目录
- **AND** 删除前 SHALL 记录到全局策略审计日志

### Requirement: 策略检索与应用

系统 SHALL 在决策构建 Prompt 时检索匹配当前 Spark 的策略。

#### Scenario: Spark 类型匹配

- **WHEN** 当前 Spark 类型为 "resource_pressure"
- **THEN** 系统 SHALL 检索 strategies/resource_pressure/STRATEGY.md
- **AND** 若目录不存在 SHALL 检索其他可能匹配的策略

#### Scenario: 多策略候选

- **WHEN** 同一 Spark 类型存在多个策略目录（如 resource_pressure_gather, resource_pressure_trade）
- **THEN** 系统 SHALL 按 success_rate 降序排序
- **AND** 仅加载 success_rate 最高的策略（Tier 2）
- **AND** 其他策略仅在 Tier 1 列表显示 metadata

#### Scenario: 策略内容注入 Prompt

- **WHEN** 策略匹配成功
- **THEN** 系统 SHALL 将策略内容注入 Prompt:
```
<strategy-context>
[系统注：以下是历史成功策略参考]

策略: resource_pressure_trade_nearby (成功率 85%, 使用12次)
条件: 资源库存 < 阈值
推荐: 感知视野→选择信任Agent→发起交易

</strategy-context>
```

#### Scenario: 策略与候选动作对齐

- **WHEN** LLM 生成候选动作后
- **THEN** 系统 SHALL 计算候选与策略推荐的对齐度
- **AND** 对齐度高的候选 SHALL 在动机加权时获得额外 +0.1 boost

### Requirement: 策略工具接口

系统 SHALL 提供完整的策略管理工具接口。

#### Scenario: strategy 工具定义

- **WHEN** 定义策略工具
- **THEN** 系统 SHALL 提供以下 actions:
```
strategy(
  action: str,        # create/patch/list/view/delete/rename
  name: str,          # 策略名（目录名）
  content: str,       # 策略内容（仅 create 时必填）
  find: str,          # 待查找文本（仅 patch 时必填）
  replace: str,       # 替换文本（仅 patch 时必填）
  target: str,        # 目标文件（默认 STRATEGY.md，可选 references/xxx.md）
)
```

#### Scenario: strategy_list

- **WHEN** action="list"
- **THEN** 系统 SHALL 返回所有策略 Tier 1 metadata
- **AND** 格式: `[spark_type] success_rate=X.XX, uses=N, deprecated=Y/N`

#### Scenario: strategy_view

- **WHEN** action="view" 且 name 有效
- **THEN** 系统 SHALL 返回完整策略内容（Tier 2）
- **AND** 若 target="logs/xxx" SHALL 返回 Tier 3 执行案例

#### Scenario: strategy_delete

- **WHEN** action="delete" 且 name 有效
- **THEN** 系统 SHALL 删除策略目录
- **AND** 记录删除到审计日志
- **AND** 仅允许删除 use_count < 3 或 deprecated=true 的策略

### Requirement: 策略与动机向量联动

系统 SHALL 使策略库与动机向量引擎联动，策略执行结果影响动机权重。

#### Scenario: 策略成功强化动机

- **WHEN** 策略执行成功（Echo 正反馈）
- **THEN** 系统 SHALL 按策略 frontmatter 的 motivation_delta 调整动机向量
- **AND** 调整幅度 SHALL 乘以策略 success_rate 作为权重

#### Scenario: 策略失败弱化动机

- **WHEN** 策略执行失败
- **THEN** 系统 SHALL 反向调整动机向量（motivation_delta 取负）
- **AND** 调整幅度 SHALL 乘以 0.5（失败影响较小）

#### Scenario: 策略创建时记录动机变化

- **WHEN** 创建策略
- **THEN** 系统 SHALL 从本次决策的 Action.motivation_delta 提取并记录到 frontmatter
- **AND** motivation_delta SHALL 归一化到 [-0.2, +0.2] 范围

## P2P 同步边界

策略库属于本地私有数据，不通过 P2P 同步。策略可能作为遗产的一部分（摘要形式）在 Agent 死亡时广播，但完整策略内容不传输。

---

> **设计红线**: 策略库是 Agent 智能演化的核心机制，必须保证:
> 1. 策略创建和 patch 是 Agent 自主行为，不需要玩家干预
> 2. 衰减机制防止策略膨胀成为负担
> 3. 策略内容可追溯，审计日志支持事后分析