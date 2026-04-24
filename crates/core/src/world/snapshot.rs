//! 世界快照生成
//!
//! 生成 WorldSnapshot 用于客户端渲染和网络同步。
//! 使用统一的 AgentState 结构。

use crate::snapshot::{WorldSnapshot, AgentState, CellChange, NarrativeEvent, LegacyEvent, PressureSnapshot, MilestoneSnapshot, NarrativeChannel, AgentSource};
use crate::world::World;
use crate::types::Position;
use std::collections::HashSet;

impl World {
    /// 生成世界快照
    pub fn snapshot(&self) -> WorldSnapshot {
        let agents = self.agents
            .values()
            .filter(|a| a.is_alive)
            .map(|agent| {
                // reasoning：从 current_actions 获取（存储的是 action.reasoning）
                let reasoning = self.current_actions.get(&agent.id)
                    .map(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();

                // current_action：从 agent.last_action_type 获取（已存储简短描述）
                let current_action = agent.last_action_type.clone()
                    .unwrap_or_else(|| if reasoning.is_empty() { "等待".to_string() } else { "思考中...".to_string() });

                let action_result = agent.last_action_result.as_deref().unwrap_or("").to_string();

                AgentState {
                    id: agent.id.as_str().to_string(),
                    name: agent.name.clone(),
                    position: (agent.position.x, agent.position.y),
                    health: agent.health,
                    max_health: agent.max_health,
                    satiety: agent.satiety,
                    hydration: agent.hydration,
                    age: agent.age,
                    level: agent.level,
                    is_alive: agent.is_alive,
                    inventory_summary: agent.inventory.iter()
                        .map(|(k, v)| (k.clone(), *v))
                        .collect(),
                    current_action,
                    action_result,
                    reasoning: Some(reasoning),
                }
            })
            .collect();

        // 从 tick_events 填充 events（带频道信息）
        let events: Vec<NarrativeEvent> = self.tick_events.iter().map(|e| NarrativeEvent {
            tick: e.tick,
            agent_id: e.agent_id.clone(),
            agent_name: e.agent_name.clone(),
            event_type: e.event_type.clone(),
            description: e.description.clone(),
            color_code: e.color_code.clone(),
            channel: NarrativeChannel::Local, // 本地默认
            agent_source: AgentSource::Local,
        }).collect();

        // 从 legacies 填充 legacies
        let legacies: Vec<LegacyEvent> = self.legacies.iter().map(|l| LegacyEvent {
            id: l.id.clone(),
            position: (l.position.x, l.position.y),
            legacy_type: "agent_legacy".to_string(),
            original_agent_name: l.original_agent_name.clone(),
        }).collect();

        // 从 pressure_pool 填充 pressures
        let pressures = self.pressure_pool.iter().map(|p| PressureSnapshot {
            id: p.id.clone(),
            pressure_type: format!("{:?}", p.pressure_type),
            description: p.description.clone(),
            remaining_ticks: p.remaining_ticks,
        }).collect();

        // 从 structures 和 resources 填充 map_changes
        // 首先收集所有需要发送的位置
        let mut positions_to_send: HashSet<Position> = HashSet::new();

        // 收集建筑位置
        for pos in self.structures.keys() {
            positions_to_send.insert(*pos);
        }

        // 收集资源位置
        for (pos, node) in &self.resources {
            if !node.is_depleted && node.current_amount > 0 {
                positions_to_send.insert(*pos);
            }
        }

        let map_changes = positions_to_send.iter().map(|pos| {
            let terrain = format!("{:?}", self.map.get_terrain(*pos));
            let structure = self.structures.get(pos).map(|s| format!("{:?}", s.structure_type));
            let structure_owner_id = self.structures.get(pos).and_then(|s| s.owner_id.as_ref().map(|id| id.as_str().to_string()));
            let (resource_type, resource_amount) = self.resources.get(pos)
                .filter(|n| !n.is_depleted && n.current_amount > 0)
                .map(|n| (Some(n.resource_type.as_str().to_string()), Some(n.current_amount)))
                .unwrap_or((None, None));

            CellChange {
                x: pos.x,
                y: pos.y,
                terrain,
                structure,
                structure_owner_id,
                resource_type,
                resource_amount,
            }
        }).collect();

        // 构建地形网格数据（完整地形快照，用于Godot客户端渲染）
        let (width, height) = self.map.size();
        let terrain_grid: Vec<u8> = (0..height).flat_map(|y| {
            (0..width).map(|x| {
                self.map.get_terrain(Position::new(x, y)).to_index()
            }).collect::<Vec<_>>()
        }).collect();

        WorldSnapshot {
            tick: self.tick,
            agents,
            terrain_grid: Some(terrain_grid),
            terrain_width: Some(width),
            terrain_height: Some(height),
            map_changes,
            structures: std::collections::HashMap::new(), // 可后续填充
            resources: std::collections::HashMap::new(),   // 可后续填充
            pressures,
            milestones: self.milestones.iter().map(|m| MilestoneSnapshot {
                name: m.name.clone(),
                display_name: m.display_name.clone(),
                achieved_tick: m.achieved_tick,
            }).collect(),
        }
    }
}