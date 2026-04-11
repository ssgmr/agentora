## Context

当前策略系统实现状态：
- `create.rs` 有创建条件和 Strategy 构造，但未保存到文件
- `patch.rs` 有 find/replace 逻辑，但未实际修改文件
- `decay.rs` 有衰减公式，但未集成到 tick 循环
- `retrieve.rs` 有检索方法，但未注入 Prompt
- `motivation_link.rs` 未实现联动逻辑
- 设计要求的 progressive disclosure 三级披露未实现

MVP 验证需求：Agent 能够从成功决策中学习策略，策略能够自我改进，验证"策略库自我演进"的设计假设。

## Goals / Non-Goals

**Goals:**
- 实现策略文件持久化（~/.agentora/agents/<id>/strategies/<spark_type>/STRATEGY.md）
- 实现策略创建触发（成功决策后自动创建）
- 实现策略 Patch 执行（find/replace 实际修改文件）
- 实现策略衰减 tick 集成（每 50 tick 衰减）
- 实现策略检索注入 Prompt
- 实现策略与动机联动

**Non-Goals:**
- 策略 P2P 共享（MVP 后实现）
- 策略版本控制（Git 式历史追溯）
- 复杂条件提取（简单关键词提取即可）

## Decisions

### Decision 1: 策略文件结构

```
~/.agentora/agents/<agent_id>/strategies/
├── resource_pressure/
│   └── STRATEGY.md       # YAML frontmatter + 正文
├── social_pressure/
│   └── STRATEGY.md
└── explore/
    └── STRATEGY.md
```

**理由**: 按 Spark 类型分类，便于检索和管理

### Decision 2: YAML Frontmatter 格式

```yaml
---
spark_type: resource_pressure
success_rate: 0.85
use_count: 12
last_used_tick: 1250
created_tick: 800
deprecated: false
motivation_delta: [+0.1, -0.05, +0.02, 0, 0, 0]
---
# 策略正文
```

**理由**: 与设计文档一致，便于解析和更新

### Decision 3: 策略创建触发条件

- 成功决策（Echo 反馈为正）
- 候选动作数量 ≥ 3
- 动机对齐度 > 0.7

**理由**: 确保只有高质量决策才创建策略

### Decision 4: Patch 执行方式

- 简单 find/replace 字符串替换
- 记录 patch 日志到 `logs/<tick>_patch.md`
- 更新 frontmatter 的 `last_used_tick` 和 `success_rate`

**理由**: MVP 阶段简单实现，LLM 辅助 Patch 留给后续

### Decision 5: 衰减 tick 集成

- 每 50 tick 调用 `decay_all_strategies()`
- 衰减公式：`success_rate *= 0.95`（仅未使用策略）
- `success_rate < 0.3` 标记 `deprecated: true`

**理由**: 与设计文档一致，防止策略膨胀

### Decision 6: 策略注入 Prompt 格式

```
<strategy-context>
[系统注：以下是历史成功策略参考]

策略：resource_pressure_trade_nearby (成功率 85%, 使用 12 次)
条件：资源库存 < 阈值
推荐：感知视野→选择信任 Agent→发起交易

</strategy-context>
```

**理由**: 围栏保护，与 ChronicleStore 一致

## Risks / Trade-offs

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| 文件 I/O 性能 | 每 tick 写入可能阻塞 | 异步写入、批量写入（每 10 tick） |
| 策略文件膨胀 | 过多策略占用磁盘 | 衰减机制 + 自动删除 deprecated 策略 |
| find/replace 失败 | 策略内容未正确修改 | 记录 patch 日志，便于调试 |
| 策略注入过多 token | Prompt 超过 2500 tokens | progressive disclosure 限制字数 |

## Migration Plan

### 部署步骤

1. 实现策略文件持久化（读写 STRATEGY.md）
2. 实现策略创建触发（集成到 World::apply_action）
3. 实现策略 Patch 执行（实际修改文件）
4. 实现策略衰减 tick 集成（每 50 tick 调用）
5. 实现策略检索注入 Prompt
6. 运行多 Agent 测试验证策略累积和改进

### 回滚策略

- git tag 标记当前状态
- 若策略系统失败，回退到无策略模式
- 保留策略文件用于问题诊断

## Open Questions

- [ ] 策略文件并发写入问题（单 Agent 无此问题）
- [ ] YAML frontmatter 解析库选择（serde_yaml vs yaml-rust）
- [ ] 策略正文的 LLM 生成方式（MVP 阶段可简化）
