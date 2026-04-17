# 实施任务清单

## 1. 类型定义扩展

新增 MoveToward 动作类型，扩展 ActionType 枚举。

- [ ] 1.1 扩展 ActionType 枚举
  - 文件: `crates/core/src/types.rs`
  - 在 ActionType 枚举中添加 `MoveToward { target: Position }` 变体
  - 原有 Move 动作保持不变，确保向后兼容

- [ ] 1.2 更新 ActionType 的 Serialize/Deserialize 派生
  - 文件: `crates/core/src/types.rs`
  - 确保 MoveToward 支持 JSON 序列化和反序列化

- [ ] 1.3 更新 ActionType 的 Display 实现
  - 文件: `crates/core/src/types.rs`
  - 为 MoveToward 添加格式化输出

## 2. 方向计算工具函数

实现方向计算和导航辅助函数。

- [ ] 2.1 添加 calculate_direction 函数
  - 文件: `crates/core/src/vision.rs`
  - 实现从源位置到目标位置的方向计算
  - 返回 Option<Direction>，处理目标即当前位置的情况

- [ ] 2.2 添加 direction_description 函数
  - 文件: `crates/core/src/vision.rs`
  - 实现方向的中文描述输出（如"东北方向"）
  - 计算曼哈顿距离

- [ ] 2.3 添加单元测试
  - 文件: `tests/vision_tests.rs`（如不存在则创建）
  - 测试所有 8 个方向的计算
  - 测试边界情况（同位置、超远距离）

## 3. 感知摘要增强

修改 DecisionPipeline 的感知构建逻辑，添加资源方向和距离信息。

- [ ] 3.1 修改 build_perception_summary 函数
  - 文件: `crates/core/src/decision.rs`
  - 为每个资源添加 `[X方向，距N格]` 信息
  - 添加资源丰富度描述（大量/中等/少量）

- [ ] 3.2 实现资源优先级排序
  - 文件: `crates/core/src/decision.rs`
  - 饥饿时 Food 排第一，口渴时 Water 排第一
  - 相同类型按距离排序

- [ ] 3.3 添加感知摘要测试
  - 文件: `tests/decision_tests.rs`
  - 验证方向描述正确
  - 验证排序逻辑正确

## 4. LLM 响应解析扩展

扩展 parse_action_type 支持 MoveToward 动作解析。

- [ ] 4.1 添加 MoveToward 解析分支
  - 文件: `crates/core/src/decision.rs`
  - 支持 "MoveToward", "move_toward", "移动到", "前往" 等关键词

- [ ] 4.2 实现 parse_target_position 函数
  - 文件: `crates/core/src/decision.rs`
  - 支持多种坐标格式：
    - `{ x: 130, y: 125 }`
    - `[130, 125]`
    - `"130,125"` 或 `"(130, 125)"`

- [ ] 4.3 添加解析容错处理
  - 文件: `crates/core/src/decision.rs`
  - 解析失败时使用 Agent 当前位置作为默认值
  - 记录警告日志

## 5. 规则引擎验证

扩展规则引擎支持 MoveToward 动作的硬约束验证。

- [ ] 5.1 添加 MoveToward 验证函数
  - 文件: `crates/core/src/rule_engine.rs`
  - 实现 `is_valid_move_toward_target()` 函数
  - 验证目标在地图范围内
  - 验证目标在视野范围内（曼哈顿距离 ≤ 5）
  - 验证目标地形可通行

- [ ] 5.2 扩展 filter_hard_constraints
  - 文件: `crates/core/src/rule_engine.rs`
  - 在硬约束过滤中验证 MoveToward 目标
  - 为视野内的资源生成 MoveToward 候选动作
  - 限制最多 3 个 MoveToward 候选（避免过多）

- [ ] 5.3 更新 validate_action
  - 文件: `crates/core/src/rule_engine.rs`
  - 添加 MoveToward 动作的验证逻辑

- [ ] 5.4 添加规则引擎测试
  - 文件: `tests/rule_engine_tests.rs`
  - 测试目标验证的各种边界情况

## 6. 动作执行实现

实现 handle_move_toward 函数完成单步移动。

- [ ] 6.1 实现 handle_move_toward 函数
  - 文件: `crates/core/src/world/actions.rs`
  - 计算从当前位置到目标的方向
  - 调用现有 handle_move 执行单步移动
  - 处理目标即当前位置的情况

- [ ] 6.2 更新 apply_action 分发逻辑
  - 文件: `crates/core/src/world/mod.rs`
  - 在 match 分支中添加 MoveToward 处理

- [ ] 6.3 添加 ActionResult 类型（如需要）
  - 文件: `crates/core/src/types.rs`
  - 确保 ActionResult 枚举包含所有必要的结果类型

- [ ] 6.4 添加动作执行测试
  - 文件: `tests/action_tests.rs`
  - 测试单步移动到目标
  - 测试目标即当前位置
  - 测试路径被阻挡的情况

## 7. Bridge 序列化更新

更新 Godot Bridge 支持 MoveToward 动作的序列化。

- [ ] 7.1 更新 ActionType 序列化
  - 文件: `crates/bridge/src/lib.rs`
  - 确保 MoveToward 动作在 snapshot 中正确序列化

- [ ] 7.2 测试 Bridge 编译
  - 运行 `cargo build -p agentora-bridge --release`
  - 确保无编译错误

## 8. 集成测试与验证

端到端测试验证 LLM 导航能力。

- [ ] 8.1 单元测试 - 方向计算
  - 测试 calculate_direction 所有方向组合
  - 测试边界情况

- [ ] 8.2 单元测试 - 感知摘要
  - 测试资源方向描述正确
  - 测试排序逻辑正确

- [ ] 8.3 单元测试 - 动作解析
  - 测试各种 JSON 格式的 MoveToward 解析
  - 测试容错处理

- [ ] 8.4 单元测试 - 规则引擎
  - 测试目标验证逻辑
  - 测试候选动作生成

- [ ] 8.5 集成测试 - LLM 决策流程
  - 创建测试场景：资源在视野内
  - 验证 LLM 能够正确输出 MoveToward { target: Position }
  - 验证 Agent 能够移动到资源附近

- [ ] 8.6 验收测试 - 端到端导航
  - 运行 Godot 客户端
  - 观察 Agent 是否能正确导航到资源

## 任务依赖关系

```
1.1 → 1.2 → 1.3
    ↓
2.1 → 2.2 → 2.3
    ↓
3.1 → 3.2 → 3.3
    ↓
4.1 → 4.2 → 4.3
    ↓
5.1 → 5.2 → 5.3 → 5.4
    ↓
6.1 → 6.2 → 6.3 → 6.4
    ↓
7.1 → 7.2
    ↓
8.1 → 8.2 → 8.3 → 8.4 → 8.5 → 8.6
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.x, 2.x | 类型定义和工具函数，为后续任务提供基础 |
| 阶段二 | 3.x, 4.x | 感知增强和解析扩展，让 LLM 能接收信息和输出正确动作 |
| 阶段三 | 5.x, 6.x | 验证和执行逻辑，完成动作处理链路 |
| 阶段四 | 7.x | Bridge 序列化，确保 Rust 到 Godot 数据传递正确 |
| 阶段五 | 8.x | 测试验证，确保整体功能正确 |

## 文件结构总览

```
crates/core/src/
├── types.rs              # 修改: 新增 ActionType::MoveToward
├── vision.rs             # 修改: 新增方向计算函数
├── decision.rs           # 修改: 感知摘要增强 + 解析扩展
├── rule_engine.rs        # 修改: MoveToward 验证
└── world/
    ├── mod.rs            # 修改: apply_action 分发
    └── actions.rs        # 修改: handle_move_toward 实现

crates/bridge/src/
└── lib.rs                # 修改: 序列化支持

tests/
├── vision_tests.rs       # 新增: 方向计算测试
├── decision_tests.rs     # 修改: 感知摘要测试
├── rule_engine_tests.rs  # 修改: 验证逻辑测试
└── action_tests.rs       # 修改: 动作执行测试
```