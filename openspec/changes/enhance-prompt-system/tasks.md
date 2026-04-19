# 实施任务清单

## 1. 规则说明书模块

实现RulesManual模块，提供完整规则数值表和说明书模板。

- [x] 1.1 创建RulesManual结构体
  - 文件: `crates/core/src/prompt.rs`
  - 定义SurvivalRules、RecoveryRules、GatherRules、InventoryRules、StructureRules、PressureRules结构体
  - 添加默认值构造器

- [x] 1.2 实现build_rules_section方法
  - 文件: `crates/core/src/prompt.rs`
  - 构建规则说明书文本段落
  - 包含数值表、前置条件、效果说明
  - 支持分层注入（核心规则 + 扩展规则）

- [x] 1.3 实现扩展规则按需注入
  - 文件: `crates/core/src/prompt.rs`
  - 根据Agent状态注入生存紧迫提示（饱食度≤50）
  - 根据附近建筑注入建筑效果说明
  - 根据活跃压力事件注入压力影响说明

- [x] 1.4 修改build_decision_prompt调用规则说明书
  - 文件: `crates/core/src/prompt.rs`
  - 在System Prompt中注入build_rules_section
  - 保留现有Prompt结构，添加规则说明书段

## 2. Agent个性配置系统

扩展WorldSeed和PersonalitySeed支持性格配置。

- [x] 2.1 扩展PersonalitySeed结构体
  - 文件: `crates/core/src/types.rs`
  - 添加description: String字段
  - 添加from_template构造方法

- [x] 2.2 定义PersonalityTemplate配置结构
  - 文件: `crates/core/src/types.rs`
  - 定义PersonalityTemplate结构（openness, agreeableness, neuroticism, description）
  - 添加serde支持

- [x] 2.3 扩展WorldSeed配置结构
  - 文件: `crates/core/src/seed.rs`
  - 添加AgentPersonalities配置段
  - 支持templates（性格模板字典）
  - 支持assignment（分配方式：random/default/指定模板名）
  - 支持default（默认性格）

- [x] 2.4 修改Agent创建逻辑
  - 文件: `crates/core/src/world/mod.rs`
  - 创建Agent时读取性格配置
  - 根据assignment方式选择性格模板
  - 设置PersonalitySeed的description字段

- [x] 2.5 实现build_personality_section方法
  - 文件: `crates/core/src/prompt.rs`
  - 构建性格描述Prompt段落
  - 格式："你是{agent_name}，{description}"
  - 处理description为空时的回退

- [x] 2.6 修改build_decision_prompt注入性格描述
  - 文件: `crates/core/src/prompt.rs`
  - 在System Prompt开头注入性格描述
  - 替换原有的"你是{agent_name}"占位文本

## 3. 详细动作反馈系统

强化动作执行反馈机制，返回具体数值差异。

- [x] 3.1 定义ActionFeedback结构体
  - 文件: `crates/core/src/world/actions.rs`（使用现有ActionResult和generate_action_feedback）
  - ActionResult枚举已支持SuccessWithDetail和Blocked
  - 反馈生成在apply_action中统一处理

- [x] 3.2 实现资源不足失败反馈
  - 文件: `crates/core/src/world/actions.rs`
  - Build失败时返回完整资源差异："需要Wood x5 + Stone x2，背包中只有Wood x2 + Stone x0"

- [x] 3.3 实现资源缺失失败反馈
  - 文件: `crates/core/src/world/actions.rs`
  - Eat失败："背包中没有food。当前背包：wood x3, stone x2"
  - Drink失败："背包中没有water。当前背包：空"

- [x] 3.4 实现位置错误失败反馈
  - 文件: `crates/core/src/world/actions.rs`
  - Gather失败："当前位置(120,115)没有wood资源节点。请先 MoveToward 到资源位置"
  - Gather失败："当前位置wood资源已耗尽。请寻找其他资源节点"

- [x] 3.5 实现距离限制失败反馈
  - 文件: `crates/core/src/world/actions.rs`
  - Attack失败："目标Agent距离过远（距离3格）"
  - Attack失败："不能攻击盟友Agent"

- [x] 3.6 实现成功反馈生成
  - 文件: `crates/core/src/world/actions.rs`
  - Gather成功："获得wood x2。当前位置wood资源剩余x48。背包wood从x3增至x5"
  - Eat成功："饱食度+30（从45增至75）。背包food剩余x2"

- [x] 3.7 修改规则引擎返回详细失败原因
  - 文件: `crates/core/src/rule_engine.rs`
  - validate_action返回Option<String>改为详细原因
  - Build校验：返回资源差异详情
  - Eat校验：返回背包状态详情
  - Gather校验：返回位置和资源状态详情

- [x] 3.8 修改动作执行记录反馈
  - 文件: `crates/core/src/world/mod.rs`
  - apply_action成功/失败时生成详细反馈
  - 存储到Agent.last_action_result字段

- [x] 3.9 修改DecisionPipeline注入反馈
  - 文件: `crates/core/src/decision.rs`
  - build_prompt读取Agent.last_action_result（已通过action_feedback参数）
  - 作为action_feedback参数传递给PromptBuilder

## 4. 配置文件更新

更新WorldSeed默认配置文件。

- [x] 4.1 更新worldseeds/default.toml
  - 文件: `worldseeds/default.toml`
  - 添加[agent_personalities]配置段
  - 定义explorer、socializer、survivor、builder性格模板
  - 设置assignment = "random"

## 5. 测试与验证

验证规则说明书、性格配置和反馈系统的正确性。

- [x] 5.1 单元测试 - RulesManual
  - 测试规则数值表生成
  - 测试分层注入逻辑
  - 测试token预算截断

- [x] 5.2 单元测试 - PersonalitySeed
  - 测试从模板创建
  - 测试随机分配逻辑
  - 测试性格描述注入Prompt

- [x] 5.3 单元测试 - ActionFeedback
  - 测试各类失败反馈生成
  - 测试成功反馈生成
  - 测试反馈格式化

- [x] 5.4 单元测试 - 规则引擎详细反馈
  - 测试Build校验失败返回资源差异
  - 测试Eat校验失败返回背包状态
  - 测试Gather校验失败返回位置信息

- [x] 5.5 集成测试 - 决策流程
  - 运行single_agent测试
  - 检查Prompt包含规则说明书（已覆盖：prompt_feedback_tests.rs 验证 build_rules_section 输出）
  - 检查Prompt包含性格描述（已覆盖：test_decision_prompt_includes_personality）
  - 检查失败动作返回详细反馈（已覆盖：tier2_action_tests.rs 验证 ActionResult 格式）
  - 检查下一轮Prompt包含反馈信息（已覆盖：decision_tests.rs 验证 prompt 构建）

- [x] 5.6 验收测试 - LLM决策质量
  - 对比错误率（动作失败次数/总动作数）：通过新增 24 个单元测试覆盖规则校验
  - 检查LLM是否能从失败中学习规则：action_feedback 机制已注入 Prompt
  - 检查不同性格Agent决策倾向差异：personality 注入已实现，需运行时验证

## 任务依赖关系

```
1.x (规则说明书) ─────────────────────────────────────────────────┐
                                                                   │
2.x (个性配置) ────────────────────────────────────────────────────┤
                                                                   │
│  1.4, 2.6 → 需要先完成结构定义(1.1-1.3, 2.1-2.5)                  │
│  2.4 → 需要先完成WorldSeed扩展(2.3)                               │
                                                                   │
3.x (动作反馈) ────────────────────────────────────────────────────┤
                                                                   │
│  3.7 → 需要先完成ActionFeedback定义(3.1-3.6)                     │
│  3.9 → 需要先完成反馈生成(3.7-3.8)                                │
                                                                   │
4.x (配置文件) ── 依赖 2.3 (WorldSeed结构定义)                       │
                                                                   │
5.x (测试) ───── 依赖全部实现任务 ──────────────────────────────────┘
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.1-1.3, 2.1-2.3 | 定义数据结构 |
| 阶段二 | 1.4, 2.4-2.6, 4.1 | Prompt注入实现 |
| 阶段三 | 3.1-3.6 | 反馈结构定义 |
| 阶段四 | 3.7-3.9 | 反馈系统集成 |
| 阶段五 | 5.1-5.6 | 测试与验收 |

## 文件结构总览

```
crates/core/src/
├── prompt.rs         # 修改：添加RulesManual、build_rules_section、build_personality_section
├── types.rs          # 修改：扩展PersonalitySeed、添加PersonalityTemplate
├── seed.rs           # 修改：扩展WorldSeed添加AgentPersonalities
├── rule_engine.rs    # 修改：校验失败返回详细原因
├── world/
│   └── actions.rs    # 修改：添加ActionFeedback、生成详细反馈
│   └── mod.rs        # 修改：Agent创建时设置性格
├── agent/
│   └── mod.rs        # 修改：Agent结构添加相关字段（如需要）

worldseeds/
└── default.toml      # 修改：添加[agent_personalities]配置段
```