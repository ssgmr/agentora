//! 记忆总量控制
//!
//! 总记忆 ≤ 1800 chars（默认），按优先级分配空间

use agentora_ai::config::MemoryConfig;

const DOWNGRADE_STRATEGY_LIMIT: usize = 200;
const DOWNGRADE_DB_LIMIT: usize = 300;

/// 记忆预算分配器
#[derive(Debug)]
pub struct TokenBudget {
    total_limit: usize,
    chronicle_budget: usize,
    db_budget: usize,
    strategy_budget: usize,
    /// 当前 tick 已使用的预算
    used_chronicle: usize,
    used_db: usize,
    used_strategy: usize,
    /// 当前 tick 计数器
    current_tick: u32,
}

impl TokenBudget {
    /// 从配置初始化
    pub fn from_config(config: &MemoryConfig) -> Self {
        Self {
            total_limit: config.total_budget,
            chronicle_budget: config.chronicle_budget,
            db_budget: config.db_budget,
            strategy_budget: config.strategy_budget,
            used_chronicle: 0,
            used_db: 0,
            used_strategy: 0,
            current_tick: 0,
        }
    }

    /// 使用默认配置初始化（向后兼容）
    pub fn with_defaults() -> Self {
        Self::from_config(&MemoryConfig::default())
    }

    /// 向后兼容别名
    pub fn new() -> Self {
        Self::with_defaults()
    }

    /// 分配空间，返回各部分截断后的内容
    pub fn allocate(&self, chronicle: &str, db_results: &str, strategy: &str) -> BudgetAllocation {
        let chronicle_truncated = truncate_to_chars(chronicle, self.chronicle_budget);
        let db_truncated = truncate_to_chars(db_results, self.db_budget);
        let strategy_truncated = truncate_to_chars(strategy, self.strategy_budget);

        BudgetAllocation {
            chronicle: chronicle_truncated,
            db_results: db_truncated,
            strategy: strategy_truncated,
        }
    }

    /// 动态调整预算（超限时截断低优先级）
    pub fn dynamic_allocate(&self, chronicle: &str, db_results: &str, strategy: &str) -> BudgetAllocation {
        let total_len = chronicle.chars().count() + db_results.chars().count() + strategy.chars().count();

        if total_len <= self.total_limit {
            return self.allocate(chronicle, db_results, strategy);
        }

        // 超限：按优先级截断
        // 优先级：Chronicle(最高) > ChronicleDB > Strategy(最低)
        let mut db_budget = self.db_budget;
        let mut strategy_budget = self.strategy_budget;

        // 先截断 Strategy 到 metadata only (保留 200)
        if total_len > self.total_limit {
            strategy_budget = strategy_budget.min(DOWNGRADE_STRATEGY_LIMIT);
        }

        // 再截断 DB（保留 top 1）
        let remaining = self.total_limit - self.chronicle_budget;
        if chronicle.chars().count() + db_results.chars().count() + strategy_budget > remaining {
            db_budget = db_budget.min(DOWNGRADE_DB_LIMIT);
        }

        BudgetAllocation {
            chronicle: truncate_to_chars(chronicle, self.chronicle_budget),
            db_results: truncate_to_chars(db_results, db_budget),
            strategy: truncate_to_chars(strategy, strategy_budget),
        }
    }

    /// 重置预算计数器（每 tick 开始）
    pub fn reset_for_tick(&mut self, tick: u32) {
        self.current_tick = tick;
        self.used_chronicle = 0;
        self.used_db = 0;
        self.used_strategy = 0;
        tracing::debug!("TokenBudget 重置 for tick {}", tick);
    }

    /// 跟踪已使用的 Chronicle 预算
    pub fn track_chronicle_usage(&mut self, chars: usize) {
        self.used_chronicle += chars;
    }

    /// 跟踪已使用的 DB 预算
    pub fn track_db_usage(&mut self, chars: usize) {
        self.used_db += chars;
    }

    /// 跟踪已使用的 Strategy 预算
    pub fn track_strategy_usage(&mut self, chars: usize) {
        self.used_strategy += chars;
    }

    /// 获取当前预算使用情况
    pub fn get_usage(&self) -> BudgetUsage {
        BudgetUsage {
            chronicle_budget: self.chronicle_budget,
            chronicle_used: self.used_chronicle,
            db_budget: self.db_budget,
            db_used: self.used_db,
            strategy_budget: self.strategy_budget,
            strategy_used: self.used_strategy,
            total_limit: self.total_limit,
            current_tick: self.current_tick,
        }
    }

    /// 检查是否可以添加更多内容
    pub fn has_budget_remaining(&self, component: BudgetComponent) -> bool {
        match component {
            BudgetComponent::Chronicle => self.used_chronicle < self.chronicle_budget,
            BudgetComponent::Database => self.used_db < self.db_budget,
            BudgetComponent::Strategy => self.used_strategy < self.strategy_budget,
        }
    }
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// 预算组件类型
#[derive(Debug, Clone, Copy)]
pub enum BudgetComponent {
    Chronicle,
    Database,
    Strategy,
}

/// 预算使用情况
#[derive(Debug, Clone)]
pub struct BudgetUsage {
    pub chronicle_budget: usize,
    pub chronicle_used: usize,
    pub db_budget: usize,
    pub db_used: usize,
    pub strategy_budget: usize,
    pub strategy_used: usize,
    pub total_limit: usize,
    pub current_tick: u32,
}

/// 预算分配结果
#[derive(Debug, Clone)]
pub struct BudgetAllocation {
    pub chronicle: String,
    pub db_results: String,
    pub strategy: String,
}

/// 按字符数截断字符串
fn truncate_to_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_to_chars_english() {
        assert_eq!(truncate_to_chars("hello world", 5), "hello");
        assert_eq!(truncate_to_chars("hi", 10), "hi");
    }

    #[test]
    fn test_truncate_to_chars_chinese() {
        assert_eq!(truncate_to_chars("你好世界", 2), "你好");
        assert_eq!(truncate_to_chars("你好世界", 10), "你好世界");
    }

    #[test]
    fn test_truncate_to_chars_mixed() {
        assert_eq!(truncate_to_chars("hello你好world世界", 8), "hello你好w");
    }

    #[test]
    fn test_truncate_boundary_chars() {
        // 确保不会截断多字节字符
        assert_eq!(truncate_to_chars("你好", 1), "你");
    }

    #[test]
    fn test_token_budget_from_config() {
        let mut config = MemoryConfig::default();
        config.total_budget = 2000;
        config.chronicle_budget = 900;
        config.db_budget = 700;
        config.strategy_budget = 400;

        let budget = TokenBudget::from_config(&config);
        let usage = budget.get_usage();
        assert_eq!(usage.total_limit, 2000);
        assert_eq!(usage.chronicle_budget, 900);
        assert_eq!(usage.db_budget, 700);
        assert_eq!(usage.strategy_budget, 400);
    }

    #[test]
    fn test_token_budget_defaults() {
        let budget = TokenBudget::with_defaults();
        let usage = budget.get_usage();
        assert_eq!(usage.total_limit, 1800);
        assert_eq!(usage.chronicle_budget, 800);
        assert_eq!(usage.db_budget, 600);
        assert_eq!(usage.strategy_budget, 400);
    }
}
