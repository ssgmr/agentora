//! Agent状态CRUD

use crate::agent::Agent;
use crate::types::{AgentId, Position, PersonalitySeed};
use rusqlite::Connection;

/// 保存Agent
pub fn save_agent(conn: &Connection, agent: &Agent) -> Result<(), rusqlite::Error> {
    // 序列化人格种子
    let personality_bytes: Vec<u8> = [
        agent.personality.openness,
        agent.personality.agreeableness,
        agent.personality.neuroticism,
    ].iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();

    conn.execute(
        "INSERT OR REPLACE INTO agents (id, name, position_x, position_y, health, max_health, age, personality_seed, is_alive, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            agent.id.as_str(),
            agent.name,
            agent.position.x,
            agent.position.y,
            agent.health,
            agent.max_health,
            agent.age,
            personality_bytes,
            agent.is_alive,
            chrono::Utc::now().timestamp(),
        ],
    )?;
    Ok(())
}

/// 加载Agent
pub fn load_agent(conn: &Connection, id: &AgentId) -> Result<Option<Agent>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, position_x, position_y, health, max_health, age, personality_seed, is_alive FROM agents WHERE id = ?1"
    )?;

    let result = stmt.query_row(rusqlite::params![id.as_str()], |row| {
        let personality_bytes: Vec<u8> = row.get(7)?;
        let personality = PersonalitySeed {
            openness: f32::from_le_bytes(personality_bytes[0..4].try_into().unwrap()),
            agreeableness: f32::from_le_bytes(personality_bytes[4..8].try_into().unwrap()),
            neuroticism: f32::from_le_bytes(personality_bytes[8..12].try_into().unwrap()),
            description: String::new(), // 从数据库加载时使用空字符串（旧数据兼容）
            custom_prompt: None,
            icon_id: None,
            custom_icon_path: None,
        };

        Ok(Agent {
            temp_preferences: Vec::new(),
            id: AgentId::new(row.get::<_, String>(0)?),
            name: row.get(1)?,
            position: Position::new(row.get(2)?, row.get(3)?),
            health: row.get(4)?,
            max_health: row.get(5)?,
            satiety: 100,
            hydration: 100,
            inventory: Default::default(),
            frozen_inventory: Default::default(),
            memory: Default::default(),
            relations: Default::default(),
            strategies: Default::default(),
            personality,
            age: row.get(6)?,
            max_age: 200,
            is_alive: row.get::<_, bool>(8)?,
            experience: 0,
            level: 1,
            last_action_type: None,
            last_action_result: None,
            last_position: None,
            pending_trade_id: None,
        })
    });

    match result {
        Ok(agent) => Ok(Some(agent)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// 更新Agent位置
pub fn update_position(conn: &Connection, id: &AgentId, position: &Position) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE agents SET position_x = ?2, position_y = ?3, updated_at = ?4 WHERE id = ?1",
        rusqlite::params![id.as_str(), position.x, position.y, chrono::Utc::now().timestamp()],
    )?;
    Ok(())
}

/// 更新Agent健康值
pub fn update_health(conn: &Connection, id: &AgentId, health: u32) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE agents SET health = ?2, is_alive = ?3, updated_at = ?4 WHERE id = ?1",
        rusqlite::params![id.as_str(), health, health > 0, chrono::Utc::now().timestamp()],
    )?;
    Ok(())
}
