# 架构重构

## 概述

完整的架构重构，建立清晰的职责分离：
- Bridge 成为薄 GDExtension 桥接层（约150行）
- Core 新增 `simulation/` 编排模块
- World 模块合理拆分为子模块
- Agent 方法统一（World handler 调用 Agent 方法）
- Godot 客户端移除硬编码值，使用后端数据

## 动机

### 当前问题

1. **Bridge职责越界（1481行）**
   - 包含模拟编排、Agent循环、Tick循环、NPC创建
   - 应只是 GDExtension + 类型转换

2. **Core缺少simulation编排层**
   - 模拟逻辑散落在 bridge
   - 缺少统一的模拟控制公开 API

3. **Agent方法被World绕过**
   - `handle_attack` 直接操作 `agent.health` 和 `agent.relations`
   - `handle_trade_accept` 直接操作 inventory
   - Agent封装被破坏

4. **World/mod.rs职责混杂（1609行）**
   - 包含地形生成、tick、压力、里程碑、反馈、snapshot
   - 应拆分为专注的子模块

5. **Godot硬编码数据**
   - `_map_size = 256` 在多处硬编码
   - Snapshot 已包含 `terrain_width/height` 但未使用
   - SimulationBridge 路径在7个文件中硬编码，5种不同写法

6. **Godot重复代码**
   - `guide_panel.gd` 与 `agent_detail_panel.gd` 功能重复
   - `agent_sprite.tscn` 定义但从未使用

### 目标

- 建立清晰架构：配置 → Core → Bridge → Godot
- 让 Agent 真正自主（Agent 方法控制 Agent 状态）
- 为未来维护和扩展打好基础
- 移除所有客户端硬编码配置

## 范围

### 包含

1. **Rust 后端**
   - 新建 `core/simulation/` 模块
   - 拆分 `world/mod.rs` 为 generator, tick, milestones, pressure, feedback, snapshot
   - 统一 World handler 中 Agent 方法调用
   - Bridge瘦身（约150行）

2. **Godot 客户端**
   - 删除 `guide_panel.gd`（重复）
   - 删除 `agent_sprite.tscn`（未使用）
   - 创建统一 BridgeAccessor
   - 使用 snapshot 数据而非硬编码值
   - 移除空节点

### 不包含

- Network crate 重构（正常运行）
- AI crate 重构（正常运行）
- Storage 实现（结构已合理）
- 新功能

## 成功标准

1. Bridge crate 总行数 < 200
2. 所有模拟编排在 `core/simulation/`
3. 所有 World handler 调用 Agent 方法（不绕过）
4. Godot 客户端无硬编码地图尺寸
5. Godot 脚本无重复代码
6. 所有测试通过
7. 客户端渲染正确

## 风险

| 风险 | 缓解措施 |
|------|----------|
| 破坏现有功能 | 每阶段后运行测试，验证客户端渲染 |
| Rust借用检查器问题 | 可能需要仔细重构 World handler |
| Godot信号连接变化 | Bridge瘦身后测试信号流 |

## 依赖

无（纯重构）

## 时间估算

- Phase 1（simulation层）：1-2次
- Phase 2（world拆分）：1次
- Phase 3（agent统一）：0.5次
- Phase 4（godot清理）：0.5次
- Phase 5（代码打磨）：0.5次

总计：约3-4次会话