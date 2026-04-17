# 需求说明书

## 背景概述

当前Agentora的模拟引擎有完善的动机-决策-行为管线，13种Action全部可执行，但缺少一个能让玩家持续投入的核心游戏循环。核心问题有三个：一是资源只进不出，Agent采集后无消耗需求，经济系统无法转动；二是建筑只有视觉效果没有功能，Build动作缺乏策略价值；三是世界太平静，没有环境压力事件来创造戏剧性。玩家的体验更接近"自动播放的电影"——调整滑块后只能被动观察，缺少"我的引导有意义"的因果反馈。

## 变更目标

- 建立Agent生存消耗循环（食物+水），让资源采集有持续意义，形成基础经济驱动
- 为三种建筑赋予实际游戏效果（Camp回血/Fence防御/Warehouse扩容），让建造产生策略选择
- 激活压力事件系统，让世界定期发生环境变化，迫使Agent改变行为，创造戏剧性
- 增强玩家引导界面，用直觉化倾向按钮替代抽象滑块，并显示Agent饱食度/水分度等状态
- 添加文明里程碑检测与展示，给玩家阶段性成就感

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `survival-consumption` | Agent饱食度(satiety)和水分度(hydration)系统，每tick自动衰减，耗尽后持续掉HP；Wait动作改为消耗背包食物/水恢复饱食度/水分 |
| `structure-effects` | 三种建筑的实际游戏效果：Camp(附近Agent每tick恢复HP)、Fence(阻挡敌对Agent通行)、Warehouse(附近Agent库存上限提升) |
| `pressure-activation` | 激活压力事件生成（干旱/丰饶/瘟疫），影响资源产出和Agent状态，事件注入决策Prompt和叙事推送 |
| `guide-enhancement` | 玩家引导面板增强：6个预设倾向按钮(生存/社交/探索/创造/征服/传承) + 自定义高级滑块；Agent详情面板显示饱食度/水分度条和库存 |
| `civilization-milestones` | 文明里程碑自动检测系统，7个里程碑从营地时代到黄金时代，达成时推送通知和叙事 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `world-model` | advance_tick新增生存消耗和建筑效果tick逻辑；pressure_tick从TODO改为实际生成事件 |
| `action-execution` | Wait动作从"回血+5"改为"专注吃喝"；建筑效果在结构体中新增功能字段 |
| `motivation-engine` | 饥饿/口渴状态影响生存动机权重；压力事件影响相关动机维度 |
| `decision-pipeline` | Prompt注入当前饱食度/水分度状态和压力事件描述 |
| `rule-decision` | NPC决策新增满足饮食需求的行为逻辑 |
| `godot-client` | 引导面板重设计；Agent详情面板新增状态条；里程碑UI；压力事件叙事 |

## 影响范围

- **代码模块**：
  - `crates/core/src/agent/mod.rs` — Agent新增satiety/hydration字段
  - `crates/core/src/world/mod.rs` — advance_tick新增消耗/建筑/压力逻辑
  - `crates/core/src/world/pressure.rs` — 激活事件生成、影响资源产出
  - `crates/core/src/world/structure.rs` — 建筑效果逻辑
  - `crates/core/src/world/actions.rs` — Wait动作重定义
  - `crates/core/src/decision.rs` — Prompt注入消耗/压力信息
  - `crates/core/src/rule_engine.rs` — NPC饮食决策
  - `crates/core/src/motivation.rs` — 消耗影响生存权重
  - `crates/core/src/snapshot.rs` — 新增satiety/hydration/milestones字段
  - `crates/bridge/src/lib.rs` — 新增delta变体推送
  - `client/scripts/` — guide_panel/agent_manager/narrative_feed增强
- **API接口**：Bridge Delta新增 MilestoneReached / PressureEvent / SurvivalStatus 变体
- **依赖组件**：无新外部依赖
- **关联系统**：与Tier-2世界交互、Tier-3涌现催化剂变更衔接

## 验收标准

- [ ] Agent每tick消耗satiety/hydration，耗尽后HP持续下降直至死亡
- [ ] Wait动作消耗背包食物/水恢复satiety/hydration（不再直接回血）
- [ ] Camp附近Agent每tick恢复2HP；Fence阻挡敌对Agent通行；Warehouse附近Agent库存上限+20
- [ ] 每40-80tick随机生成压力事件（干旱/丰饶/瘟鹏），影响资源产出或Agent状态
- [ ] 压力事件描述注入LLM决策Prompt
- [ ] Godot引导面板有6个直觉化倾向按钮 + 自定义滑块
- [ ] 选中Agent时显示饱食度/水分度条和库存列表
- [ ] 7个文明里程碑可自动检测，达成时Godot弹出提示 + 叙事推送
- [ ] cargo test全部通过，无回归