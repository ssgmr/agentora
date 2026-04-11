## 1. ChronicleStore 文件 I/O

- [x] 1.1 实现文件加载（load 方法）
- [x] 1.2 实现文件路径解析（`~/.agentora/agents/<agent_id>/`）
- [x] 1.3 实现临时文件写入
- [x] 1.4 实现 rename 原子覆盖
- [x] 1.5 实现崩溃恢复（删除残留.tmp 文件）
- [x] 1.6 实现截断逻辑（按§分隔符删除最旧 entry）

## 2. ChronicleDB 检索集成

- [x] 2.1 实现 FTS5 查询构建器（按 Spark 类型映射关键词）
- [x] 2.2 实现检索结果注入 Prompt（`<chronicle-context>` 围栏）
- [x] 2.3 实现检索结果截断（围绕匹配词，每片段≤200 chars）
- [x] 2.4 实现重要性过滤（importance > 0.5）

## 3. TokenBudget 实现

- [x] 3.1 定义 TokenBudget 结构体（各部分预算跟踪）
- [x] 3.2 实现预算计算方法
- [x] 3.3 实现优先级截断逻辑
- [x] 3.4 实现预算重置（每 tick）

## 4. MemorySystem 集成

- [x] 4.1 修改 MemorySystem 初始化 ChronicleDB 连接
- [x] 4.2 修改 MemorySystem 初始化 ChronicleStore
- [x] 4.3 实现 record() 写入 ChronicleDB
- [x] 4.4 实现 get_summary() 返回三层记忆摘要

## 5. PromptBuilder 集成

- [x] 5.1 修改 build_decision_prompt 调用 MemorySystem
- [x] 5.2 实现记忆围栏标签包裹（`<chronicle-context>`）
- [x] 5.3 实现系统注添加（"以下是 Agent 历史记忆摘要"）
- [x] 5.4 实现总 token 数检查（≤2500）

## 6. 测试与验证

- [x] 6.1 编写 ChronicleStore 单元测试（加载/写入/截断）
- [x] 6.2 编写 ChronicleDB 单元测试（插入/检索/衰减）
- [x] 6.3 编写 TokenBudget 单元测试（预算分配/截断）
- [x] 6.4 编写集成测试：记忆累积循环
- [x] 6.5 验证记忆注入 Prompt 后的决策质量
