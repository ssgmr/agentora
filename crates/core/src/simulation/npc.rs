//! NPC Agent 创建和管理
//!
//! 使用规则引擎快速决策的 NPC Agent

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{World, Agent, AgentId, Position};
use super::SimConfig;

/// NPC Agent 创建
pub async fn create_npc_agents(
    world_arc: &Arc<Mutex<World>>,
    config: &SimConfig,
) -> Vec<AgentId> {
    let mut ids = Vec::new();

    if config.npc_count == 0 {
        return ids;
    }

    let mut world = world_arc.lock().await;

    let npc_names = ["Explorer", "Miner", "Builder", "Trader", "Guard", "Scout", "Gatherer", "Hunter", "Farmer", "Nomad"];

    // NPC spawn 位置（地图中心附近，确保相机能看到）
    let cx = 128u32;
    let cy = 128u32;
    let npc_positions = [
        (cx, cy), (cx + 5, cy), (cx - 5, cy), (cx, cy + 5), (cx, cy - 5),
        (cx + 10, cy + 10), (cx - 10, cy - 10), (cx + 10, cy - 10), (cx - 10, cy + 10), (cx + 15, cy),
    ];

    for i in 0..config.npc_count.min(npc_names.len()).min(npc_positions.len()) {
        let name = format!("[NPC]{}", npc_names[i]);
        let (mut x, mut y) = npc_positions[i];

        // 确保出生位置可通行，如果不可通行则找附近可通行位置
        let pos = Position::new(x, y);
        if !world.map.get_terrain(pos).is_passable() {
            // 在附近 5x5 范围内找可通行位置
            let mut found = false;
            for dx in 0..=5u32 {
                for dy in 0..=5u32 {
                    let nx = x.saturating_add(dx).min(255);
                    let ny = y.saturating_add(dy).min(255);
                    let trial = Position::new(nx, ny);
                    if world.map.get_terrain(trial).is_passable() {
                        x = nx;
                        y = ny;
                        found = true;
                        break;
                    }
                }
                if found { break; }
            }
        }

        let agent = Agent::new(
            AgentId::default(),
            name.clone(),
            Position::new(x, y),
        );

        let aid = agent.id.clone();
        world.insert_agent_at(aid.clone(), agent);
        ids.push(aid);
    }

    ids
}