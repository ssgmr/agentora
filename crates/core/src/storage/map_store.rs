//! 地图持久化

use crate::types::{Position, TerrainType};
use crate::world::map::CellGrid;
use rusqlite::Connection;

/// 保存地图单元格
pub fn save_cell(conn: &Connection, pos: Position, terrain: TerrainType, region_id: u32) -> Result<(), rusqlite::Error> {
    let terrain_name = terrain_name(terrain);
    conn.execute(
        "INSERT OR REPLACE INTO map_cells (x, y, terrain, region_id, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            pos.x,
            pos.y,
            terrain_name,
            region_id,
            chrono::Utc::now().timestamp(),
        ],
    )?;
    Ok(())
}

/// 加载整个地图
pub fn load_map(conn: &Connection, width: u32, height: u32) -> Result<CellGrid, rusqlite::Error> {
    let mut grid = CellGrid::new(width, height);
    let mut stmt = conn.prepare("SELECT x, y, terrain FROM map_cells")?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?, row.get::<_, String>(2)?))
    })?;

    for row in rows {
        let (x, y, terrain_name) = row?;
        let terrain = terrain_from_name(&terrain_name);
        grid.set_terrain(Position::new(x, y), terrain);
    }

    Ok(grid)
}

/// 更新资源状态
pub fn update_resource(conn: &Connection, pos: Position, resource_type: &str, current: u32, max: u32) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE map_cells SET resource_type = ?3, resource_current = ?4, resource_max = ?5, updated_at = ?6 WHERE x = ?1 AND y = ?2",
        rusqlite::params![
            pos.x,
            pos.y,
            resource_type,
            current,
            max,
            chrono::Utc::now().timestamp(),
        ],
    )?;
    Ok(())
}

fn terrain_name(terrain: TerrainType) -> String {
    match terrain {
        TerrainType::Plains => "plains",
        TerrainType::Forest => "forest",
        TerrainType::Mountain => "mountain",
        TerrainType::Water => "water",
        TerrainType::Desert => "desert",
    }.to_string()
}

fn terrain_from_name(name: &str) -> TerrainType {
    match name {
        "plains" => TerrainType::Plains,
        "forest" => TerrainType::Forest,
        "mountain" => TerrainType::Mountain,
        "water" => TerrainType::Water,
        "desert" => TerrainType::Desert,
        _ => TerrainType::Plains,
    }
}