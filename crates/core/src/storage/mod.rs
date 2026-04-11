//! 存储与持久化系统

pub mod schema;
pub mod agent_store;
pub mod memory_store;
pub mod strategy_store;
pub mod map_store;
pub mod world_store;

use rusqlite::Connection;

/// 数据存储管理器
pub struct StorageManager {
    conn: Connection,
}

impl StorageManager {
    pub fn new(db_path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;
        let manager = Self { conn };
        manager.init_schema()?;
        Ok(manager)
    }

    /// 初始化表结构
    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        schema::create_tables(&self.conn)?;
        Ok(())
    }

    /// 获取连接
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}