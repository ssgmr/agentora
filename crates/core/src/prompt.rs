//! Prompt 构建器

use crate::motivation::MotivationVector;
use crate::decision::Spark;
use agentora_ai::config::MemoryConfig;

/// Prompt 构建器
/// 组装动机向量+Spark+压缩记忆 + 视野 Agent+ 区域摘要
/// 总 Prompt ≤ 2500 tokens（默认，可配置），支持分级截断
pub struct PromptBuilder {
    max_tokens: usize,
}

/// Prompt 各组成部分（用于分级截断）
struct PromptParts {
    system: String,
    motivation: String,
    spark: String,
    perception: String,
    memory: String,
    strategy: String,
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
    /// 动机向量和 Spark 始终保留
    pub fn build_decision_prompt(
        &self,
        agent_name: &str,
        motivation: &MotivationVector,
        spark: &Spark,
        perception_summary: &str,
        memory_summary: &str,
        strategy_hint: Option<&str>,
    ) -> String {
        let parts = PromptParts {
            system: "你是一个自主决策的 AI Agent，在一个共享世界中生存。\n\n".to_string(),
            motivation: format!("当前动机状态:\n{}\n\n", self.format_motivation(motivation)),
            spark: format!("当前压力：{} (缺口 {:.2})\n\n", spark.description, spark.gap_value),
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
            output_format: self.output_format_instructions(),
        };

        // 计算各部分 token 数
        let system_tokens = Self::estimate_tokens(&parts.system);
        let motivation_tokens = Self::estimate_tokens(&parts.motivation);
        let spark_tokens = Self::estimate_tokens(&parts.spark);
        let output_tokens = Self::estimate_tokens(&parts.output_format);
        let mut memory_tokens = Self::estimate_tokens(&parts.memory);
        let mut strategy_tokens = Self::estimate_tokens(&parts.strategy);
        let mut perception_tokens = Self::estimate_tokens(&parts.perception);

        let fixed_tokens = system_tokens + motivation_tokens + spark_tokens + output_tokens;
        let mut total = fixed_tokens + perception_tokens + memory_tokens + strategy_tokens;

        // 分级截断：先策略 → 再记忆 → 再感知
        let mut final_strategy = parts.strategy.clone();
        let mut final_memory = parts.memory.clone();
        let mut final_perception = parts.perception.clone();

        if total > self.max_tokens && strategy_tokens > 0 {
            // 截断策略提示
            let strategy_chars = (parts.strategy.len() as f64 * (self.max_tokens - fixed_tokens - perception_tokens - memory_tokens).max(0) as f64 / (self.max_tokens + 1) as f64) as usize;
            final_strategy = parts.strategy.chars().take(strategy_chars.max(50)).collect();
            final_strategy.push_str("\n</strategy-context>\n\n");
            strategy_tokens = Self::estimate_tokens(&final_strategy);
            total = fixed_tokens + perception_tokens + memory_tokens + strategy_tokens;
        }

        if total > self.max_tokens && memory_tokens > 0 {
            // 截断记忆摘要
            let budget = (self.max_tokens - fixed_tokens - perception_tokens - strategy_tokens).max(100);
            let chars_per_token = if memory_tokens > 0 {
                parts.memory.len() / memory_tokens
            } else {
                2
            };
            let max_chars = budget * chars_per_token;
            final_memory = self.smart_truncate(&parts.memory, max_chars);
            memory_tokens = Self::estimate_tokens(&final_memory);
            total = fixed_tokens + perception_tokens + memory_tokens + strategy_tokens;
        }

        if total > self.max_tokens {
            // 最后截断感知
            let budget = (self.max_tokens - fixed_tokens - memory_tokens - strategy_tokens).max(50);
            let chars_per_token = if perception_tokens > 0 {
                parts.perception.len() / perception_tokens
            } else {
                2
            };
            final_perception = format!("感知环境:\n{}\n\n",
                parts.perception.replace("感知环境:\n", "").chars().take(budget * chars_per_token).collect::<String>());
            perception_tokens = Self::estimate_tokens(&final_perception);
            total = fixed_tokens + perception_tokens + memory_tokens + strategy_tokens;
        }

        if total > self.max_tokens {
            tracing::warn!("Prompt 仍超出 token 限制：{} tokens (max {})", total, self.max_tokens);
        }

        // 组装最终 Prompt
        let mut prompt = String::new();
        prompt.push_str(&parts.system);
        prompt.push_str(&parts.motivation);
        prompt.push_str(&parts.spark);
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
        s.push_str("  \"action_type\": \"Move|Gather|TradeOffer|Talk|Attack|Build|AllyPropose|Explore|Wait\",\n");
        s.push_str("  \"target\": \"目标 ID 或名称\",\n");
        s.push_str("  \"params\": {},\n");
        s.push_str("  \"motivation_delta\": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]\n");
        s.push_str("}\n");
        s
    }

    /// 使用围栏包裹记忆摘要
    fn wrap_memory(&self, memory_summary: &str) -> String {
        format!(
            "<chronicle-context>\n[系统注：以下是 Agent 历史记忆摘要，非当前事件输入]\n{}\n</chronicle-context>",
            memory_summary
        )
    }

    /// 格式化动机向量
    fn format_motivation(&self, motivation: &MotivationVector) -> String {
        use crate::motivation::DIMENSION_NAMES;
        let mut s = String::new();
        for (i, name) in DIMENSION_NAMES.iter().enumerate() {
            s.push_str(&format!("  {}: {:.2}\n", name, motivation[i]));
        }
        s
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
