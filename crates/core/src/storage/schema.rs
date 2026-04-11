//! SQLite表结构定义

use rusqlite::Connection;

/// 创建所有表
pub fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch("
        -- Agent状态
        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            position_x INTEGER NOT NULL,
            position_y INTEGER NOT NULL,
            motivation_vector BLOB NOT NULL,
            health INTEGER NOT NULL DEFAULT 100,
            max_health INTEGER NOT NULL DEFAULT 100,
            age INTEGER NOT NULL DEFAULT 0,
            personality_seed BLOB,
            is_alive BOOLEAN NOT NULL DEFAULT 1,
            updated_at INTEGER NOT NULL
        );

        -- Agent背包
        CREATE TABLE IF NOT EXISTS inventory (
            agent_id TEXT NOT NULL,
            resource_type TEXT NOT NULL,
            quantity INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (agent_id, resource_type)
        );

        -- 记忆片段
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

        -- FTS5全文索引
        CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
            text_summary,
            emotion_tag,
            event_type,
            content='memory_fragments',
            content_rowid=id
        );

        -- 策略库索引
        CREATE TABLE IF NOT EXISTS strategies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            spark_type TEXT NOT NULL,
            success_rate REAL NOT NULL DEFAULT 1.0,
            use_count INTEGER NOT NULL DEFAULT 0,
            last_used_tick INTEGER NOT NULL,
            deprecated BOOLEAN NOT NULL DEFAULT 0,
            created_tick INTEGER NOT NULL,
            UNIQUE(spark_type)
        );

        -- 策略执行日志
        CREATE TABLE IF NOT EXISTS strategy_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            strategy_id INTEGER NOT NULL,
            tick INTEGER NOT NULL,
            action_type TEXT NOT NULL,
            result TEXT NOT NULL,
            motivation_delta BLOB,
            FOREIGN KEY (strategy_id) REFERENCES strategies(id)
        );

        -- 事件日志
        CREATE TABLE IF NOT EXISTS event_log (
            id TEXT PRIMARY KEY,
            tick INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            actor_id TEXT,
            data TEXT NOT NULL,
            peer_id TEXT NOT NULL,
            tag_counter INTEGER NOT NULL,
            is_removed BOOLEAN NOT NULL DEFAULT 0
        );

        -- 世界地图单元格
        CREATE TABLE IF NOT EXISTS map_cells (
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            terrain TEXT NOT NULL DEFAULT 'plains',
            structure_type TEXT,
            structure_owner TEXT,
            resource_type TEXT,
            resource_current INTEGER DEFAULT 0,
            resource_max INTEGER DEFAULT 0,
            region_id INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (x, y)
        );

        -- 遗迹
        CREATE TABLE IF NOT EXISTS legacies (
            id TEXT PRIMARY KEY,
            position_x INTEGER NOT NULL,
            position_y INTEGER NOT NULL,
            legacy_type TEXT NOT NULL,
            original_agent_id TEXT NOT NULL,
            items TEXT NOT NULL,
            echo_log TEXT,
            created_tick INTEGER NOT NULL
        );

        -- 社会关系
        CREATE TABLE IF NOT EXISTS relations (
            agent_id TEXT NOT NULL,
            other_id TEXT NOT NULL,
            trust REAL NOT NULL DEFAULT 0.5,
            relation_type TEXT NOT NULL DEFAULT 'neutral',
            last_interaction_tick INTEGER,
            PRIMARY KEY (agent_id, other_id)
        );
    ")?;
    Ok(())
}