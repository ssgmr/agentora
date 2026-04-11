//! 世界快照保存与恢复

use crate::world::World;
use crate::seed::WorldSeed;
use rusqlite::Connection;

/// 保存完整世界状态
pub fn save_world(conn: &Connection, world: &World) -> Result<(), rusqlite::Error> {
    // 保存所有Agent
    for agent in world.agents.values() {
        crate::storage::agent_store::save_agent(conn, agent)?;
    }

    // 保存地图
    let (width, height) = world.map.size();
    for y in 0..height {
        for x in 0..width {
            let pos = crate::types::Position::new(x, y);
            let terrain = world.map.get_terrain(pos);
            let region_id = crate::world::region::Region::position_to_region_id(x, y, 16);
            crate::storage::map_store::save_cell(conn, pos, terrain, region_id)?;
        }
    }

    // TODO: 保存资源、结构、遗产等

    Ok(())
}

/// 恢复世界状态
pub fn load_world(conn: &Connection, seed: &WorldSeed) -> Result<World, rusqlite::Error> {
    let mut world = World::new(seed);

    // 加载地图
    let [width, height] = seed.map_size;
    world.map = crate::storage::map_store::load_map(conn, width, height)?;

    // 加载Agent
    let mut stmt = conn.prepare("SELECT id FROM agents WHERE is_alive = 1")?;
    let ids: Vec<String> = stmt.query_map([], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;

    for id_str in ids {
        let id = crate::types::AgentId::new(id_str);
        if let Some(agent) = crate::storage::agent_store::load_agent(conn, &id)? {
            world.agents.insert(id, agent);
        }
    }

    Ok(world)
}