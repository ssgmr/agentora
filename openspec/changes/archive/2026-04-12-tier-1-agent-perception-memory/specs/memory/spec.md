# Capability: Agent 短期记忆

## 需求

### 需求：行动后自动记录记忆
系统 SHALL 在每次 `apply_action` 成功后，调用 `agent.memory.record()` 记录一个 `MemoryEvent`，包含：
- tick：当前世界 tick
- event_type：动作类型字符串（如 "move", "gather", "attack"）
- content：动作描述文本（如 "向东移动至(130, 128)"）
- emotion_tags：情感标签（根据动作类型自动标注，如 Attack → ["冲突", "敌意"]）
- importance：重要度评分（Attack/Death=0.8, Gather/Talk=0.3, Move=0.1）

### 需求：记忆摘要注入决策 Prompt
系统 SHALL 在每次 LLM 决策前，调用 `agent.memory.get_summary(spark_type)` 获取三层记忆摘要（短期记忆 + ChronicleDB 检索 + ChronicleStore 快照），并将其注入决策 Prompt。

### 需求：记忆系统懒初始化
系统 SHALL 在 Agent 创建时（`World::generate_agents`）初始化 ChronicleDB 和 ChronicleStore 连接。初始化失败时（如磁盘不可写） SHALL 降级为仅使用短期记忆（内存队列），不阻断模拟运行。
