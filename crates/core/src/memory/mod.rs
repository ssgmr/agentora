//! 三层记忆架构：ChronicleStore + ChronicleDB + StrategyHub

pub mod chronicle_store;
pub mod chronicle_db;
pub mod fence;
pub mod token_budget;
pub mod short_term;

use self::chronicle_store::ChronicleStore;
use self::chronicle_db::ChronicleDB;
use self::token_budget::TokenBudget;
use self::short_term::ShortTermMemory;
use agentora_ai::config::MemoryConfig;

/// 记忆系统
/// 注意：db 字段包含 SQLite 连接，不能 Clone，使用 Option 包装
#[derive(Debug)]
pub struct MemorySystem {
    /// 短期记忆（最近 5 条）
    short_term: ShortTermMemory,
    /// ChronicleDB 索引
    chronicle_db: Option<ChronicleDB>,
    /// ChronicleStore 持久化存储
    chronicle_store: Option<ChronicleStore>,
    /// TokenBudget 总量控制
    token_budget: TokenBudget,
    /// Agent ID（用于路径解析）
    agent_id: String,
}

impl Clone for MemorySystem {
    fn clone(&self) -> Self {
        // ChronicleDB 和 ChronicleStore 不能 Clone，重新初始化
        Self {
            short_term: self.short_term.clone(),
            chronicle_db: self.clone_chronicle_db(),
            chronicle_store: self.clone_chronicle_store(),
            token_budget: TokenBudget::with_defaults(),
            agent_id: self.agent_id.clone(),
        }
    }
}

impl MemorySystem {
    /// 从配置初始化
    pub fn from_config(agent_id: &str, config: &MemoryConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // 校验配置
        config.validate()?;

        let agent_id = agent_id.to_string();
        Ok(Self {
            short_term: ShortTermMemory::from_config(config),
            chronicle_db: None,
            chronicle_store: None,
            token_budget: TokenBudget::from_config(config),
            agent_id,
        })
    }

    /// 使用默认配置初始化（向后兼容）
    pub fn with_defaults(agent_id: &str) -> Self {
        Self::from_config(agent_id, &MemoryConfig::default())
            .expect("Default MemoryConfig should always be valid")
    }

    pub fn new(agent_id: &str) -> Self {
        Self::with_defaults(agent_id)
    }

    /// 初始化 ChronicleDB 连接（使用配置）
    pub fn init_chronicle_db_with_config(&mut self, config: &MemoryConfig) -> Result<(), Box<dyn std::error::Error>> {
        let db_path = self.get_db_path();
        let db = ChronicleDB::from_config(&db_path, config)?;
        self.chronicle_db = Some(db);
        tracing::info!("ChronicleDB 初始化完成：{}", db_path);
        Ok(())
    }

    /// 初始化 ChronicleStore（使用配置）
    pub fn init_chronicle_store_with_config(&mut self, config: &MemoryConfig) -> Result<(), std::io::Error> {
        let mut store = ChronicleStore::from_config(&self.agent_id, config);
        store.load()?;
        self.chronicle_store = Some(store);
        tracing::info!("ChronicleStore 初始化完成 for agent {}", self.agent_id);
        Ok(())
    }

    /// 初始化 ChronicleDB 连接（向后兼容，使用默认配置）
    pub fn init_chronicle_db(&mut self) -> Result<(), rusqlite::Error> {
        let db_path = self.get_db_path();
        match ChronicleDB::with_defaults(&db_path) {
            Ok(db) => { self.chronicle_db = Some(db); Ok(()) }
            Err(e) => Err(e),
        }
    }

    /// 初始化 ChronicleStore（向后兼容，使用默认配置）
    pub fn init_chronicle_store(&mut self) -> Result<(), std::io::Error> {
        let mut store = ChronicleStore::with_defaults(&self.agent_id);
        store.load()?;
        self.chronicle_store = Some(store);
        tracing::info!("ChronicleStore 初始化完成 for agent {}", self.agent_id);
        Ok(())
    }

    /// 获取数据库路径
    fn get_db_path(&self) -> String {
        let home_dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        home_dir
            .join(".agentora")
            .join("agents")
            .join(&self.agent_id)
            .join("chronicle.db")
            .to_string_lossy()
            .to_string()
    }

    /// 记录事件
    pub fn record(&mut self, event: &MemoryEvent) {
        // 1. 写入短期记忆
        self.short_term.push(event.clone());

        // 2. 写入 ChronicleDB（若已初始化）
        if let Some(db) = &self.chronicle_db {
            let fragment = chronicle_db::MemoryFragment {
                id: 0,
                tick: event.tick,
                text_summary: event.content.clone(),
                emotion_tag: event.emotion_tags.join(","),
                event_type: event.event_type.clone(),
                importance: event.importance,
                created_at: chrono::Utc::now().timestamp(),
            };

            if let Err(e) = db.insert(&fragment) {
                tracing::error!("写入 ChronicleDB 失败：{}", e);
            }
        }

        // 3. 写入 ChronicleStore（若已初始化）
        if let Some(store) = &mut self.chronicle_store {
            store.add_entry(event.tick, &event.content);
            if let Err(e) = store.atomic_write() {
                tracing::error!("写入 ChronicleStore 失败：{}", e);
            }
        }

        tracing::debug!("记忆事件记录完成：tick={}, importance={}", event.tick, event.importance);
    }

    /// 获取三层记忆摘要（用于决策 Prompt）
    pub fn get_summary(&self, spark_type: crate::decision::SparkType) -> String {
        let mut summary = String::new();

        // 1. ChronicleStore 快照
        if let Some(store) = &self.chronicle_store {
            let snapshot = store.get_snapshot();
            if !snapshot.chronicle.is_empty() {
                summary.push_str("<chronicle-snapshot>\n");
                summary.push_str(&snapshot.chronicle);
                summary.push_str("\n</chronicle-snapshot>\n");
            }
            if !snapshot.world_seed.is_empty() {
                summary.push_str("<world-seed>\n");
                summary.push_str(&snapshot.world_seed);
                summary.push_str("\n</world-seed>\n");
            }
        }

        // 2. ChronicleDB 检索结果
        if let Some(db) = &self.chronicle_db {
            // 使用 token_budget 中的 db_budget 作为 max_chars
            let db_budget = self.token_budget.get_usage().db_budget;
            match db.search_for_prompt(spark_type, db_budget) {
                Ok(result) => {
                    if !result.is_empty() {
                        summary.push_str(&result);
                        summary.push_str("\n");
                    }
                }
                Err(e) => {
                    tracing::error!("ChronicleDB 检索失败：{}", e);
                }
            }
        }

        // 3. 短期记忆
        let short_term = self.short_term.summary();
        if !short_term.is_empty() {
            summary.push_str("<short-term-memory>\n");
            summary.push_str(&short_term);
            summary.push_str("\n</short-term-memory>");
        }

        summary
    }

    /// 获取短期记忆摘要（向后兼容）
    pub fn get_short_term_summary(&self) -> String {
        self.short_term.summary()
    }

    /// 获取最近 N 条短期记忆事件
    pub fn get_recent_memories(&self, n: usize) -> Vec<&MemoryEvent> {
        self.short_term.get_recent(n)
    }

    /// 重置预算（新 tick 开始）
    pub fn reset_for_tick(&mut self, tick: u32) {
        self.token_budget.reset_for_tick(tick);
    }

    /// 获取预算使用情况
    pub fn get_budget_usage(&self) -> token_budget::BudgetUsage {
        self.token_budget.get_usage()
    }

    /// 辅助方法：克隆 ChronicleDB（用于 Clone）
    fn clone_chronicle_db(&self) -> Option<ChronicleDB> {
        let db_path = self.get_db_path();
        ChronicleDB::new(&db_path).ok()
    }

    /// 辅助方法：克隆 ChronicleStore（用于 Clone）
    fn clone_chronicle_store(&self) -> Option<ChronicleStore> {
        let mut store = ChronicleStore::new(&self.agent_id);
        store.load().ok()?;
        Some(store)
    }
}

impl Default for MemorySystem {
    fn default() -> Self {
        Self::with_defaults("default")
    }
}

/// 记忆事件
#[derive(Debug, Clone)]
pub struct MemoryEvent {
    pub tick: u32,
    pub event_type: String,
    pub content: String,
    pub emotion_tags: Vec<String>,
    pub importance: f32,
}
