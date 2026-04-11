## Context

当前 LLM Provider 实现状态：
- `OpenAiProvider` 有 HTTP 请求框架，但缺少重试逻辑
- `AnthropicProvider` 有 prefill trick，但缺少重试逻辑
- `parser.rs` 有多层 JSON 解析，已实现
- `FallbackChain` 有降级框架，但缺少配置加载
- `local.rs` 有框架但 mistralrs 集成为 TODO
- `rule_engine.rs` 有兜底框架但未集成到降级链

MVP 验证需求：LLM 调用稳定，API 失败时能降级到本地推理或规则引擎，保证决策不中断。

## Goals / Non-Goals

**Goals:**
- 实现配置加载（从 config/llm.toml）
- 实现 429 重试逻辑（最多 2 次）
- 实现本地 GGUF Provider（mistralrs 集成）
- 实现规则引擎兜底（集成到降级链）

**Non-Goals:**
- 多模型并行推理
- 复杂提示词优化工程
- 模型微调/LoRA 适配

## Decisions

### Decision 1: 配置文件格式

```toml
# config/llm.toml
[providers.openai]
enabled = true
api_base = "http://localhost:1234/v1"
api_key = "your-key"
model = "qwen3.5-2b"
timeout = 10

[providers.anthropic]
enabled = false
api_key = "your-key"
model = "claude-sonnet-4-6-20250929"

[providers.local]
enabled = true
model_path = "~/.agentora/models/qwen3.5-2b.gguf"
backend = "cpu"  # cpu / metal / cuda
```

**理由**: TOML 格式简洁，支持注释，Rust 有 serde 解析库

### Decision 2: 重试逻辑

- 429 响应时读取 `Retry-After` 头
- 等待指定秒数后重试
- 最多重试 2 次
- 仍失败则降级到下一个 Provider

**理由**: 符合 HTTP 标准，避免频繁请求激怒 API

### Decision 3: mistralrs 集成

- 使用 mistralrs crate 加载 GGUF 模型
- 支持 CPU/Metal 后端选择
- 内存不足时自动回退到 API

**理由**: mistralrs 纯 Rust 实现，活跃维护，支持结构化输出

### Decision 4: 降级链顺序

1. OpenAI 兼容 API（优先，速度快）
2. Anthropic API（备用）
3. 本地 GGUF（API 不可用时）
4. 规则引擎（最后兜底）

**理由**: 优先级按速度和可靠性排序

## Risks / Trade-offs

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| mistralrs 集成复杂 | 可能增加构建时间 | 作为可选依赖，feature flag 控制 |
| 本地推理速度慢 | 决策延迟可能>8 秒 | 使用小模型（2B），接受 MVP 性能 |
| API Key 泄露 | 配置文件包含敏感信息 | 支持环境变量覆盖 |

## Migration Plan

### 部署步骤

1. 创建 config/llm.toml 配置文件
2. 实现配置加载器
3. 实现 429 重试逻辑
4. 集成 mistralrs（可选 feature）
5. 修改 FallbackChain 调用规则引擎兜底
6. 运行单 Agent 测试验证降级链

### 回滚策略

- git tag 标记当前状态
- 若 LLM 调用失败，回退到纯规则引擎模式

## Open Questions

- [ ] mistralrs 版本选择（需要兼容 GGUF 格式）
- [ ] 本地模型的 context 长度限制
- [ ] API Key 存储方式（环境变量 vs 配置文件）
