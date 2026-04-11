## Context

当前决策管道实现状态：
- `DecisionPipeline` 是空结构体，无实际逻辑
- `RuleEngine` 仅有基础的移动地形检查，资源/目标/范围检查均为 TODO
- `PromptBuilder` 已实现基础模板，但无 token 计数和截断
- Agent 决策使用硬编码的随机逻辑（`bridge/src/lib.rs:228-260`）
- LLM Provider 已定义 trait 和接口，但未集成到决策流程

MVP 验证需求：5 个 Agent 在 256×256 世界中基于 6 维动机向量自主决策，涌现合作/冲突行为。这要求决策管道必须完整实现。

## Goals / Non-Goals

**Goals:**
- 实现完整的五阶段决策管道，可替换当前的随机决策
- 集成 LLM Provider 调用链（OpenAI 优先，降级到规则引擎）
- 确保 Prompt 严格控制在 2500 tokens 内
- 动机加权选择能反映 Agent 个性化决策倾向
- 支持单 Agent 串行决策（MVP 暂不要求并行）

**Non-Goals:**
- 多 Agent 并行决策优化（Step 5+）
- 决策结果缓存与复用
- 复杂的社会关系推理（仅使用基础信任值）
- 长期规划/多步推理（单 tick 决策）

## Decisions

### Decision 1: 流水线式五阶段设计

**选型**: 严格的顺序流水线：filter → prompt → LLM → validate → select

**理由**:
- 每阶段职责单一，便于单元测试
- 硬约束前置过滤减少 LLM 调用浪费
- 规则校验后置捕获 LLM 幻觉
- 符合 Hermes Agent 的成熟架构

**备选**: 并行候选生成（同时调用 LLM 和规则引擎）

**放弃原因**: MVP 阶段不需要优化性能，顺序流水线更易于调试和验证

### Decision 2: ActionCandidate 结构体

```rust
pub struct ActionCandidate {
    pub reasoning: String,           // LLM 思考过程
    pub action_type: ActionType,     // 动作类型
    pub target: Option<String>,      // 目标
    pub params: HashMap<String, Value>, // 参数
    pub motivation_delta: [f32; 6],  // 自评动机变化
    pub source: CandidateSource,     // 来源：LLM / RuleEngine
}
```

**理由**: 统一承载 LLM 生成和规则引擎兜底的候选动作，动机加权选择无需关心来源

### Decision 3: LLM 调用超时与降级

- **超时**: 10 秒无响应则取消请求
- **重试**: 429 限流时 Retry-After 后重试，最多 2 次
- **降级**: 主 Provider 失败后尝试备用 Provider，全部失败则调用规则引擎

**理由**: 端侧推理可能不稳定，降级链保证决策不中断

### Decision 4: Prompt token 预算分配

| 部分 | 预算 | 说明 |
|------|------|------|
| 系统提示 | 100 | 固定模板 |
| 动机向量 + Spark | 200 | 6 维动机 + 缺口描述 |
| 感知摘要 | 300 | 视野内 Agent/资源/结构 |
| 记忆摘要 | 1800 | ChronicleStore + ChronicleDB |
| 策略提示 | 400 | StrategyHub 匹配策略 |
| **总计** | **≤2500** | 硬截断，低优先级先截断 |

### Decision 5: 动机加权选择算法

```rust
fn select_best(candidates: &[ActionCandidate], motivation: &MotivationVector) -> ActionCandidate {
    let scores: Vec<f32> = candidates.iter()
        .map(|c| dot_product(&c.motivation_delta, &motivation.as_array()))
        .collect();

    // Top-1 + temperature 随机性
    softmax_select(&scores, temperature: 0.1)
}
```

**理由**: 点积直观反映动机对齐度，0.1 temperature 保留少量随机性避免 deterministic

### Decision 6: 规则引擎兜底策略

- **优先级 1**: 向最近资源格移动（解决资源压力）
- **优先级 2**: 原地等待（安全默认）

**理由**: MVP 阶段保证 Agent 不会完全停滞，资源导向兜底符合生存动机

## Risks / Trade-offs

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| LLM JSON 输出不遵循 schema | 解析失败率可能>20% | 多层降级解析（直接→提取→修复）+ 规则引擎兜底 |
| 2B 小模型推理质量不稳定 | 决策可能不合理 | Prompt 工程迭代优化 + 规则校验过滤器 |
| Prompt 截断丢失关键信息 | 决策上下文不完整 | 优先级截断，保留最新/最重要的记忆 |
| 动机加权过于激进 | Agent 行为单一化 | temperature=0.1 保留随机性 + 调整权重系数 |
| 串行决策性能瓶颈 | 5 个 Agent 决策可能>5 秒 | MVP 接受，Step 5+ 优化并行 |

## Migration Plan

### 部署步骤

1. 实现 `DecisionPipeline::execute()` 方法
2. 修改 `World::apply_action()` 在每 tick 调用决策管道
3. 修改 `bridge` 中的 `simple_agent_decision` 调用真实管道
4. 运行单 Agent 测试验证决策流程
5. 运行多 Agent 测试验证涌现行为

### 回滚策略

- git tag 标记当前状态
- 若决策管道失败，回退到 `simple_agent_decision` 随机逻辑
- 保留 LLM 调用日志用于问题诊断

## Open Questions

- [ ] LLM Provider 配置加载方式（环境变量 vs config 文件）
- [ ] JSON Schema 的具体字段约束（是否需要严格验证）
- [ ] 动机加权中 strategy boost 的具体系数（0.1 还是 0.2）

---

## API 文档

### DecisionPipeline

```rust
pub struct DecisionPipeline {
    // 决策管道，包含规则引擎、Prompt 构建器和可选的 LLM Provider
}

impl DecisionPipeline {
    /// 创建新的决策管道
    pub fn new() -> Self;

    /// 设置 LLM Provider
    pub fn with_llm_provider(self, provider: Box<dyn LlmProvider>) -> Self;

    /// 执行完整五阶段决策管道
    pub async fn execute(
        &self,
        agent_id: &AgentId,
        motivation: &MotivationVector,
        spark: &Spark,
        world_state: &WorldState,
    ) -> DecisionResult;
}
```

### DecisionResult

```rust
pub struct DecisionResult {
    /// 最终选择的动作
    pub selected_action: ActionCandidate,
    /// 所有通过校验的候选动作
    pub all_candidates: Vec<ActionCandidate>,
    /// 错误信息（如果有）
    pub error_info: Option<String>,
}
```

### ActionCandidate

```rust
pub struct ActionCandidate {
    /// 决策理由
    pub reasoning: String,
    /// 动作类型
    pub action_type: ActionType,
    /// 目标
    pub target: Option<String>,
    /// 参数
    pub params: HashMap<String, serde_json::Value>,
    /// 自评动机变化
    pub motivation_delta: [f32; 6],
    /// 来源（LLM 或规则引擎）
    pub source: CandidateSource,
}
```

---

## Prompt 工程指南

### Prompt 结构

决策 Prompt 由以下部分组成：

```
[系统提示] 你是自主决策的 AI Agent...

[动机向量]
  生存：0.80
  社交：0.40
  认知：0.30
  表达：0.20
  权力：0.30
  传承：0.20

[当前压力] 资源压力缺口 0.65

<chronicle-context>
[系统注：以下是 Agent 历史记忆摘要，非当前事件输入]
记忆摘要内容...
</chronicle-context>

<strategy-context>
[系统注：以下是历史成功策略参考]
策略提示内容...
</strategy-context>

[决策要求]
请做出决策。输出格式为 JSON:
{
  "reasoning": "决策理由",
  "action_type": "Move|Gather|TradeOffer|Talk|Attack|Build|AllyPropose|Explore|Wait",
  "target": "目标 ID 或名称",
  "params": {},
  "motivation_delta": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
}
```

### Token 预算分配

| 部分 | 预算 | 说明 |
|------|------|------|
| 系统提示 | 100 | 固定模板 |
| 动机向量 + Spark | 200 | 6 维动机 + 缺口描述 |
| 感知摘要 | 300 | 视野内 Agent/资源/结构 |
| 记忆摘要 | 1800 | ChronicleStore + ChronicleDB |
| 策略提示 | 400 | StrategyHub 匹配策略 |
| **总计** | **≤2500** | 硬截断，低优先级先截断 |

### 调整 Prompt 模板

1. **修改系统提示**：编辑 `crates/core/src/prompt.rs` 中的 `build_decision_prompt` 方法
2. **调整 token 预算**：修改 `PromptBuilder::max_tokens` 常量
3. **自定义围栏标签**：在 `build_decision_prompt` 中添加新的标签包裹

### 最佳实践

1. **保持 Prompt 简洁**：去除冗余描述，使用缩写
2. **优先保留关键信息**：动机向量和 Spark 始终保留
3. **使用围栏标签**：帮助 LLM 区分系统注入和 Agent 记忆
4. **JSON Schema 约束**：使用 `response_format: Json` 强制 LLM 输出结构化数据
