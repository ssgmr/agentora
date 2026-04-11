# 功能规格说明

## ADDED Requirements

### Requirement: 记忆配置加载

系统 SHALL 从 `llm.toml` 的 `[memory]` section 加载记忆系统全部参数配置，包含预算层、存储层、检索层、容量层和 Prompt 约束五大类共 11 个参数。

#### Scenario: 成功加载完整配置

- **WHEN** `llm.toml` 中存在合法的 `[memory]` section 且包含全部 11 个参数
- **THEN** 系统 SHALL 解析配置为 `MemoryConfig` 结构体
- **AND** 所有子模块（TokenBudget、ChronicleStore、ChronicleDB、ShortTerm、PromptBuilder） SHALL 使用配置值初始化

#### Scenario: 部分配置缺失使用默认值

- **WHEN** `[memory]` section 存在但仅包含部分参数
- **THEN** 缺失的参数 SHALL 使用默认值（等于当前硬编码常量）
- **AND** 系统 SHALL 正常初始化不报错

#### Scenario: 配置 section 不存在

- **WHEN** `llm.toml` 中不存在 `[memory]` section
- **THEN** 系统 SHALL 使用全部默认值初始化
- **AND** 默认值 SHALL 与当前硬编码常量完全一致
- **AND** 系统行为 SHALL 与变更前完全相同

### Requirement: 记忆配置校验

系统 SHALL 在配置加载后执行校验规则，校验失败 SHALL 返回错误并阻止初始化。

#### Scenario: 子预算之和超过总预算

- **WHEN** `chronicle_budget + db_budget + strategy_budget > total_budget`
- **THEN** 系统 SHALL 返回校验错误 "子预算之和({sum})超过总预算({total})"
- **AND** 系统 SHALL 拒绝初始化

#### Scenario: 总预算超过 Prompt 上限

- **WHEN** `total_budget > prompt_max_tokens`
- **THEN** 系统 SHALL 返回校验错误 "记忆总预算({total})超过Prompt上限({prompt})"
- **AND** 系统 SHALL 拒绝初始化

#### Scenario: 重要性阈值超出范围

- **WHEN** `importance_threshold <= 0.0` 或 `importance_threshold > 1.0`
- **THEN** 系统 SHALL 返回校验错误 "重要性阈值必须在 (0.0, 1.0] 范围内"
- **AND** 系统 SHALL 拒绝初始化

#### Scenario: 预算值为零或负数

- **WHEN** 任意预算值（total_budget、chronicle_budget、db_budget、strategy_budget）<= 0
- **THEN** 系统 SHALL 返回校验错误 "预算值必须大于 0"
- **AND** 系统 SHALL 拒绝初始化

#### Scenario: 容量值为零或负数

- **WHEN** `search_limit <= 0` 或 `short_term_capacity <= 0`
- **THEN** 系统 SHALL 返回校验错误 "容量值必须大于 0"
- **AND** 系统 SHALL 拒绝初始化

### Requirement: 字符数计量截断

系统 SHALL 使用字符数（`.chars().count()`）而非字节数（`.len()`）进行记忆内容截断。

#### Scenario: 中文内容正确按字符截断

- **WHEN** 记忆内容为中文文本 "你好世界你好世界..."（超过 1800 字符）
- **THEN** 系统 SHALL 截断至最多 1800 个汉字
- **AND** 截断后内容 SHALL 包含恰好 1800 个字符

#### Scenario: 混合中英文内容正确截断

- **WHEN** 记忆内容为中英文混合文本
- **THEN** 系统 SHALL 按字符总数截断（中文 1 字符、英文 1 字符等同计数）
- **AND** 截断边界 SHALL 不会截断多字节字符的中间字节

#### Scenario: 内容未超限不截断

- **WHEN** 记忆内容字符数小于配置上限
- **THEN** 系统 SHALL 保留完整内容不做截断

### Requirement: 动态预算分配

系统 SHALL 在子预算之和超过总预算时执行动态降级，优先级为：Chronicle > DB > Strategy。

#### Scenario: 子预算之和未超限

- **WHEN** `chronicle_budget + db_budget + strategy_budget <= total_budget`
- **THEN** 系统 SHALL 按配置的各子预算值独立分配

#### Scenario: 子预算之和超过总预算触发降级

- **WHEN** `chronicle_budget + db_budget + strategy_budget > total_budget`
- **THEN** 系统 SHALL 执行降级：
  - Strategy 降级至最多 200 字符
  - DB 降级至最多 300 字符
  - Chronicle 保持原始配置值不变（最高优先级）
- **AND** 降级后三项之和 SHALL <= total_budget

### Requirement: 废弃 [memory_compression] section

系统 SHALL 静默忽略 `llm.toml` 中的 `[memory_compression]` section，不报错、不使用其中的值。

#### Scenario: 存在旧的 [memory_compression] section

- **WHEN** `llm.toml` 中同时存在 `[memory_compression]` 和 `[memory]` section
- **THEN** 系统 SHALL 仅使用 `[memory]` 中的值
- **AND** 系统 SHALL 不报错、不警告
- **AND** `[memory_compression]` 中的 `max_tokens` 和 `temperature`  SHALL 不产生任何效果

#### Scenario: 仅存在旧的 [memory_compression] section

- **WHEN** `llm.toml` 中仅有 `[memory_compression]` 而无 `[memory]` section
- **THEN** 系统 SHALL 使用全部默认值初始化
- **AND** 系统行为 SHALL 与变更前完全相同

## MODIFIED Requirements

<!-- 无修改现有规格的需求 -->

## REMOVED Requirements

<!-- 无废弃功能，仅废弃旧的配置 section -->

## RENAMED Requirements

<!-- 无重命名需求 -->
