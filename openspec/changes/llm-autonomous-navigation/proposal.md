# 需求说明书

## 背景概述

Agentora 的核心目标是让 LLM 驱动的 Agent 自主演化，规则引擎仅作为兜底。然而当前的动作系统存在一个致命缺陷：

**Agent 无法直接导航到目标位置，必须先计算方向。**

当前 `ActionType::Move { direction: Direction }` 只支持东南西北四个方向。LLM 在感知摘要中看到：
```
位置：(128, 130)
资源分布:
  (130, 125): Food x100
  (135, 128): Water x50
```

如果要移动到食物位置，LLM 必须自己计算：
- dx = 130 - 128 = 2（东边）
- dy = 125 - 130 = -5（北边）
- 然后选择方向...

这违背了项目核心哲学——让 LLM 像真正智能体一样决策，而不是做数学题。小模型（如 Qwen-2B）容易算错，浪费 token，限制了 Agent 的自主性。

## 变更目标

- **目标1**：提供高层导航动作 `MoveToward { target: Position }`，让 LLM 直接指定目标坐标
- **目标2**：增强感知摘要，在资源信息中显示相对方向和距离（"东北方向，距5格"）
- **目标3**：确保规则引擎从 LLM 候选动作中正确验证和执行导航动作
- **目标4**：保持向后兼容，原有的 `Move { direction }` 动作继续有效

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `move-toward-action` | 新增 `ActionType::MoveToward { target: Position }` 动作，让 LLM 直接指定目标坐标，底层处理路径计算 |
| `navigation-perception` | 增强感知摘要，为每个检测到的资源显示相对方向（东南西北）和曼哈顿距离 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `action-validation` | 规则引擎需要验证 `MoveToward` 动作：检查目标是否可达、是否在视野范围内、路径是否被阻挡 |
| `action-execution` | `handle_move_toward` 需要实现单步移动逻辑：计算方向 → 调用 `handle_move` → 返回结果 |
| `llm-response-parser` | LLM 响应解析器需要支持解析 `MoveToward` 动作的 `target` 参数 |

## 影响范围

- **代码模块**：
  - `crates/core/src/types.rs` — 新增 `ActionType::MoveToward` 枚举变体
  - `crates/core/src/decision.rs` — 感知摘要增强、动作解析支持
  - `crates/core/src/rule_engine.rs` — 新增 `MoveToward` 验证逻辑
  - `crates/core/src/world/actions.rs` — 新增 `handle_move_toward` 实现
  - `crates/core/src/vision.rs` — 可选：增强视野扫描返回方向/距离信息

- **API接口**：无外部 API 变更，仅内部动作系统扩展

- **依赖组件**：无新增依赖

- **关联系统**：与现有移动系统（`handle_move`）集成

## 验收标准

- [ ] LLM 可以输出 `MoveToward { target: (130, 125) }` 格式的动作
- [ ] 感知摘要显示资源的相对方向（如"东北方向"）和距离（如"距5格"）
- [ ] 规则引擎正确验证 `MoveToward` 动作（目标可达性检查）
- [ ] `handle_move_toward` 正确执行单步移动，并返回与 `Move` 相同的结果类型
- [ ] 原有 `Move { direction }` 动作继续正常工作
- [ ] 单元测试覆盖新增的验证和执行逻辑
- [ ] 集成测试验证 LLM Agent 在看到食物时能正确导航采集