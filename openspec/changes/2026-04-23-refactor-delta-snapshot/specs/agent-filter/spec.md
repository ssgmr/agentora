# Agent Filter Spec

## Purpose

定义叙事面板的 Agent 过滤功能，支持用户按 Agent ID 筛选叙事流，方便开发测试时追踪特定 Agent，以及用户游玩时关注某个远程 Agent。

## ADDED Requirements

### Requirement: Agent 选择器

叙事面板 SHALL 提供 Agent 选择器，允许用户选择查看特定 Agent 的叙事。

#### Scenario: 下拉选择 Agent

- **WHEN** 用户打开 Agent 选择器下拉
- **THEN** 下拉列表 SHALL 显示所有可见 Agent（本地 + 视野内远程）
- **AND** 每个 Agent 选项 SHALL 显示：名称 + 来源标记（本地/远程peer）
- **AND** 列表 SHALL 包含"全部Agent"选项（默认选中）

#### Scenario: 选择特定 Agent

- **WHEN** 用户从下拉选择"张三"
- **THEN** 叙事面板 SHALL 只显示 agent_id="张三" 的叙事
- **AND** 过滤 SHALL 与当前频道 Tab 组合生效
- **AND** 其他 Agent 的叙事 SHALL 隐藏

#### Scenario: 点击地图 Agent

- **WHEN** 用户点击地图上的 Agent Sprite
- **THEN** Agent 选择器 SHALL 自动切换到该 Agent
- **AND** 叙事面板 SHALL 过滤显示该 Agent 的叙事
- **AND** Agent 详情面板 SHALL 同时打开

### Requirement: 过滤组合逻辑

Agent 过滤 SHALL 与频道切换组合生效，实现多维度筛选。

#### Scenario: 频道 + Agent 组合过滤

- **WHEN** 用户选择 Tab="附近" 且 Agent="张三"
- **THEN** 叙事面板 SHALL 显示 channel=Nearby AND agent_id="张三" 的叙事
- **AND** 张三的本地频道叙事 SHALL 不显示
- **AND** 其他 Agent 的附近频道叙事 SHALL 不显示

#### Scenario: 世界频道忽略 Agent 过滤

- **WHEN** 用户选择 Tab="世界"
- **THEN** Agent 过滤 SHALL 不生效（世界事件不区分 Agent）
- **AND** 叙事面板 SHALL 显示所有世界频道叙事

#### Scenario: 重置过滤

- **WHEN** 用户选择"全部Agent"
- **THEN** Agent 过滤 SHALL 清除
- **AND** 叙事面板 SHALL 显示当前频道所有叙事

### Requirement: StateManager 过滤状态

StateManager SHALL 维护叙事过滤状态，支持 NarrativeFeed 查询。

#### Scenario: 存储过滤状态

- **WHEN** 用户切换频道或选择 Agent
- **THEN** StateManager SHALL 存储：
  - _narrative_channel: NarrativeChannel
  - _narrative_agent_filter: Option<String>
- **AND** 状态 SHALL 持久化在内存（不跨场景）

#### Scenario: 提供查询接口

- **WHEN** NarrativeFeed 调用 StateManager.get_filtered_narratives()
- **THEN** 返回结果 SHALL 根据 _narrative_channel 和 _narrative_agent_filter 过滤
- **AND** 过滤逻辑 SHALL 在 StateManager 内实现

### Requirement: Agent 来源显示

叙事面板 SHALL 显示叙事来源（本地/远程），帮助用户区分 Agent。

#### Scenario: 来源图标显示

- **WHEN** 叙事事件在面板显示
- **THEN** 本地 Agent 叙事 SHALL 显示 📋 图标
- **AND** 远程 Agent 叙事 SHALL 显示 📍 图标
- **AND** 世界事件 SHALL 显示 🌍 图标

#### Scenario: 来源 tooltip

- **WHEN** 用户鼠标悬停叙事来源图标
- **THEN** tooltip SHALL 显示：
  - 本地 Agent："本地 Agent"
  - 远程 Agent："远程 Agent (来自 peer_xxx)"