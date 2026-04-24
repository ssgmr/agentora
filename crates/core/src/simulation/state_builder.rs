//! WorldState 构建器
//!
//! 从 World 自动构建 WorldState，消除 agent_loop.rs 中的手动组装代码。
//! 集成视野扫描、压力事件、临时偏好提取。

use crate::rule_engine::WorldState;
use crate::world::World;
use crate::world::vision::scan_vision;
use crate::types::{AgentId, ResourceType};
use crate::world::vision::calculate_direction;
use std::collections::HashMap;

/// WorldState 构建器
pub struct WorldStateBuilder;

impl WorldStateBuilder {
    /// 从 World 自动构建 WorldState
    ///
    /// # 参数
    /// - `world`: 世界状态引用
    /// - `agent_id`: 目标 Agent ID
    /// - `vision_radius`: 视野半径
    ///
    /// # 返回
    /// - `Option<WorldState>`: Agent 存在时返回 WorldState，否则返回 None
    pub fn build(world: &World, agent_id: &AgentId, vision_radius: u32) -> Option<WorldState> {
        // 1. 获取 Agent 基本信息
        let agent = world.agents.get(agent_id)?;

        // Agent 已死亡时不构建状态
        if !agent.is_alive {
            return None;
        }

        // 2. 执行视野扫描
        let vision = scan_vision(world, agent_id, vision_radius);

        tracing::debug!(
            "[WorldStateBuilder] Agent {:?} vision: {} terrain, {} resources, {} agents, {} structures, {} legacies",
            agent_id,
            vision.terrain_at.len(),
            vision.resources_at.len(),
            vision.nearby_agents.len(),
            vision.nearby_structures.len(),
            vision.nearby_legacies.len()
        );

        // 3. 构建库存映射（字符串 key → ResourceType）
        let inventory: HashMap<ResourceType, u32> = agent.inventory.iter()
            .map(|(k, v)| {
                let resource = match k.as_str() {
                    "iron" => ResourceType::Iron,
                    "food" => ResourceType::Food,
                    "wood" => ResourceType::Wood,
                    "water" => ResourceType::Water,
                    "stone" => ResourceType::Stone,
                    _ => ResourceType::Food,
                };
                (resource, *v)
            })
            .collect();

        // 4. 获取地图尺寸
        let (map_width, _map_height) = world.map.size();

        // 5. 构建完整 WorldState
        Some(WorldState {
            map_size: map_width,
            agent_position: agent.position,
            agent_inventory: inventory,
            agent_satiety: agent.satiety,
            agent_hydration: agent.hydration,
            terrain_at: vision.terrain_at,
            self_id: agent_id.clone(),
            existing_agents: world.agents.keys().cloned().collect(),
            resources_at: vision.resources_at,
            nearby_agents: vision.nearby_agents,
            nearby_structures: vision.nearby_structures,
            nearby_legacies: vision.nearby_legacies,
            active_pressures: world.pressure_pool.iter()
                .map(|p| p.description.clone())
                .collect(),
            last_move_direction: agent.last_position.and_then(|last_pos| {
                calculate_direction(&last_pos, &agent.position)
            }),
            temp_preferences: agent.temp_preferences.iter()
                .map(|p| (p.key.clone(), p.boost, p.remaining_ticks))
                .collect(),
            agent_personality: Some(agent.personality.clone()),
            pending_trades: world.pending_trades.iter()
                .filter(|t| t.acceptor_id == *agent_id)
                .map(|t| crate::rule_engine::PendingTradeInfo {
                    trade_id: t.trade_id.clone(),
                    proposer_name: world.agents.get(&t.proposer_id).map(|a| a.name.clone()).unwrap_or_else(|| "未知".to_string()),
                    proposer_id: t.proposer_id.clone(),
                    offer: t.offer_resources.iter()
                        .filter_map(|(k, v)| Some((k.parse().ok()?, *v)))
                        .collect(),
                    want: t.want_resources.iter()
                        .filter_map(|(k, v)| Some((k.parse().ok()?, *v)))
                        .collect(),
                })
                .collect(),
            pending_ally_requests: Vec::new(),
        })
    }

    /// 获取 Agent 克隆（用于后续决策）
    ///
    /// 分离此方法以避免在构建 WorldState 时同时持有 world 的多个引用
    pub fn get_agent_clone(world: &World, agent_id: &AgentId) -> Option<crate::agent::Agent> {
        world.agents.get(agent_id).cloned()
    }
}