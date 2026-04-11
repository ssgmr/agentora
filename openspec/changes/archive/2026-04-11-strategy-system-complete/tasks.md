## 1. 策略文件持久化

- [x] 1.1 实现策略目录结构管理（创建目录、检查存在）
- [x] 1.2 实现 YAML frontmatter 解析（serde_yaml）
- [x] 1.3 实现策略文件读取（load_strategy）
- [x] 1.4 实现策略文件写入（save_strategy）
- [x] 1.5 实现原子写入（临时文件 + rename）
- [x] 1.6 实现策略列表方法（列出所有有效策略）

## 2. 策略创建触发集成

- [x] 2.1 修改 World::apply_action 检查策略创建条件
- [x] 2.2 实现 should_create_strategy 逻辑（成功≥3 候选>0.7 对齐）
- [x] 2.3 实现 create_strategy 调用（提取 reasoning/motivation_delta）
- [x] 2.4 实现策略内容安全扫描
- [x] 2.5 实现 strategy 工具接口（create action）

## 3. 策略 Patch 执行

- [x] 3.1 实现 detect_problem 逻辑（从 Echo 反馈识别问题）
- [x] 3.2 实现 patch_strategy 执行 find/replace
- [x] 3.3 实现 frontmatter 更新（last_used_tick, success_rate）
- [x] 3.4 实现 patch 日志记录（logs/<tick>_patch.md）
- [x] 3.5 实现 strategy 工具接口（patch action）

## 4. 策略衰减 tick 集成

- [x] 4.1 修改 World::advance_tick 每 50 tick 调用衰减
- [x] 4.2 实现 decay_all_strategies 方法
- [x] 4.3 实现 check_deprecation（success_rate < 0.3 标记废弃）
- [x] 4.4 实现 should_auto_delete（deprecated 且 100 tick 未使用）
- [x] 4.5 实现自动删除废弃策略

## 5. 策略检索注入 Prompt

- [x] 5.1 实现 retrieve_strategy 按 Spark 类型检索
- [x] 5.2 实现 get_strategy_summary 提取摘要
- [x] 5.3 实现 wrap_strategy_for_prompt（`<strategy-context>` 围栏）
- [x] 5.4 修改 PromptBuilder 注入策略摘要
- [x] 5.5 实现 progressive disclosure（Tier 1/2/3）

## 6. 策略与动机联动

- [x] 6.1 实现 on_strategy_success 调整动机向量
- [x] 6.2 实现 on_strategy_failure 反向调整动机
- [x] 6.3 实现 success_rate 权重计算
- [x] 6.4 修改 World::apply_action 调用动机联动

## 7. 测试与验证

- [x] 7.1 编写策略持久化单元测试
- [x] 7.2 编写策略创建触发集成测试
- [x] 7.3 编写策略衰减单元测试
- [x] 7.4 编写动机联动单元测试
- [x] 7.5 运行多 Agent 测试验证策略累积和改进
