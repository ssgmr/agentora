## 1. 配置加载器

- [x] 1.1 定义 LlmConfig 结构体（serde 解析 TOML）
- [x] 1.2 实现配置加载器（从 config/llm.toml 读取）
- [x] 1.3 实现环境变量覆盖（LLM_API_KEY 等）
- [x] 1.4 实现默认配置（配置文件不存在时）

## 2. 重试逻辑

- [x] 2.1 实现 429 检测（响应状态码判断）
- [x] 2.2 实现 Retry-After 头解析
- [x] 2.3 实现重试等待（tokio::time::sleep）
- [x] 2.4 实现最多重试 2 次逻辑
- [x] 2.5 修改 OpenAiProvider 集成重试
- [x] 2.6 修改 AnthropicProvider 集成重试

## 3. 本地 GGUF Provider

- [x] 3.1 添加 mistralrs 依赖（可选 feature）
- [x] 3.2 实现 LocalProvider 结构体
- [x] 3.3 实现模型加载（mistralrs 初始化）
- [x] 3.4 实现内存检查（系统可用内存查询）
- [x] 3.5 实现推理调用（mistralrs generate）
- [x] 3.6 实现 OOM 降级到 API

## 4. 规则引擎兜底

- [x] 4.1 修改 FallbackChain 添加规则引擎为最后 Provider
- [x] 4.2 实现 RuleEngineProvider（包装规则引擎）
- [x] 4.3 实现兜底动作生成（资源压力→移动，无压力→等待）
- [x] 4.4 实现兜底日志记录

## 5. 集成与测试

- [x] 5.1 编写配置加载单元测试
- [x] 5.2 编写重试逻辑单元测试
- [x] 5.3 编写 JSON 解析集成测试
- [x] 5.4 编写降级链集成测试
- [x] 5.5 运行单 Agent 测试验证 LLM 调用
