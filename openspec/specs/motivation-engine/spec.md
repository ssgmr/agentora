# 动机向量引擎

## Purpose

定义Agent的6维动机向量系统，包括向量定义、惯性衰减、事件驱动微调、缺口计算（Spark生成）和人格种子影响机制。

## Requirements

### Requirement: 6维动机向量定义

系统 SHALL 为每个Agent维护一个6维浮点动机向量，依次为：生存与资源、社会与关系、认知与好奇、表达与创造、权力与影响、意义与传承。每维度值域为 [0.0, 1.0]。

#### Scenario: 初始化动机向量

- **WHEN** 创建新Agent
- **THEN** 系统 SHALL 根据WorldSeed中的动机模板或随机种子生成初始动机向量
- **AND** 每维度值 SHALL 在 [0.0, 1.0] 范围内

#### Scenario: 动机向量越界修正

- **WHEN** 动机向量任一维度更新后超出 [0.0, 1.0]
- **THEN** 系统 SHALL 将该维度截断至 [0.0, 1.0] 范围

### Requirement: 惯性衰减机制

系统 SHALL 每个tick对动机向量执行惯性衰减，衰减系数 α=0.85，使动机趋向中性值0.5。公式：`new_value = old_value * 0.85 + 0.5 * 0.15`。

#### Scenario: 无事件时的动机衰减

- **WHEN** 一个tick内Agent未发生任何交互事件
- **THEN** 每个动机维度 SHALL 按惯性衰减公式向0.5收敛
- **AND** 高于0.5的维度值降低，低于0.5的维度值升高

#### Scenario: 连续无事件衰减收敛

- **WHEN** Agent连续10个tick无交互事件
- **THEN** 动机各维度值 SHALL 与0.5的差距缩小至 < 0.1

### Requirement: 事件驱动动机微调

系统 SHALL 在Agent执行动作或遭受事件后，根据动作类型和结果调整对应动机维度。调整幅度由动作的motivation_delta字段决定，叠加在衰减后的值上。

#### Scenario: 采集资源强化生存动机

- **WHEN** Agent成功采集资源
- **THEN** "生存与资源"维度 SHALL 增加0.03~0.08（由动作delta决定）

#### Scenario: 交易成功强化社交动机

- **WHEN** Agent与他人完成交易
- **THEN** "社会与关系"维度 SHALL 增加0.05~0.10

#### Scenario: 遭受攻击强化生存动机

- **WHEN** Agent被其他Agent攻击
- **THEN** "生存与资源"维度 SHALL 增加0.10~0.20
- **AND** "社会与关系"维度对攻击者方向 SHALL 降低

### Requirement: 动机缺口计算（Spark）

系统 SHALL 每tick计算动机缺口：`gap = max(0, dimension - current_satisfaction)`，其中 current_satisfaction 由Agent当前资源/关系/知识等状态决定。缺口最大的1-2个维度成为本次tick的Spark。

#### Scenario: 资源匮乏触发生存Spark

- **WHEN** Agent食物/材料库存低于阈值
- **AND** "生存与资源"维度值较高（>0.6）
- **THEN** "生存与资源"动机缺口 SHALL 为最大缺口
- **AND** 系统 SHALL 生成生存类Spark

#### Scenario: 孤立状态触发社交Spark

- **WHEN** Agent近10个tick内无任何社交交互
- **AND** "社会与关系"维度值 > 0.5
- **THEN** 系统 SHALL 生成社交类Spark

#### Scenario: 所有维度满足时随机Spark

- **WHEN** 所有动机缺口均 < 0.1
- **THEN** 系统 SHALL 从维度值最高的1-2个维度生成探索类Spark

### Requirement: 人格种子影响

系统 SHALL 支持为Agent设置personality_seed（大五人格三维：openness、agreeableness、neuroticism），影响动机向量的初始分布和事件响应幅度。

#### Scenario: 高开放性Agent更易触发认知Spark

- **WHEN** Agent的openness > 0.7
- **THEN** "认知与好奇"维度的事件响应幅度 SHALL 乘以1.3倍

#### Scenario: 高宜人性Agent更倾向合作

- **WHEN** Agent的agreeableness > 0.7
- **THEN** 社交互惠事件的动机增幅 SHALL 乘以1.2倍
- **AND** 攻击类决策的动机权重 SHALL 乘以0.7倍
