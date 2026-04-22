# 需求说明书

## 背景概述

当前Agent决策Prompt存在三大核心缺陷，导致LLM决策质量低下：

**规则说明不完整**：System Prompt中的"世界规则"只描述了大概机制，缺少关键数值（饱食度下降速率、Eat/Drink恢复量、Gather产出）、前置条件（Build需要什么资源）、建筑效果（Camp回血、Warehouse扩容）、压力事件影响等。LLM不知道规则就无法做出合理决策。

**Agent个性缺失**：`PersonalitySeed`结构存在但永远使用默认值(0.5,0.5,0.5)，WorldSeed没有Agent性格配置字段，Prompt只说"你是{agent_name}"没有任何性格描述。所有Agent决策风格雷同，缺乏多样性。

**动作反馈不够详细**：虽有`validation_failure`机制，但反馈信息不够具体。LLM无法从失败中学习规则，如"Build失败：需要5个Wood，你只有2个"这样的明确反馈缺失。

## 变更目标

- 目标1：补全Prompt中的规则说明书，包含所有数值、前置条件、效果描述
- 目标2：创建Agent个性配置系统，支持性格设定注入Prompt影响决策倾向
- 目标3：强化动作反馈机制，失败时提供具体数值和条件信息供LLM学习

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `prompt-rules-manual` | 完整规则说明书模块，包含数值表、前置条件、效果描述，可分层注入Prompt |
| `agent-personality-config` | Agent个性配置系统，在WorldSeed中定义性格参数，Prompt中体现个性描述 |
| `detailed-action-feedback` | 详细的动作执行反馈，失败时返回具体数值差异和条件检查结果 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `decision-pipeline` | DecisionPipeline调用时读取个性配置，构建含性格描述的Prompt |
| `world-seed` | WorldSeed增加Agent性格配置字段（可选，默认使用随机性格） |

## 影响范围

- **代码模块**：
  - `crates/core/src/prompt.rs` — 规则说明书模板、个性描述注入
  - `crates/core/src/decision.rs` — 个性参数传递、反馈信息构建
  - `crates/core/src/types.rs` — PersonalitySeed扩展（可选性格描述文本）
  - `crates/core/src/seed.rs` — WorldSeed增加性格配置字段
  - `crates/core/src/world/actions.rs` — 动作执行失败时生成详细反馈
  - `crates/core/src/rule_engine.rs` — 校验失败时返回具体数值差异

- **API接口**：无新增接口，内部数据结构扩展

- **依赖组件**：无新增依赖

- **关联系统**：WorldSeed配置文件需更新格式

## 验收标准

- [ ] Prompt包含完整的规则数值表（饱食/水分下降率、恢复量、采集量、建造消耗等）
- [ ] Prompt包含所有动作的前置条件和效果描述
- [ ] Prompt包含建筑效果说明（Camp回血、Warehouse扩容、Fence阻挡）
- [ ] Prompt包含压力事件类型和影响说明
- [ ] WorldSeed支持Agent性格配置（性格描述文本或大五人格参数）
- [ ] Agent创建时使用配置的个性或随机生成个性
- [ ] Prompt中注入Agent性格描述影响决策倾向
- [ ] 动作执行失败时返回具体的数值差异（如"需要5 Wood，只有2")
- [ ] 动作执行失败时返回具体的条件检查结果（如"背包中无food")
- [ ] 运行测试验证LLM决策质量提升（错误率下降）