//! Prompt 构建器

use agentora_ai::config::MemoryConfig;

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
    pub fn build_decision_prompt(
        &self,
        agent_name: &str,
        perception_summary: &str,
        memory_summary: &str,
        strategy_hint: Option<&str>,
        action_feedback: Option<&str>,
        stack_limit: u32,
    ) -> String {
        let warehouse_limit = stack_limit * 2;
        let parts = PromptParts {
            system: format!(
                "你是 {agent_name}，一个自主决策的 AI Agent，在一个共享世界中生存。\n\
                \n\
                世界规则：\n\
                - 饱食度和水分度会随时间自然下降，归零时 HP 会持续扣减\n\
                - 当饱食度或水分度偏低时，你需要主动进食或饮水\n\
                - 世界中有各种资源（木材、石材、铁矿、食物、水源），可以采集\n\
                - 你可以用资源建造建筑、与其他 Agent 交易或战斗\n\
                - 你应该根据当前状态和环境，自主决定做什么\n\
                \n\
                背包规则：\n\
                - 每种资源堆叠上限 {stack_limit}（仓库建筑附近可达 {warehouse_limit}）\n\
                - 背包中的食物(food)和水(water)可以直接用于 Eat/Drink 动作\n\
                \n\
                采集规则（Gather 动作）：\n\
                - 每次采集固定获得 2 个资源（如果资源节点存量不足则取实际剩余量）\n\
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
        let mut action_feedback_tokens = Self::estimate_tokens(&parts.action_feedback);

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
        // 尝试在句号/换行处截断
        if let Some(pos) = truncated.rfind(|c| c == '。' || c == '！' || c == '？' || c == '\n') {
            return truncated[..pos + 1].to_string();
        }
        truncated
    }

    /// 输出格式指令
    fn output_format_instructions(&self) -> String {
        let mut s = String::new();
        s.push_str("请做出一个决策。输出格式为 JSON:\n");
        s.push_str("{\n");
        s.push_str("  \"reasoning\": \"决策理由\",\n");
        s.push_str("  \"action_type\": \"MoveToward|Gather|Eat|Drink|TradeOffer|Talk|Attack|Build|AllyPropose|Explore|Wait\",\n");
        s.push_str("  \"target\": \"简短描述，如 \\\"Wood x134\\\" 或 Agent 名字\",\n");
        s.push_str("  \"params\": {\"direction\": \"north\"}  // MoveToward 时填写方向\n");
        s.push_str("}\n\n");
        s.push_str("坐标系规则（重要！）：\n");
        s.push_str("- X增大 = 向东，X减小 = 向西\n");
        s.push_str("- Y增大 = 向南（屏幕下方），Y减小 = 向北（屏幕上方）\n");
        s.push_str("- 例如：从 (121, 113) 到 (121, 117)，Y从113→117增大了，所以是向南，不是向北！\n");
        s.push_str("- 相邻格信息中的方向标注是准确的，请直接参考它（如\"南(121,117): Forest\"）\n\n");
        s.push_str("动作说明：\n");
        s.push_str("- MoveToward: 向相邻的一格移动。params 格式：{\"direction\": \"north\"}，支持 north/south/east/west 或 北/南/东/西\n");
        s.push_str("  target 字段：简短描述目标，如 \\\"Wood x134\\\" 或 \\\"Water x50\\\" 或 \\\"Forest\\\"\n");
        s.push_str("  示例：\"action_type\": \"MoveToward\", \"target\": \"Wood\", \"params\": {\"direction\": \"east\"}\n");
        s.push_str("- Gather: 采集当前位置的资源（需要资源就在脚下），params 格式：{\"resource\": \"wood\"}\n");
        s.push_str("- Eat: 消耗背包中的1个食物，恢复饱食度(+30)。需要 背包 中有 food\n");
        s.push_str("- Drink: 消耗背包中的1个水，恢复水分度(+25)。需要 背包 中有 water\n");
        s.push_str("- Talk: 与附近 Agent 对话，target 字段填 Agent 名字\n");
        s.push_str("- Explore: 探索周边，params 格式：{\"target_region\": 区域编号}\n");
        s.push_str("- Wait: 等待一回合，不执行任何动作\n");
        s.push_str("决策策略：\n");
        s.push_str("- 到达资源位置后，可以使用 Gather 采集它\n");
        s.push_str("- 采集流程：MoveToward 靠近资源 → 到达后用 Gather 采集\n");
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
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::with_defaults()
    }
}
