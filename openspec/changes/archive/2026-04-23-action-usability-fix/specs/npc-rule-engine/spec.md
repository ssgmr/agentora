# 功能规格增量说明

## MODIFIED Requirements

### Requirement: NPC 规则引擎动作映射

原需求（部分）：
| 认知(Cognitive) | Explore |

修改后：
| 认知(Cognitive) | MoveToward（随机方向） |

#### Scenario: 认知压力触发动作

- **WHEN** NPC 规则引擎检测到认知压力
- **THEN** 系统 SHALL 选择 MoveToward 动作配合随机方向
- **AND** 系统 SHALL 不再使用 Explore 动作类型