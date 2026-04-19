//! 策略库持久化

use crate::strategy::Strategy;
use rusqlite::Connection;

/// 保存策略
pub fn save_strategy(conn: &Connection, strategy: &Strategy) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO strategies (spark_type, success_rate, use_count, last_used_tick, deprecated, created_tick)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            strategy.spark_type,
            strategy.success_rate,
            strategy.use_count,
            strategy.last_used_tick,
            strategy.deprecated,
            strategy.created_tick,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

/// 加载所有策略
pub fn load_all_strategies(conn: &Connection) -> Result<Vec<Strategy>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT spark_type, success_rate, use_count, last_used_tick, deprecated, created_tick FROM strategies WHERE deprecated = 0"
    )?;

    let strategies = stmt.query_map([], |row| {
        Ok(Strategy {
            spark_type: row.get(0)?,
            success_rate: row.get(1)?,
            use_count: row.get(2)?,
            last_used_tick: row.get(3)?,
            deprecated: row.get::<_, bool>(4)?,
            created_tick: row.get(5)?,
            content: String::new(),
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(strategies)
}

/// 更新策略成功率
pub fn update_strategy_success(conn: &Connection, spark_type: &str, success: bool) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE strategies SET success_rate = (success_rate * use_count + ?2) / (use_count + 1), use_count = use_count + 1 WHERE spark_type = ?1",
        rusqlite::params![spark_type, if success { 1.0 } else { 0.0 }],
    )?;
    Ok(())
}