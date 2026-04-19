# 功能规格说明 — motivation-engine (修改)

## MODIFIED Requirements

### Requirement: 生存压力驱动动机

当Agent的satiety ≤ 30或hydration ≤ 30时，生存动机维度(维度0) SHALL 临时提升0.3。当satiety = 0或hydration = 0时 SHALL 临时提升0.5。此加成在effective_motivation()计算中体现。

#### Scenario: 低饱食度驱动

- **WHEN** Agent satiety = 25, hydration = 80
- **THEN** effective_motivation()[0] 包含 +0.3 生存加成

#### Scenario: 零水分度强驱动

- **WHEN** Agent hydration = 0
- **THEN** effective_motivation()[0] 包含 +0.5 生存加成

#### Scenario: 两者叠加取最大

- **WHEN** Agent satiety = 0, hydration = 0
- **THEN** effective_motivation()[0] 加成取 max(food_boost, water_boost) = 0.5（不叠加）