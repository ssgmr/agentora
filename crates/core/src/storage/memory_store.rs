//! 记忆CRUD

use rusqlite::Connection;

/// 写入记忆片段
pub fn insert_memory_fragment(
    conn: &Connection,
    tick: u32,
    text_summary: &str,
    emotion_tag: &str,
    event_type: &str,
    importance: f32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO memory_fragments (tick, text_summary, emotion_tag, event_type, importance, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            tick,
            text_summary,
            emotion_tag,
            event_type,
            importance,
            chrono::Utc::now().timestamp(),
        ],
    )?;
    Ok(())
}

/// FTS5检索记忆
pub fn search_memories(conn: &Connection, query: &str, limit: usize) -> Result<Vec<MemoryFragmentRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, tick, text_summary, emotion_tag, event_type, importance
         FROM memory_fragments
         WHERE memory_fts MATCH ?1
         ORDER BY importance DESC
         LIMIT ?2"
    )?;

    let rows = stmt.query_map(rusqlite::params![query, limit], |row| {
        Ok(MemoryFragmentRow {
            id: row.get(0)?,
            tick: row.get(1)?,
            text_summary: row.get(2)?,
            emotion_tag: row.get(3)?,
            event_type: row.get(4)?,
            importance: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// 记忆衰减
pub fn decay_memories(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute("UPDATE memory_fragments SET importance = importance * 0.95", [])?;
    conn.execute("DELETE FROM memory_fragments WHERE importance < 0.3", [])?;
    Ok(())
}

/// 记忆片段行
#[derive(Debug, Clone)]
pub struct MemoryFragmentRow {
    pub id: i64,
    pub tick: u32,
    pub text_summary: String,
    pub emotion_tag: String,
    pub event_type: String,
    pub importance: f32,
}