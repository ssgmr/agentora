# Capability: Agent 视野感知

## Purpose
Agent 通过圆形视野扫描感知周围环境，获取地形、资源、其他 Agent 的信息，并注入决策 Prompt。

## 需求

### 需求：圆形视野扫描
系统 SHALL 以 Agent 当前位置为中心、半径为 5 格进行圆形（含边界方形）扫描，覆盖所有方向（东/南/西/北及对角线）。扫描 SHALL 通过遍历位置（非遍历实体）实现，复杂度为 O(r²)。

### 需求：资源感知带数量
系统 SHALL 在视野扫描中记录每个资源节点的 Position、ResourceType 和当前剩余 amount。

### 需求：地形感知
系统 SHALL 在视野扫描中记录每个可见位置的地形类型（Plains/Forest/Mountain/Water/Desert）。

### 需求：其他 Agent 感知
系统 SHALL 在视野扫描中通过 `agent_positions` 反向索引检测半径 5 格内的所有其他 Agent，记录其 AgentId、名字、位置、动机摘要（6维数组）、以及对自己的关系类型和信任值。

### 需求：感知数据注入决策 Prompt
系统 SHALL 将感知信息格式化为人类可读文本并注入 LLM 决策 Prompt，内容包括：
- 自身位置坐标
- 附近 Agent 列表（名字、距离、关系状态如"朋友"/"敌人"/"陌生人"、动机最高维度）
- 资源分布（位置、类型、数量）
- 地形概览（各方向主要地形类型）
