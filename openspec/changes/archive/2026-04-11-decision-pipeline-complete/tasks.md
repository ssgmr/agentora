## 1. 核心类型与数据结构

- [x] 1.1 定义 ActionCandidate 结构体（reasoning/action_type/target/params/motivation_delta/source）
- [x] 1.2 定义 CandidateSource 枚举（LLM / RuleEngine）
- [x] 1.3 定义 DecisonResult 结构体（selected_action/all_candidates/error_info）

## 2. 硬约束过滤器实现

- [x] 2.1 实现地形通行性检查（水域/山脉不可通行）
- [x] 2.2 实现边界检查（坐标不超出 256×256 地图）
- [x] 2.3 实现资源检查（建造/采集需要足够资源）
- [x] 2.4 实现目标存在性检查（交易/攻击/对话目标必须存在）
- [x] 2.5 实现距离检查（交互动作目标必须≤1 格）

## 3. 上下文构建器实现

- [x] 3.1 实现 Prompt 模板组装（动机+Spark+ 记忆 + 感知 + 策略）- 已在 prompt.rs 中实现
- [x] 3.2 实现 token 计数功能（估算各部分 token 数）
- [x] 3.3 实现截断逻辑（超限时先截断策略，再截断记忆）
- [x] 3.4 实现围栏标签包裹（`<chronicle-context>`, `<current-spark>`, `<strategy-context>`）- 已在 prompt.rs 中实现

## 4. LLM 调用集成

- [x] 4.1 实现 OpenAiProvider 完整 HTTP 请求（POST /v1/chat/completions）- 已在 openai.rs 中实现
- [x] 4.2 实现 AnthropicProvider 完整 HTTP 请求（POST /v1/messages）- 已在 anthropic.rs 中实现
- [x] 4.3 实现超时处理（10 秒无响应取消请求）- 已在 openai.rs/anthropic.rs 中实现
- [x] 4.4 实现 429 限流重试（Retry-After 后重试，最多 2 次）- 已在 retry.rs 中实现
- [x] 4.5 实现 Provider 降级链（OpenAI → Anthropic → 本地 GGUF）- 已在 fallback.rs 中实现

## 5. JSON 解析器实现

- [x] 5.1 实现 Layer 1：serde_json 直接解析
- [x] 5.2 实现 Layer 2：提取第一个{...}块
- [x] 5.3 实现 Layer 3：修复常见错误（尾逗号/单引号/注释）
- [x] 5.4 实现全部失败降级返回 ParseError

## 6. 规则校验器实现

- [x] 6.1 实现动作类型白名单校验（ActionType 枚举 discriminant）
- [x] 6.2 实现交易参数校验（offer/want 资源必须存在于背包）
- [x] 6.3 实现攻击目标校验（目标必须存在且≤1 格）
- [x] 6.4 实现建造参数校验（材料足够且位置可通行）

## 7. 动机加权选择器实现

- [x] 7.1 实现点积计算：score = dot_product(candidate.motivation_delta, agent.motivation)
- [x] 7.2 实现 softmax 选择函数（带 temperature 参数）
- [x] 7.3 实现唯一候选直接选择优化
- [x] 7.4 实现无候选通过校验时的规则引擎兜底

## 8. 决策管道串联

- [x] 8.1 实现 DecisionPipeline::execute() 主流程
- [x] 8.2 实现五阶段顺序调用：filter → prompt → llm → validate → select
- [x] 8.3 实现错误处理和降级逻辑（任一阶段失败降级到规则引擎）
- [x] 8.4 实现决策日志记录（各阶段结果、最终选择）

## 9. 集成与测试

- [x] 9.1 修改 World::apply_action 调用决策管道替代随机决策
- [x] 9.2 修改 bridge::simple_agent_decision 调用真实管道
- [x] 9.3 编写单元测试：硬约束过滤（各场景）
- [x] 9.4 编写单元测试：JSON 解析（各 Layer）
- [x] 9.5 编写单元测试：动机加权选择
- [x] 9.6 编写集成测试：单 Agent 完整决策循环
- [x] 9.7 运行多 Agent 测试验证涌现行为

## 10. 配置与文档

- [x] 10.1 创建 LLM 配置文件示例（config/llm.toml）
- [x] 10.2 更新决策管道 API 文档
- [x] 10.3 编写 Prompt 工程指南（如何调整模板）

---

## 完成状态总结

**已完成任务 (43/43):**
- 1.1-1.3: 核心数据结构 (ActionCandidate, CandidateSource, DecisionResult)
- 2.1-2.5: 硬约束过滤器
- 3.1, 3.4: Prompt 模板组装和围栏标签
- 4.1-4.5: LLM 调用集成 (OpenAiProvider, AnthropicProvider, 重试，降级链)
- 5.1-5.4: JSON 解析器
- 6.1-6.4: 规则校验器
- 7.1-7.4: 动机加权选择器
- 8.1-8.4: 决策管道串联
- 9.1-9.2: 集成到 World 和 bridge
- 10.1-10.3: 配置与文档

**待完成任务 (0/43):**
- 3.2-3.3: Token 计数和分级截断逻辑（已完成：中英文混合估算 + 策略→记忆→感知三级截断）
- 9.3: 硬约束过滤单元测试（已完成：8个测试用例）
- 9.4: JSON 解析单元测试（已完成：6个测试用例）
- 9.5: 动机加权选择单元测试（已完成：4个测试用例）
- 9.6: 单 Agent 集成测试（已完成：2个异步测试）
- 9.7: 多 Agent 涌现行为测试（已完成：4个测试用例）
