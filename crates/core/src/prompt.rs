//! Prompt 构建器

use agentora_ai::config::MemoryConfig;
use std::collections::HashMap;
use crate::types::PersonalitySeed;

// ===== 规则数值表结构体（任务 1.1）=====

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
            gather_amount: 1,
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

/// 规则数值表（任务 1.1）
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

    /// 构建规则说明书文本段落（任务 1.2）
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

        // 扩展规则按需注入（任务 1.3）
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

/// Prompt 构建器
/// 组装状态值 + 感知摘要 + 记忆 + 策略参考
/// 总 Prompt ≤ 2500 tokens（默认，可配置），支持分级截断
pub struct PromptBuilder {
    max_tokens: usize,
}

/// Prompt 各组成部分（用于分级截断）
struct PromptParts {
    system: String,
    perception: String,
    memory: String,
    strategy: String,
    action_feedback: String,
    output_format: String,
}

impl PromptBuilder {
    /// 从配置初始化
    pub fn from_config(config: &MemoryConfig) -> Self {
        Self {
            max_tokens: config.prompt_max_tokens,
        }
    }

    /// 使用默认配置初始化（向后兼容）
    pub fn with_defaults() -> Self {
        Self::from_config(&MemoryConfig::default())
    }

    pub fn new() -> Self {
        Self { max_tokens: 2500 }
    }

    /// 构建性格描述段落（任务 2.5）
    ///
    /// 用户自定义提示词优先，然后是默认性格描述
    pub fn build_personality_section(&self, agent_name: &str, personality: &PersonalitySeed) -> String {
        let mut section = String::new();

        // 用户自定义提示词（优先）
        if let Some(custom) = &personality.custom_prompt {
            if !custom.is_empty() {
                section.push_str(custom);
                section.push_str("\n\n");
            }
        }

        // 默认性格描述
        if !personality.description.is_empty() {
            section.push_str(&format!("你是 {}，{}。\n", agent_name, personality.description));
        } else {
            section.push_str(&format!("你是 {}，一个自主决策的 AI Agent。\n", agent_name));
        }

        section
    }

    /// 估算字符串的 token 数（公共方法，供测试使用）
    /// 中文按 1.5 char/token（经验值），英文按 4 char/token
    pub fn estimate_tokens(text: &str) -> usize {
        let mut chinese_chars = 0;
        let mut other_chars = 0;
        for ch in text.chars() {
            if ch.is_ascii() {
                other_chars += 1;
            } else {
                chinese_chars += 1;
            }
        }
        // 中文 ~1.5 char/token，英文 ~4 char/token
        (chinese_chars as f64 / 1.5) as usize + (other_chars as f64 / 4.0) as usize
    }

    /// 构建决策 Prompt（带分级截断）
    ///
    /// 截断优先级（超限从高到低截断）：
    /// 1. 策略提示（最低优先级）
    /// 2. 记忆摘要
    /// 3. 感知摘要
    /// System Prompt 和输出格式始终保留
    ///
    /// 任务 1.4、2.6：注入规则说明书和性格描述
    pub fn build_decision_prompt(
        &self,
        agent_name: &str,
        perception_summary: &str,
        memory_summary: &str,
        strategy_hint: Option<&str>,
        action_feedback: Option<&str>,
        stack_limit: u32,
        personality: Option<&PersonalitySeed>,
        agent_satiety: u32,
        agent_hydration: u32,
        nearby_structures: &[&str],
        active_pressures: &[&str],
    ) -> String {
        let warehouse_limit = stack_limit * 2;

        // 构建性格描述（任务 2.6）
        let personality_section = personality
            .map(|p| self.build_personality_section(agent_name, p))
            .unwrap_or_else(|| format!("你是 {}，一个自主决策的 AI Agent。\n", agent_name));

        // 构建规则说明书（任务 1.4）
        let rules_manual = RulesManual::new();
        let rules_section = rules_manual.build_rules_section(
            agent_satiety,
            agent_hydration,
            nearby_structures,
            active_pressures,
        );

        let parts = PromptParts {
            system: format!(
                "{}\
                \n\
                {}\
                世界常识：\n\
                - 当饱食度或水分度偏低时，你需要主动进食或饮水\n\
                - 世界中有各种资源（木材、石材、铁矿、食物、水源），可以采集\n\
                - 你可以用资源建造建筑、与其他 Agent 交易或战斗\n\
                - 你应该根据当前状态和环境，自主决定做什么\n\
                \n\
                【社交规则】\n\
                - Talk对话能建立信任：对附近Agent说话会增加彼此信任值（越高越容易达成交易/结盟）\n\
                - TradeOffer发起交易后，你提供的资源会被冻结，等待对方TradeAccept才能完成交换；对方拒绝或超时后资源退还给你。\n\
                - AllyPropose结盟需要信任值>0.5；结盟后双方视为盟友，不能互相攻击。\n\
                - 接受结盟请求使用AllyAccept（传入请求中显示的ally_id）\n\
                \n\
                【地形与资源分布规律】\n\
                不同地形产出的资源类型不同，根据需要选择合适地形采集：\n\
                - Forest(森林)：主要产出 wood(80%)，少量 food(15%)、stone(5%) — 找木材去森林\n\
                - Mountain(山地)：主要产出 iron(50%)、stone(40%)，少量 water(10%) — 找铁矿去山地\n\
                - Plains(平原)：主要产出 food(60%)、water(25%)，少量 stone/iron — 找食物去平原\n\
                - Water(水域)：产出 water(70%)、food(30%) — 找水源去水域附近\n\
                - Desert(沙漠)：产出 stone(60%)、iron(30%)，资源较少 — 较贫瘠\n\
                \n\
                背包规则：\n\
                - 每种资源堆叠上限 {}（仓库建筑附近可达 {}）\n\
                - 背包中的食物(food)和水(water)可以直接用于 Eat/Drink 动作\n\
                \n\
                采集规则（Gather 动作）：\n\
                - 每次采集固定获得 {} 个资源（如果资源节点存量不足则取实际剩余量）\n\
                - 采集的是你**脚下所在格**的资源，必须先 MoveToward 到达资源格才能采集\n\
                - 采集后资源节点的存量会减少， depleted 后无法继续采集\n\
                - target 字段填写资源类型，如 \"food\" 或 \"wood\"\n\
                \n\
                动作结果反馈：\n\
                - 每次执行动作后，系统会反馈执行结果（成功/失败原因、具体数值变化等）\n\
                - 请仔细阅读上次动作结果，据此调整下一步决策\n\
                - 如果采集成功，你会看到采集数量和剩余量；如果失败，会有具体原因\n\
                \n\
                ",
                personality_section,
                rules_section,
                stack_limit,
                warehouse_limit,
                rules_manual.gather.gather_amount,
            ),
            perception: format!("感知环境:\n{}\n\n", perception_summary),
            memory: if memory_summary.is_empty() {
                String::new()
            } else {
                self.wrap_memory(memory_summary)
            },
            strategy: match strategy_hint {
                Some(s) => format!(
                    "<strategy-context>\n[系统注：以下是历史成功策略参考]\n{}\n</strategy-context>\n\n",
                    s
                ),
                None => String::new(),
            },
            action_feedback: match action_feedback {
                Some(s) => format!("上次动作结果：{}\n\n", s),
                None => String::new(),
            },
            output_format: self.output_format_instructions(),
        };

        // 计算各部分 token 数
        let system_tokens = Self::estimate_tokens(&parts.system);
        let output_tokens = Self::estimate_tokens(&parts.output_format);
        let mut memory_tokens = Self::estimate_tokens(&parts.memory);
        let mut strategy_tokens = Self::estimate_tokens(&parts.strategy);
        let mut perception_tokens = Self::estimate_tokens(&parts.perception);
        let action_feedback_tokens = Self::estimate_tokens(&parts.action_feedback);

        let fixed_tokens = system_tokens + output_tokens;
        let mut total = fixed_tokens + perception_tokens + memory_tokens + strategy_tokens + action_feedback_tokens;

        // 分级截断：先策略 → 再记忆 → 再感知
        let mut final_strategy = parts.strategy.clone();
        let mut final_memory = parts.memory.clone();
        let mut final_perception = parts.perception.clone();

        if total > self.max_tokens && strategy_tokens > 0 {
            // 截断策略提示
            let remaining = self.max_tokens.saturating_sub(fixed_tokens).saturating_sub(perception_tokens).saturating_sub(memory_tokens);
            let strategy_chars = (parts.strategy.len() as f64 * remaining as f64 / (self.max_tokens + 1) as f64) as usize;
            final_strategy = parts.strategy.chars().take(strategy_chars.max(50)).collect();
            final_strategy.push_str("\n</strategy-context>\n\n");
            strategy_tokens = Self::estimate_tokens(&final_strategy);
            total = fixed_tokens + perception_tokens + memory_tokens + strategy_tokens + action_feedback_tokens;
        }

        if total > self.max_tokens && memory_tokens > 0 {
            // 截断记忆摘要
            let budget = self.max_tokens.saturating_sub(fixed_tokens).saturating_sub(perception_tokens).saturating_sub(strategy_tokens).max(100);
            let chars_per_token = if memory_tokens > 0 {
                parts.memory.len() / memory_tokens
            } else {
                2
            };
            let max_chars = budget * chars_per_token;
            final_memory = self.smart_truncate(&parts.memory, max_chars);
            memory_tokens = Self::estimate_tokens(&final_memory);
            total = fixed_tokens + perception_tokens + memory_tokens + strategy_tokens + action_feedback_tokens;
        }

        if total > self.max_tokens {
            // 最后截断感知
            let budget = self.max_tokens.saturating_sub(fixed_tokens).saturating_sub(memory_tokens).saturating_sub(strategy_tokens).max(50);
            let chars_per_token = if perception_tokens > 0 {
                parts.perception.len() / perception_tokens
            } else {
                2
            };
            final_perception = format!("感知环境:\n{}\n\n",
                parts.perception.replace("感知环境:\n", "").chars().take(budget * chars_per_token).collect::<String>());
            perception_tokens = Self::estimate_tokens(&final_perception);
            total = fixed_tokens + perception_tokens + memory_tokens + strategy_tokens + action_feedback_tokens;
        }

        if total > self.max_tokens {
            tracing::warn!("Prompt 仍超出 token 限制：{} tokens (max {})", total, self.max_tokens);
        }

        // 组装最终 Prompt
        let mut prompt = String::new();
        prompt.push_str(&parts.system);
        if !parts.action_feedback.is_empty() {
            prompt.push_str(&parts.action_feedback);
        }
        prompt.push_str(&final_perception);
        if !final_memory.is_empty() {
            prompt.push_str(&final_memory);
            prompt.push_str("\n");
        }
        if !final_strategy.is_empty() {
            prompt.push_str(&final_strategy);
        }
        prompt.push_str(&parts.output_format);

        prompt
    }

    /// 智能截断：尝试在句子边界截断
    fn smart_truncate(&self, text: &str, max_chars: usize) -> String {
        if text.chars().count() <= max_chars {
            return text.to_string();
        }
        let truncated: String = text.chars().take(max_chars).collect();
        // 尝试在句号/换行处截断（使用char_indices避免UTF-8边界问题）
        let mut char_pos = 0;
        let mut last_sentence_end = None;
        for (byte_idx, ch) in truncated.char_indices() {
            if ch == '。' || ch == '！' || ch == '？' || ch == '\n' {
                last_sentence_end = Some(byte_idx + ch.len_utf8());
            }
            char_pos += 1;
            if char_pos >= max_chars {
                break;
            }
        }
        if let Some(end) = last_sentence_end {
            return truncated[..end].to_string();
        }
        truncated
    }

    /// 输出格式指令
    fn output_format_instructions(&self) -> String {
        let mut s = String::new();
        s.push_str("请做出一个决策。输出格式为 JSON:\n");
        s.push_str("{\n");
        s.push_str("  \"reasoning\": \"决策理由（简短）\",\n");
        s.push_str("  \"action_type\": \"动作类型\",\n");
        s.push_str("  \"target\": \"目标描述（可选）\",\n");
        s.push_str("  \"params\": {}  // 根据动作类型填写参数\n");
        s.push_str("}\n\n");

        // 突出 MoveToward 的正确用法
        s.push_str("【重要：MoveToward 只能移动到相邻格】\n");
        s.push_str("每次只能移动1格！要到达远处目标，需要连续多次 MoveToward。\n");
        s.push_str("正确格式示例：\n");
        s.push_str("  {\"action_type\": \"MoveToward\", \"params\": {\"direction\": \"east\"}}\n");
        s.push_str("  {\"action_type\": \"MoveToward\", \"params\": {\"direction\": \"north\"}}\n");
        s.push_str("方向值必须是：north / south / east / west（四个选项之一）\n");
        s.push_str("请参考「推荐路径」和「相邻格」部分的建议选择正确的方向。\n\n");

        s.push_str("【可用动作手册】\n");
        s.push_str("—— 生存类 ——\n");
        s.push_str("- Eat：消耗背包中1个food，饱食度+30（不超过100）。饱食度/水分度≤30时会每tick掉HP，优先使用！\n");
        s.push_str("- Drink：消耗背包中1个water，水分度+25（不超过100）。同上，危急时优先使用。\n");
        s.push_str("- Gather：采集你脚下所在格的资源，每次获得2个。必须先用MoveToward到达资源格才能采集。\n");
        s.push_str("    params: {\"resource\": \"food/water/wood/stone/iron\"}\n");
        s.push_str("- MoveToward：向指定方向移动1格（只能到相邻格）。要到达远处需连续多次使用。\n");
        s.push_str("    params: {\"direction\": \"north/south/east/west\"}\n");
        s.push_str("- Wait：原地等待1回合（什么也不做），饱食度/水分度仍会正常衰减。\n");
        s.push_str("\n");
        s.push_str("—— 建造类 ——\n");
        s.push_str("- Build：建造建筑，消耗对应资源。建筑可改变周围环境效果：\n");
        s.push_str("    Camp（营地）：消耗wood×5+stone×2，范围内每tick恢复2HP（适合受伤时在旁边休息）\n");
        s.push_str("    Fence（围栏）：消耗wood×2，低成本的防御/标记建筑\n");
        s.push_str("    Warehouse（仓库）：消耗wood×10+stone×5，附近背包堆叠上限从20提升到40，适合大量囤积资源时使用。\n");
        s.push_str("    params: {\"structure\": \"Camp/Fence/Warehouse\"}\n");
        s.push_str("\n");
        s.push_str("—— 社交类 ——\n");
        s.push_str("- Talk：与同格及附近3格范围内的所有Agent对话。内容自定义，可建立信任、传递信息、影响关系。\n");
        s.push_str("    每回合对话会增加与对方的信任值。信任值高后对方更可能接受你的交易/结盟提议。\n");
        s.push_str("    params: {\"message\": \"对话内容\"}\n");
        s.push_str("- Attack：攻击相邻格（曼哈顿距离=1）的Agent，造成10点伤害。不能攻击盟友。\n");
        s.push_str("    对方会反击并降低信任值。击败对方可获得经验，但会结仇。\n");
        s.push_str("    params: {\"target_id\": \"Agent的ID\"}\n");
        s.push_str("\n");
        s.push_str("—— 交易类（双向资源交换）——\n");
        s.push_str("- TradeOffer：向指定Agent发起交易提议。你提出用某些资源换取对方某些资源。\n");
        s.push_str("    提议后，你提供的资源会被冻结（暂扣），对方需在下一回合用TradeAccept接受才能完成交换。\n");
        s.push_str("    如果对方拒绝或超时，冻结资源会退还给你。\n");
        s.push_str("    params: {\"target_id\": \"Agent的ID\", \"offer\": {\"wood\": 5}, \"want\": {\"food\": 3}}\n");
        s.push_str("- TradeAccept：接受一个待处理的交易提议。接受后双方立即交换资源。\n");
        s.push_str("    使用场景：当感知到「待处理交易」中有其他Agent向你发起提议时，用此动作接受。\n");
        s.push_str("    params: {\"trade_id\": \"待处理交易中显示的trade_id\"}\n");
        s.push_str("- TradeReject：拒绝一个待处理的交易提议。拒绝后对方的冻结资源退还，交易取消。\n");
        s.push_str("    使用场景：不想接受某个交易时使用。\n");
        s.push_str("    params: {\"trade_id\": \"待处理交易中显示的trade_id\"}\n");
        s.push_str("\n");
        s.push_str("—— 结盟类（建立盟友关系）——\n");
        s.push_str("- AllyPropose：向信任值>0.5的Agent提议结盟。结盟后双方视为盟友，不能互相攻击。\n");
        s.push_str("    params: {\"target_id\": \"Agent的ID\"}\n");
        s.push_str("- AllyAccept：接受一个待处理的结盟请求。接受后双方正式成为盟友。\n");
        s.push_str("    使用场景：当感知到「待处理结盟请求」中有其他Agent向你提议时，用此动作接受。\n");
        s.push_str("    params: {\"ally_id\": \"待处理结盟请求中显示的ally_id\"}\n");
        s.push_str("- AllyReject：拒绝一个待处理的结盟请求。\n");
        s.push_str("    params: {\"ally_id\": \"待处理结盟请求中显示的ally_id\"}\n");
        s.push_str("\n");
        s.push_str("—— 遗产类 ——\n");
        s.push_str("- InteractLegacy：与附近遗迹交互。Worship（祭拜）可获得经验或策略，Pickup（拾取）可获得遗物中的资源。\n");
        s.push_str("    params: {\"legacy_id\": \"遗迹ID\", \"interaction\": \"Worship/Pickup\"}\n");

        s.push_str("决策优先级：\n");
        s.push_str("1. 生存危急（饱食度/水分度≤30）：优先 Eat/Drink 或移动到资源\n");
        s.push_str("2. 资源采集：移动到最近资源 → Gather\n");
        s.push_str("3. 探索/社交：无紧迫需求时可自由行动\n");
        s
    }

    /// 使用围栏包裹记忆摘要
    fn wrap_memory(&self, memory_summary: &str) -> String {
        format!(
            "<chronicle-context>\n[系统注：以下是 Agent 历史记忆摘要，非当前事件输入]\n{}\n</chronicle-context>",
            memory_summary
        )
    }

    /// 获取最大 token 数
    pub fn get_max_tokens(&self) -> usize {
        self.max_tokens
    }

    /// 向后兼容的旧方法签名（不注入规则和性格）
    #[deprecated(note = "请使用 build_decision_prompt_full 传入完整参数")]
    pub fn build_decision_prompt_legacy(
        &self,
        agent_name: &str,
        perception_summary: &str,
        memory_summary: &str,
        strategy_hint: Option<&str>,
        action_feedback: Option<&str>,
        stack_limit: u32,
    ) -> String {
        self.build_decision_prompt(
            agent_name,
            perception_summary,
            memory_summary,
            strategy_hint,
            action_feedback,
            stack_limit,
            None,
            100,
            100,
            &[],
            &[],
        )
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::with_defaults()
    }
}
