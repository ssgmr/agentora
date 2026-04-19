# 规则决策规范

## Purpose

规范规则引擎的统一架构：ai crate 不包含规则引擎逻辑，FallbackChain 不执行规则兜底，core::RuleEngine 作为唯一的规则决策和 LLM 失败兜底入口。

## Requirements

### Requirement: ai crate 不包含规则引擎逻辑

ai crate SHALL 仅关注 LLM 接入层职责，不包含任何规则决策、动作映射或动机相关的领域逻辑。

#### Scenario: ai::rule_engine 模块不存在

- **WHEN** 查看 `crates/ai/src/` 目录
- **THEN** 系统 SHALL 不包含 `rule_engine.rs` 文件
- **AND** `lib.rs` SHALL 不声明 `pub mod rule_engine`
- **AND** `lib.rs` SHALL 不 re-export `FallbackAction`、`SimpleActionType`、`SimplePosition`、`fallback_decision`

### Requirement: FallbackChain 不执行规则兜底

`FallbackChain` SHALL 仅负责多 Provider 降级尝试，所有 Provider 失败时返回错误，不自行生成兜底动作。

#### Scenario: 所有 Provider 失败返回错误

- **WHEN** `FallbackChain` 中所有 Provider 的 `generate()` 调用均失败
- **THEN** 系统 SHALL 返回 `LlmError::ProviderUnavailable` 错误
- **AND** 错误信息 SHALL 包含 "所有 Provider 都失败"
- **AND** 系统 SHALL 不调用任何规则引擎生成兜底动作

#### Scenario: FallbackChain 不包含规则引擎字段

- **WHEN** 查看 `FallbackChain` 结构体定义
- **THEN** 系统 SHALL 不包含 `use_rule_engine_fallback` 字段
- **AND** 系统 SHALL 不包含 `generate_rule_engine_fallback()` 方法

#### Scenario: FallbackChain 构造函数简化

- **WHEN** 调用 `FallbackChain::new()`
- **THEN** 参数 SHALL 仅包含 `Vec<Box<dyn LlmProvider>>`
- **AND** 参数 SHALL 不包含 `use_rule_engine_fallback: bool`

### Requirement: core::RuleEngine 作为唯一兜底入口

`core::RuleEngine` SHALL 保持作为唯一的规则决策和 LLM 失败兜底入口，其所有方法 SHALL 保持不变。

#### Scenario: DecisionPipeline 使用 core::RuleEngine 兜底

- **WHEN** `DecisionPipeline::execute()` 中 LLM 调用失败
- **THEN** 系统 SHALL 调用 `core::RuleEngine::fallback_action()` 获取兜底动作
- **AND** 兜底动作 SHALL 基于 Agent 当前 6 维动机状态驱动
- **AND** 兜底动作 SHALL 支持完整 13+ 种 ActionType

### Requirement: bridge 创建 FallbackChain

bridge 层创建 `FallbackChain` 时 SHALL 不再传入 `use_rule_engine_fallback` 参数。

#### Scenario: bridge 创建 FallbackChain

- **WHEN** bridge 的 `create_llm_provider()` 创建 Provider 链
- **THEN** 系统 SHALL 使用 `FallbackChain::new(vec![Box::new(openai)])`
- **AND** 系统 SHALL 不传入第二个 bool 参数

### Requirement: NPC生存需求决策

NPC Agent通过RuleEngine决策时 SHALL 优先检查satiety和hydration状态，低于阈值时优先选择满足饮食需求的行为。

#### Scenario: 饥饿时优先采集食物

- **WHEN** NPC satiety ≤ 30
- **AND** 附近有Food ResourceNode
- **THEN** RuleEngine选择Move前往食物节点 + Gather

#### Scenario: 口渴时优先采集水

- **WHEN** NPC hydration ≤ 30
- **AND** 附近有Water ResourceNode
- **THEN** RuleEngine选择Move前往水源 + Gather

#### Scenario: 饥饿且背包有食物

- **WHEN** NPC satiety ≤ 30
- **AND** 背包有Food ≥ 1
- **THEN** RuleEngine选择Wait（消耗食物恢复饱食度）

#### Scenario: 生存需求优先于其他动机

- **WHEN** NPC satiety = 0 或 hydration = 0
- **THEN** 即使最高动机维度非生存，也优先满足饮食需求
