# 实施任务清单

## 1. 生存消耗系统

Agent新增satiety/hydration字段，每tick自动衰减，耗尽后掉血，Wait动作改为饮食恢复。

- [x] 1.1 Agent新增satiety和hydration字段
  - 文件: `crates/core/src/agent/mod.rs`
  - 新增 `pub satiety: u32` (初始100) 和 `pub hydration: u32` (初始100)
  - 在 `Agent::new()` 中初始化

- [x] 1.2 生存消耗tick逻辑
  - 文件: `crates/core/src/world/mod.rs`
  - 在 `advance_tick()` 最前面新增 `survival_consumption_tick()`
  - 每 tick: satiety -= 2, hydration -= 2（取整）
  - satiety == 0 → HP -= 2, hydration == 0 → HP -= 3
  - 最低为0，不取负值
  - 依赖: 1.1

- [x] 1.3 Wait动作重定义
  - 文件: `crates/core/src/world/actions.rs`
  - `handle_wait()`: 尝试 consume 1 Food → satiety +30, consume 1 Water → hydration +25
  - 删除原有的 HP +5 逻辑
  - satiety/hydration 截断至上限100
  - 依赖: 1.1

- [x] 1.4 生存压力驱动动机
  - 文件: `crates/core/src/agent/mod.rs`
  - 修改 `effective_motivation()`: satiety ≤ 30 → 生存维度+0.3，satiety == 0 → +0.5；hydration同理
  - 取两者最大值（不叠加）

- [x] 1.5 Prompt注入生存状态
  - 文件: `crates/core/src/prompt.rs` 或 `crates/core/src/decision.rs`
  - 感知段新增当前satiety/hydration数值和状态描述（正常/饥饿/口渴）
  - 依赖: 1.1

- [x] 1.6 AgentSnapshot扩展
  - 文件: `crates/core/src/snapshot.rs`
  - AgentSnapshot新增satiety和hydration字段
  - 依赖: 1.1

## 2. 建筑效果系统

为三种建筑赋予实际功能效果。

- [x] 2.1 Camp回血效果
  - 文件: `crates/core/src/world/mod.rs`
  - `advance_tick()` 中（生存消耗之后、动机衰减之前）新增 `structure_effects_tick()`
  - 遍历Camp建筑，曼哈顿距离≤1的存活Agent HP+2（不超max_health）
  - 多Camp不叠加

- [x] 2.2 Fence阻挡敌对Agent
  - 文件: `crates/core/src/world/actions.rs`
  - `handle_move()` 新增Fence碰撞检查：目标格有Fence且Agent与所有者为Enemy → Blocked
  - 中立/盟友/所有者自身可通行
  - 依赖: Structure有owner_id字段（检查现有structure.rs是否已有）

- [x] 2.3 Warehouse扩展库存上限
  - 文件: `crates/core/src/world/actions.rs`, `crates/core/src/agent/inventory.rs`
  - 新增 `effective_inventory_limit(agent, world)` 函数
  - Agent在Warehouse曼哈顿距离≤1时返回40，否则20
  - `handle_gather()` 和 `Agent.gather()` 使用动态上限
  - 离开Warehouse范围后超出部分保留但不可再采集

- [x] 2.4 建筑效果Delta推送
  - 文件: `crates/bridge/src/lib.rs` 或 `crates/core/src/snapshot.rs`
  - 新增 `HealedByCamp { agent_id, hp_restored }` WorldDelta变体
  - 依赖: 2.1

## 3. 压力事件系统

激活pressure_tick，实际生成影响世界的压力事件。

- [x] 3.1 World新增压力相关字段
  - 文件: `crates/core/src/world/mod.rs`
  - 新增 `next_pressure_tick: u64` (初始随机40-80)
  - 新增 `pressure_multiplier: HashMap<String, f32>` 资源产出乘数

- [x] 3.2 压力事件生成逻辑
  - 文件: `crates/core/src/world/mod.rs`
  - `pressure_tick()` 升级：tick >= next_pressure_tick 且 pool < 3 时生成
  - 随机选择干旱/丰饶/瘟疫
  - 干旱: pressure_multiplier["Water"] = 0.5, 持续30 tick
  - 丰饶: Food节点current_amount翻倍(不超max), 持续20 tick
  - 瘟疫: 随机1-3个Agent HP-20, 持续1 tick
  - 依赖: 3.1

- [x] 3.3 压力事件效果生效
  - 文件: `crates/core/src/world/actions.rs`
  - `handle_gather()` 采集时检查 pressure_multiplier，产出 = base * multiplier
  - 干旱结束后移除乘数
  - 丰饶结束后不回退已增加量

- [x] 3.4 压力事件过期处理
  - 文件: `crates/core/src/world/mod.rs`
  - `pressure_tick()` 中推进remaining_ticks，移除过期事件
  - 过期时调用revert_pressure_effect恢复产出乘数
  - 依赖: 3.2

- [x] 3.5 压力事件Prompt注入
  - 文件: `crates/core/src/prompt.rs` 或 `crates/core/src/decision.rs`
  - 感知段新增活跃压力事件列表（类型+描述+剩余tick）
  - 依赖: 3.2

- [x] 3.6 压力事件叙事推送
  - 文件: `crates/core/src/world/mod.rs`, `crates/bridge/src/lib.rs`
  - 事件生成时推送 PressureStarted NarrativeEvent
  - 事件结束时推送 PressureEnded NarrativeEvent
  - Bridge新增对应Delta变体
  - 依赖: 3.2

## 4. NPC决策增强

RuleEngine新增生存优先逻辑。

- [x] 4.1 NPC生存需求决策
  - 文件: `crates/core/src/rule_engine.rs`
  - `fallback_decision()` 开头新增：satiety ≤ 30 或 hydration ≤ 30 时优先满足饮食
  - 有库存 → Wait；无库存 → 移动到最近资源 → Gather
  - 极端(satiety=0/hydration=0)时覆盖其他动机
  - 依赖: 1.1, 1.3

## 5. 文明里程碑系统

自动检测7个里程碑，推送到Godot。

- [x] 5.1 Milestone数据结构
  - 文件: `crates/core/src/world/mod.rs`（或新建 `milestone.rs`）
  - 定义 `MilestoneType` 枚举和 `Milestone` 结构体
  - World新增 `milestones: Vec<Milestone>` 字段
  - World新增 `total_trades: u32`, `total_attacks: u32`, `total_legacy_interacts: u32` 计数器

- [x] 5.2 里程碑计数器更新
  - 文件: `crates/core/src/world/actions.rs`
  - TradeAccept成功时 total_trades += 1
  - Attack成功时 total_attacks += 1
  - InteractLegacy成功时 total_legacy_interacts += 1
  - 依赖: 5.1

- [x] 5.3 里程碑检测逻辑
  - 文件: `crates/core/src/world/mod.rs`
  - `advance_tick()` 末尾新增 `check_milestones()`
  - 7个里程碑条件检测，每个只触发一次
  - 依赖: 5.1, 5.2

- [x] 5.4 里程碑快照扩展
  - 文件: `crates/core/src/snapshot.rs`
  - WorldSnapshot新增 `milestones: Vec<MilestoneSnapshot>` 字段
  - 依赖: 5.1

- [x] 5.5 里程碑Delta推送
  - 文件: `crates/bridge/src/lib.rs` 或 `crates/core/src/snapshot.rs`
  - 新增 `MilestoneReached { name, display_name, tick }` WorldDelta变体
  - 里程碑达成时推送Delta和NarrativeEvent
  - 依赖: 5.3

## 6. Godot客户端更新

UI重设计和状态展示增强。

- [x] 6.1 引导面板重设计
  - 文件: `client/scripts/guide_panel.gd`
  - 6个预设倾向按钮替代6个滑块（主界面）
  - 折叠式高级自定义滑块面板
  - 按钮点击调用Bridge的InjectPreference命令

- [ ] 6.2 Agent详情面板增强
  - 文件: `client/scripts/agent_manager.gd`（或新建 `agent_detail_panel.gd`）
  - 选中Agent时显示：HP条、饱食度条、水分度条（绿→黄→红色变）
  - 显示背包资源列表
  - 依赖: 1.6 (snapshot含satiety/hydration)

- [ ] 6.3 里程碑进度UI
  - 文件: `client/scripts/milestone_panel.gd`（新建）
  - 显示里程碑进度 (N/7) 和已达成列表
  - 达成时弹出2秒居中提示
  - 依赖: 5.5 (Delta推送)

- [ ] 6.4 压力事件叙事显示
  - 文件: `client/scripts/narrative_feed.gd`
  - PressureStarted用橙色显示，PressureEnded用灰色显示
  - 依赖: 3.6 (Delta推送)

- [ ] 6.5 建筑效果视觉反馈
  - 文件: `client/scripts/world_renderer.gd`
  - Camp周围显示回血范围指示（可选：淡色圆圈）
  - 依赖: 2.1

## 7. 测试与验证

- [ ] 7.1 生存消耗单元测试
  - 测试satiety/hydration衰减
  - 测试饥渴掉HP逻辑
  - 测试Wait饮食恢复
  - 文件: `tests/` 新增或扩展 `agent_tests`

- [ ] 7.2 建筑效果单元测试
  - 测试Camp回血
  - 测试Fence阻挡
  - 测试Warehouse库存上限
  - 文件: `tests/` 新增

- [ ] 7.3 压力事件单元测试
  - 测试事件生成逻辑
  - 测试干旱/丰饶/瘟疫效果
  - 测试事件过期恢复
  - 文件: `tests/` 新增

- [ ] 7.4 里程碑单元测试
  - 测试7个里程碑检测条件
  - 测试不可重复达成
  - 文件: `tests/` 新增

- [ ] 7.5 NPC生存决策测试
  - 测试饥饿NPC优先Wait/Gather
  - 测试极端情况覆盖其他动机
  - 文件: `tests/` 扩展

- [ ] 7.6 cargo test全量回归
  - 运行 `cargo test` 确保无回归

- [ ] 7.7 Godot客户端集成验证
  - 启动Godot客户端运行模拟
  - 验证生存状态条显示
  - 验证引导按钮功能
  - 验证里程碑提示
  - 验证压力事件叙事

## 任务依赖关系

```
Phase 1 (生存消耗):
  1.1 Agent字段 → 1.2 消耗tick → 1.3 Wait重定义
                     ↓                ↓
                  1.4 动机驱动     1.5 Prompt注入
                     ↓
                  1.6 Snapshot扩展

Phase 2 (建筑效果):
  2.1 Camp回血 ──→ 2.4 Delta推送
  2.2 Fence阻挡 (独立)
  2.3 Warehouse扩容 (独立)

Phase 3 (压力事件):
  3.1 World字段 → 3.2 事件生成 → 3.3 效果生效 → 3.4 过期处理
                                    ↓
                                 3.5 Prompt注入
                                 3.6 叙事推送

Phase 4 (NPC决策):
  4.1 NPC生存决策 (依赖1.1+1.3)

Phase 5 (里程碑):
  5.1 数据结构 → 5.2 计数器 → 5.3 检测逻辑 → 5.5 Delta推送
                              ↓
                           5.4 Snapshot扩展

Phase 6 (Godot客户端):
  6.1 引导面板 (独立)
  6.2 Agent详情 (依赖1.6)
  6.3 里程碑UI (依赖5.5)
  6.4 压力叙事 (依赖3.6)
  6.5 建筑视觉 (依赖2.1)

Phase 7 (测试):
  7.x 各系统单元测试 → 7.6 全量回归 → 7.7 Godot集成验证
```

## 建议实施顺序

| 阶段 | 任务 | 说明 |
| --- | --- | --- |
| 阶段一 | 1.1-1.6 | 生存消耗：核心循环基础，其他功能依赖此 |
| 阶段二 | 2.1-2.4 | 建筑效果：让建造有意义 |
| 阶段三 | 4.1 | NPC决策增强：确保NPC也能应对饥饿（短任务） |
| 阶段四 | 3.1-3.6 | 压力事件：让世界动起来 |
| 阶段五 | 5.1-5.5 | 里程碑：给玩家反馈 |
| 阶段六 | 6.1-6.5 | Godot客户端：全UI更新 |
| 阶段七 | 7.1-7.7 | 测试验证 |

## 文件结构总览

```
crates/core/src/
├── agent/mod.rs              [修改] 新增satiety/hydration字段
├── world/mod.rs              [修改] 新增消耗tick/建筑效果tick/里程碑/压力字段
├── world/actions.rs          [修改] Wait重定义/Fence碰撞/Warehouse上限/计数器
├── world/pressure.rs         [修改] 无改动（PressureEvent已有generate方法）
├── decision.rs               [修改] Prompt注入生存状态和压力事件
├── prompt.rs                 [修改] 可能调整感知段构建
├── rule_engine.rs            [修改] NPC生存优先决策
├── motivation.rs             [无需修改] effective_motivation在agent/mod.rs
├── snapshot.rs               [修改] 新增satiety/hydration/milestones字段
└── world/milestone.rs        [新增] MilestoneType和Milestone结构（可选，可内联）

crates/bridge/src/
└── lib.rs                    [修改] 新增Delta变体：MilestoneReached/PressureEvent/HealedByCamp/SurvivalStatus

client/scripts/
├── guide_panel.gd            [重写] 预设按钮+高级滑块
├── agent_manager.gd          [修改] 新增生存状态条和库存显示
├── narrative_feed.gd         [修改] 压力事件颜色区分
├── milestone_panel.gd        [新增] 里程碑进度和达成提示
└── world_renderer.gd         [修改] 可选：Camp回血范围指示

tests/
├── agent_tests.rs            [扩展] 生存消耗测试
├── structure_tests.rs        [新增] 建筑效果测试
├── pressure_tests.rs         [新增] 压力事件测试
└── milestone_tests.rs        [新增] 里程碑测试
```