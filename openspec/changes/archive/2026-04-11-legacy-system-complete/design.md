## Context

当前遗产系统实现状态：
- `Legacy::from_agent` 有框架，但 EchoLog 压缩为 TODO
- `EchoLog::from_agent` 使用硬编码而非 LLM 压缩
- 遗迹交互（祭拜/探索/拾取）未实现
- 遗产广播未集成到 GossipSub
- 物品衰减逻辑有但未被调用

MVP 验证需求：Agent 死亡产生遗产，其他 Agent 可发现并交互遗迹，验证"死亡沉淀为遗产，广播至 P2P 网络成为新 Spark"的闭环。

## Goals / Non-Goals

**Goals:**
- 实现 LLM 回响压缩（最后 3 条短期记忆压缩为摘要）
- 实现遗迹交互逻辑（祭拜/探索/拾取）
- 实现遗产 GossipSub 广播
- 实现遗迹衰减逻辑
- 实现遗产交互动机反馈

**Non-Goals:**
- 复杂遗产类型（遗迹/遗物/墓冢三种足够）
- 遗产争夺战（PVP 交互）
- 遗产正典化（DAO 投票）

## Decisions

### Decision 1: LLM 回响压缩

```rust
pub fn from_agent(agent: &Agent) -> Self {
    let last_3_memories = agent.memory.short_term.last(3);
    let prompt = format!("压缩以下记忆为回响摘要：{:?}", last_3_memories);
    let summary = llm.compress(prompt).await;
    Self {
        summary,
        emotion_tags: extract_emotions(last_3_memories),
        final_words: None,
        key_memories: extract_key_memories(last_3_memories),
    }
}
```

**理由**: 压缩为简短回响日志，注入 Prompt 成为他人 Spark 来源

### Decision 2: 遗迹交互类型

| 交互类型 | 动作 | 效果 |
|----------|------|------|
| 祭拜 | 原地等待 1 tick | 认知动机 +0.05，传承动机 +0.05 |
| 探索 | 读取回响日志 | 认知动机 +0.1，获得关键记忆 |
| 拾取 | 拿取物品 | 获得遗产物品，遗产物品减少 |

**理由**: 简单三种交互，涵盖情感/信息/物质层面

### Decision 3: 遗产广播

- 死亡事件序列化为 `LegacyEvent`
- 通过 GossipSub 广播到 "legacy" topic
- 其他节点接收到后添加到本地 `World.legacies`

**理由**: P2P 广播确保全网知晓遗产存在

### Decision 4: 衰减逻辑

- 50 tick 后开始衰减
- 每 tick 衰减 10%
- 物品数量<1 时移除
- 所有物品消失后遗迹可保留（成为纯纪念性遗迹）

**理由**: 物品衰减鼓励及时探索，纪念性遗迹保留历史感

## Risks / Trade-offs

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| LLM 压缩延迟 | 死亡处理时间增加 | 异步压缩，不阻塞主循环 |
| 遗迹过多性能问题 | 内存占用增加 | 定期清理无物品的古老遗迹 |
| 广播带宽消耗 | 死亡事件频繁时流量大 | 限制广播频率（每 tick 最多 1 次） |

## Migration Plan

### 部署步骤

1. 实现 LLM 回响压缩
2. 实现遗迹交互逻辑
3. 实现遗产 GossipSub 广播
4. 实现遗迹衰减逻辑
5. 实现遗产交互动机反馈
6. 运行多 Agent 测试验证遗产闭环

### 回滚策略

- git tag 标记当前状态
- 若遗产系统失败，回退到简单死亡移除模式

## Open Questions

- [ ] LLM 压缩的 Prompt 模板设计
- [ ] 遗迹在 Godot 中的视觉表现
- [ ] 遗产交互的冷却时间（防止刷动机）
