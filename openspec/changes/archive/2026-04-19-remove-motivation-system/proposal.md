# 需求说明书

## 背景概述

当前项目中六维动机系统（`MotivationVector`）与状态值系统（`health/satiety/hydration`）并存，两套机制共同影响 Agent 决策。但动机值本身难以实时量化——它既不是纯物理状态，也不是纯性格设定，导致运行时行为混乱：状态值通过硬编码阈值映射到动机，LLM 被要求感知动机值并输出 delta，Spark 系统从动机缺口生成决策触发器。这种设计让动机系统同时扮演了"角色设定"和"决策驱动"两个角色，但两者都没做好。

项目的核心理念是 **LLM 直接理解真实世界状态并自主决策**。Agent 看到 `satiety=30` 就知道该找食物，不需要先算成 `survival=0.8` 再告诉它。六维动机应作为设计理念的顶层指导（指导我们设计哪些状态、哪些玩法），而非运行时参与决策的变量。

## 变更目标

- **动机系统降级为设计理念**：六维动机不再作为运行时数据结构存在，仅作为设计文档中的理念框架，指导状态值和玩法机制的设计
- **状态值作为唯一真实来源**：health/satiety/hydration/inventory/relations 等物理状态直接参与决策，不再经过动机转换
- **LLM 直接读取状态决策**：Prompt 中展示完整的 Agent 状态信息，LLM 基于自身理解做出决策，不需要 Spark 提示"最需要什么"
- **规则引擎简化为纯校验**：不再基于动机维度生成兜底动作，只做动作合法性校验（边界/资源/地形/库存）
- **决策输出简化为动作+原因**：移除 `motivation_delta` 字段，LLM 只输出 `{action_type, params, reasoning}`

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `agent-profile` | 角色配置系统：将 Agent 的性格/MBTI/行为倾向编译进 System Prompt，支持用户自定义和运行时动态调整 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `decision-pipeline` | 移除 Spark 生成和动机加权选择阶段，LLM 直接输出动作，规则引擎仅校验 |
| `motivation-engine` | 从运行时移除，六维动机降级为设计理念，不再参与决策管道 |
| `strategy-system` | 移除 `motivation_delta` 字段和策略-动机联动，保留策略创建/衰减/检索功能 |
| `agent-state` | Agent 实体移除 `MotivationVector` 字段，保留并可能扩展状态值体系 |

## 影响范围

- **代码模块**：
  - `crates/core/src/motivation.rs` — 整个模块移除
  - `crates/core/src/decision.rs` — 移除 Spark/动机加权选择
  - `crates/core/src/rule_engine.rs` — 移除基于动机的兜底决策
  - `crates/core/src/prompt.rs` — 移除动机格式化
  - `crates/core/src/world/mod.rs` — 移除动机 tick/联动
  - `crates/core/src/agent/mod.rs` — 移除 motivation 字段
  - `crates/core/src/strategy/` — 移除 motivation_link.rs，修改 Strategy 结构
  - `crates/core/src/types.rs` — 移除 Action.motivation_delta
  - `crates/core/src/storage/` — 移除动机相关持久化
  - `crates/bridge/src/lib.rs` — 移除动机发送到 Godot
  - `client/scripts/` — 移除动机雷达图 UI
- **测试文件**：motivation_tests、decision_tests、strategy_tests、tier2_action_tests 等
- **关联系统**：Godot 客户端（移除动机面板）、OpenSpec 归档文件中的动机相关规范

## 验收标准

- [ ] `cargo build` 编译通过，无残留动机系统引用
- [ ] 单元测试（除已移除的 motivation_tests 外）全部通过
- [ ] Agent 能基于状态值做出合理决策（通过集成测试或运行验证）
- [ ] 规则引擎能正确校验非法动作（边界、资源不足、地形阻挡等）
- [ ] LLM 调用失败时，规则引擎提供合理的默认兜底行为
- [ ] Godot 客户端正常启动，无动机面板相关报错
- [ ] 策略系统（创建/检索/衰减）继续正常工作
- [ ] 六维动机不再出现在任何运行时代码或 Prompt 中
