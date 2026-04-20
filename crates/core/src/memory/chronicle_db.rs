//! ChronicleDB 长期记忆索引
//!
//! SQLite + FTS5 全文索引，存储重要性>0.5 的事件片段

use rusqlite::Connection;
use crate::decision::SparkType;
use agentora_ai::config::MemoryConfig;

/// ChronicleDB 长期记忆数据库
#[derive(Debug)]
pub struct ChronicleDB {
    conn: Connection,
    importance_threshold: f32,
    search_limit: usize,
    snippet_max_chars: usize,
}

impl ChronicleDB {
    /// 从配置初始化
    pub fn from_config(path: &str, config: &MemoryConfig) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn,
            importance_threshold: config.importance_threshold,
            search_limit: config.search_limit,
            snippet_max_chars: config.snippet_max_chars,
        };
        db.init_schema()?;
        Ok(db)
    }

    /// 使用默认配置初始化（向后兼容）
    pub fn with_defaults(path: &str) -> Result<Self, rusqlite::Error> {
        Self::from_config(path, &MemoryConfig::default())
    }

    pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
        Self::with_defaults(path)
    }

    /// 初始化表结构
    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS memory_fragments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tick INTEGER NOT NULL,
                text_summary TEXT NOT NULL,
                emotion_tag TEXT NOT NULL,
                event_type TEXT NOT NULL,
                importance REAL NOT NULL DEFAULT 0.5,
                compression_level TEXT NOT NULL DEFAULT 'none',
                created_at INTEGER NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                text_summary,
                emotion_tag,
                event_type,
                content='memory_fragments'
            );

            CREATE TRIGGER IF NOT EXISTS memory_fts_insert AFTER INSERT ON memory_fragments BEGIN
                INSERT INTO memory_fts(rowid, text_summary, emotion_tag, event_type)
                VALUES (new.id, new.text_summary, new.emotion_tag, new.event_type);
            END;

            CREATE TRIGGER IF NOT EXISTS memory_fts_delete AFTER DELETE ON memory_fragments BEGIN
                INSERT INTO memory_fts(memory_fts, rowid, text_summary, emotion_tag, event_type)
                VALUES('delete', old.id, old.text_summary, old.emotion_tag, old.event_type);
            END;
        ")?;
        Ok(())
    }

    /// 写入记忆片段
    pub fn insert(&self, fragment: &MemoryFragment) -> Result<(), rusqlite::Error> {
        // 重要性过滤：只存储 importance > threshold 的记忆
        if fragment.importance <= self.importance_threshold {
            tracing::debug!("记忆片段重要性低于阈值，跳过：{}", fragment.importance);
            return Ok(());
        }

        self.conn.execute(
            "INSERT INTO memory_fragments (tick, text_summary, emotion_tag, event_type, importance, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                fragment.tick,
                fragment.text_summary,
                fragment.emotion_tag,
                fragment.event_type,
                fragment.importance,
                fragment.created_at,
            ],
        )?;
        Ok(())
    }

    /// FTS5 检索（带重要性过滤）
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryFragment>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tick, text_summary, emotion_tag, event_type, importance, created_at
             FROM memory_fragments
             WHERE id IN (SELECT rowid FROM memory_fts WHERE memory_fts MATCH ?1)
             AND importance > ?2
             ORDER BY importance DESC
             LIMIT ?3"
        )?;

        let fragments = stmt.query_map(rusqlite::params![query, self.importance_threshold, limit], |row| {
            Ok(MemoryFragment {
                id: row.get(0)?,
                tick: row.get(1)?,
                text_summary: row.get(2)?,
                emotion_tag: row.get(3)?,
                event_type: row.get(4)?,
                importance: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(fragments)
    }

    /// 根据 Spark 类型构建 FTS5 查询
    /// 注意：FTS5 的 NOT 是二元操作符，语法为 "A NOT B"（匹配 A 但排除 B）
    pub fn build_query_for_spark(&self, spark_type: SparkType) -> &'static str {
        match spark_type {
            SparkType::ResourcePressure => "resource AND (gather OR trade OR explore OR find)",
            SparkType::SocialPressure => "(alliance OR trade OR trust OR talk) NOT attack",
            SparkType::CognitivePressure => "learn OR discover OR understand OR explore",
            SparkType::ExpressivePressure => "create OR build OR express OR write",
            SparkType::PowerPressure => "lead OR command OR control OR influence",
            SparkType::LegacyPressure => "legacy OR teaching OR mentoring OR history",
            SparkType::Explore => "discover OR explore OR find OR new",
        }
    }

    /// 检索并格式化结果为 Prompt 片段（带围栏）
    pub fn search_for_prompt(&self, spark_type: SparkType, max_chars: usize) -> Result<String, rusqlite::Error> {
        let query = self.build_query_for_spark(spark_type);
        let fragments = self.search(query, self.search_limit)?;

        if fragments.is_empty() {
            return Ok("<chronicle-context>无相关历史记忆</chronicle-context>".to_string());
        }

        // 截断每个片段到最大字符数，围绕匹配词
        let snippets: Vec<String> = fragments
            .iter()
            .map(|f| {
                let snippet = truncate_around_match(&f.text_summary, self.snippet_max_chars);
                format!("[tick {}] {} (重要性：{:.2})", f.tick, snippet, f.importance)
            })
            .collect();

        let combined = snippets.join("\n");

        // 总字符数限制
        let truncated = if combined.chars().count() > max_chars {
            truncate_to_sentence_boundary(&combined, max_chars)
        } else {
            combined
        };

        Ok(format!("<chronicle-context>\n以下是 Agent 历史记忆摘要:\n{}\n</chronicle-context>", truncated))
    }

    /// 检索所有记忆片段（用于调试）
    pub fn get_all(&self) -> Result<Vec<MemoryFragment>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tick, text_summary, emotion_tag, event_type, importance, created_at
             FROM memory_fragments
             ORDER BY tick DESC"
        )?;

        let fragments = stmt.query_map(rusqlite::params![], |row| {
            Ok(MemoryFragment {
                id: row.get(0)?,
                tick: row.get(1)?,
                text_summary: row.get(2)?,
                emotion_tag: row.get(3)?,
                event_type: row.get(4)?,
                importance: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(fragments)
    }

    /// 记忆衰减：每 50tick 执行
    pub fn decay(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE memory_fragments SET importance = importance * 0.95",
            [],
        )?;
        self.conn.execute(
            "DELETE FROM memory_fragments WHERE importance < 0.3",
            [],
        )?;
        Ok(())
    }
}

/// 截断文本到指定字符数，围绕匹配词
fn truncate_around_match(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let truncated: String = text.chars().take(max_chars).collect();

    // 尝试在句子边界截断
    if let Some(last_period) = truncated.rfind('.') {
        return format!("{}...", &truncated[..last_period + 1]);
    }

    // 尝试在空格边界截断
    if let Some(last_space) = truncated.rfind(' ') {
        return format!("{}...", &truncated[..last_space]);
    }

    // 直接截断
    format!("{}...", truncated)
}

/// 截断到句子边界
fn truncate_to_sentence_boundary(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let truncated: String = text.chars().take(max_chars).collect();

    // 尝试在句子边界截断
    if let Some(last_period) = truncated.rfind('.') {
        return text[..last_period + 1].to_string();
    }

    // 尝试在换行处截断
    if let Some(last_newline) = truncated.rfind('\n') {
        return text[..last_newline].to_string();
    }

    // 直接截断
    truncated
}

/// 记忆片段
#[derive(Debug, Clone)]
pub struct MemoryFragment {
    pub id: i64,
    pub tick: u32,
    pub text_summary: String,
    pub emotion_tag: String,
    pub event_type: String,
    pub importance: f32,
    pub created_at: i64,
}
