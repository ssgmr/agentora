//! 世界规则数值配置
//!
//! 定义生存消耗、资源恢复、采集、库存、建筑、压力等规则数值。

use std::collections::HashMap;

/// 生存消耗规则
#[derive(Debug, Clone)]
pub struct SurvivalRules {
    /// 饱食度每tick下降
    pub satiety_decay_per_tick: u32,
    /// 水分度每tick下降
    pub hydration_decay_per_tick: u32,
    /// HP归零后死亡
    pub death_on_hp_zero: bool,
}

impl Default for SurvivalRules {
    fn default() -> Self {
        Self {
            satiety_decay_per_tick: 1,
            hydration_decay_per_tick: 1,
            death_on_hp_zero: true,
        }
    }
}

/// 资源恢复规则
#[derive(Debug, Clone)]
pub struct RecoveryRules {
    /// Eat恢复饱食度
    pub eat_satiety_gain: u32,
    /// Drink恢复水分度
    pub drink_hydration_gain: u32,
    /// Eat需要食物数量
    pub eat_requires_food: u32,
    /// Drink需要水数量
    pub drink_requires_water: u32,
}

impl Default for RecoveryRules {
    fn default() -> Self {
        Self {
            eat_satiety_gain: 30,
            drink_hydration_gain: 25,
            eat_requires_food: 1,
            drink_requires_water: 1,
        }
    }
}

/// 采集规则
#[derive(Debug, Clone)]
pub struct GatherRules {
    /// 每次采集获得数量
    pub gather_amount: u32,
    /// 资源枯竭阈值
    pub depleted_threshold: u32,
}

impl Default for GatherRules {
    fn default() -> Self {
        Self {
            gather_amount: 2,
            depleted_threshold: 0,
        }
    }
}

/// 库存规则
#[derive(Debug, Clone)]
pub struct InventoryRules {
    /// 默认背包堆叠上限
    pub default_stack_limit: u32,
    /// 仓库附近堆叠上限
    pub warehouse_stack_limit: u32,
}

impl Default for InventoryRules {
    fn default() -> Self {
        Self {
            default_stack_limit: 20,
            warehouse_stack_limit: 40,
        }
    }
}

/// 建筑规则
#[derive(Debug, Clone)]
pub struct StructureRules {
    /// Camp建造消耗
    pub camp_cost: HashMap<String, u32>,
    /// Fence建造消耗
    pub fence_cost: HashMap<String, u32>,
    /// Warehouse建造消耗
    pub warehouse_cost: HashMap<String, u32>,
    /// Camp每tick恢复HP
    pub camp_heal_per_tick: u32,
    /// Camp覆盖范围（曼哈顿距离）
    pub camp_range: u32,
}

impl Default for StructureRules {
    fn default() -> Self {
        Self {
            camp_cost: HashMap::from([
                ("wood".to_string(), 5),
                ("stone".to_string(), 2),
            ]),
            fence_cost: HashMap::from([
                ("wood".to_string(), 2),
            ]),
            warehouse_cost: HashMap::from([
                ("wood".to_string(), 10),
                ("stone".to_string(), 5),
            ]),
            camp_heal_per_tick: 2,
            camp_range: 1,
        }
    }
}

/// 压力事件规则
#[derive(Debug, Clone)]
pub struct PressureRules {
    /// 干旱时水资源产出比例
    pub drought_water_reduction: f32,
    /// 丰饶时食物产出倍数
    pub abundance_food_multiplier: f32,
    /// 瘟疫HP损失
    pub plague_hp_loss: u32,
    /// 压力事件触发间隔范围
    pub trigger_interval: (u32, u32),
}

impl Default for PressureRules {
    fn default() -> Self {
        Self {
            drought_water_reduction: 0.5,
            abundance_food_multiplier: 2.0,
            plague_hp_loss: 20,
            trigger_interval: (40, 80),
        }
    }
}

/// 规则数值表
#[derive(Debug, Clone, Default)]
pub struct RulesManual {
    pub survival: SurvivalRules,
    pub recovery: RecoveryRules,
    pub gather: GatherRules,
    pub inventory: InventoryRules,
    pub structure: StructureRules,
    pub pressure: PressureRules,
}

impl RulesManual {
    /// 创建默认规则手册
    pub fn new() -> Self {
        Self::default()
    }

    /// 构建规则说明书文本段落（用于 Prompt 注入）
    pub fn build_rules_section(&self, agent_satiety: u32, agent_hydration: u32, nearby_structures: &[&str], active_pressures: &[&str]) -> String {
        let mut section = String::new();

        // 核心规则（始终注入）
        section.push_str("【世界规则数值表】\n");

        // 生存消耗（tick 间隔可配置，默认 5 秒）
        section.push_str("- 饱食度每tick下降1，水分度每tick下降1（默认5秒/tick）\n");
        section.push_str("- 饱食度或水分度归零时，HP每tick扣减1\n");

        // Eat/Drink规则
        section.push_str(&format!(
            "- Eat：消耗{}个food，饱食度+{}（不超过100）\n",
            self.recovery.eat_requires_food,
            self.recovery.eat_satiety_gain
        ));
        section.push_str(&format!(
            "- Drink：消耗{}个water，水分度+{}（不超过100）\n",
            self.recovery.drink_requires_water,
            self.recovery.drink_hydration_gain
        ));

        // Gather规则
        section.push_str(&format!(
            "- Gather：每次采集获得{}个资源（需要站在资源节点上）\n",
            self.gather.gather_amount
        ));

        // 库存规则
        section.push_str(&format!(
            "- 背包每种资源上限{}（Warehouse附近可达{}）\n",
            self.inventory.default_stack_limit,
            self.inventory.warehouse_stack_limit
        ));

        // 建筑消耗
        let camp_cost = self.structure.camp_cost.iter()
            .map(|(r, n)| format!("{}x{}", r, n))
            .collect::<Vec<_>>()
            .join("+");
        let fence_cost = self.structure.fence_cost.iter()
            .map(|(r, n)| format!("{}x{}", r, n))
            .collect::<Vec<_>>()
            .join("+");
        let warehouse_cost = self.structure.warehouse_cost.iter()
            .map(|(r, n)| format!("{}x{}", r, n))
            .collect::<Vec<_>>()
            .join("+");
        section.push_str(&format!(
            "- Build消耗：Camp={}，Fence={}，Warehouse={}\n",
            camp_cost, fence_cost, warehouse_cost
        ));

        // Camp效果
        section.push_str(&format!(
            "- Camp效果：范围内每tick恢复{}HP（曼哈顿距离≤{}）\n",
            self.structure.camp_heal_per_tick,
            self.structure.camp_range
        ));

        // 扩展规则按需注入
        // 生存紧迫提示
        if agent_satiety <= 50 {
            section.push_str("\n【生存紧迫提示】饱食度偏低，建议优先进食恢复\n");
        }
        if agent_hydration <= 50 {
            section.push_str("\n【生存紧迫提示】水分度偏低，建议优先饮水恢复\n");
        }

        // 建筑效果说明
        if nearby_structures.iter().any(|s| s.contains("Camp")) {
            section.push_str(&format!(
                "\n【建筑效果】当前位置附近有Camp，范围内每tick恢复{}HP\n",
                self.structure.camp_heal_per_tick
            ));
        }
        if nearby_structures.iter().any(|s| s.contains("Warehouse")) {
            section.push_str(&format!(
                "\n【建筑效果】当前位置附近有Warehouse，库存上限从{}提升到{}\n",
                self.inventory.default_stack_limit,
                self.inventory.warehouse_stack_limit
            ));
        }

        // 压力事件影响
        if !active_pressures.is_empty() {
            section.push_str("\n【压力事件】");
            for p in active_pressures {
                if p.contains("干旱") {
                    section.push_str(&format!("干旱：水资源产出{}% ", (self.pressure.drought_water_reduction * 100.0) as u32));
                } else if p.contains("丰饶") {
                    section.push_str(&format!("丰饶：食物产出翻倍{}x ", self.pressure.abundance_food_multiplier));
                } else if p.contains("瘟疫") {
                    section.push_str(&format!("瘟疫：随机Agent损失{}HP ", self.pressure.plague_hp_loss));
                } else {
                    section.push_str(&format!("{} ", p));
                }
            }
            section.push_str("\n");
        }

        section.push_str("\n");
        section
    }
}