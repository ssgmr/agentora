//! 类型转换
//!
//! 将 Rust 类型转换为 GDScript Dictionary

use godot::prelude::*;
use agentora_core::simulation::{Delta, WorldEvent, ChangeHint};
use agentora_core::snapshot::AgentState;

/// 将 Delta 转为 GDScript Dictionary
pub fn delta_to_dict(delta: &Delta) -> Variant {
    let mut dict: Dictionary<GString, Variant> = Dictionary::new();
    match delta {
        Delta::AgentStateChanged { agent_id, state, change_hint, source_peer_id } => {
            dict.set("type", &"agent_state_changed".to_variant());
            dict.set("agent_id", &agent_id.to_variant());
            dict.set("change_hint", &change_hint_to_str(change_hint).to_variant());

            // Agent state（扁平结构，与 snapshot agent_to_dict 格式一致）
            dict.set("name", &state.name.clone().to_variant());
            let pos = Vector2::new(state.position.0 as f32, state.position.1 as f32);
            dict.set("position", &pos.to_variant());
            dict.set("health", &(Variant::from(state.health as i64)));
            dict.set("max_health", &(Variant::from(state.max_health as i64)));
            dict.set("satiety", &(Variant::from(state.satiety as i64)));
            dict.set("hydration", &(Variant::from(state.hydration as i64)));
            dict.set("is_alive", &state.is_alive.to_variant());
            dict.set("age", &(Variant::from(state.age as i64)));
            dict.set("level", &(Variant::from(state.level as i64)));
            dict.set("current_action", &state.current_action.clone().to_variant());
            dict.set("action_result", &state.action_result.clone().to_variant());
            dict.set("reasoning", &state.reasoning.clone().unwrap_or_default().to_variant());

            // Inventory
            let mut inv_dict: Dictionary<GString, Variant> = Dictionary::new();
            for (k, v) in &state.inventory_summary {
                inv_dict.set(k, &(Variant::from(*v as i64)));
            }
            dict.set("inventory_summary", &inv_dict.to_variant());

            // 来源 peer ID（P2P 远程 Agent）
            if let Some(ref peer_id) = source_peer_id {
                dict.set("source_peer_id", &peer_id.to_variant());
            }
        }
        Delta::WorldEvent(world_event) => {
            dict.set("type", &"world_event".to_variant());
            dict.set("event_type", &world_event.event_type().to_variant());
            match world_event {
                WorldEvent::StructureCreated { pos, structure_type, owner_id } => {
                    let position = Vector2::new(pos.0 as f32, pos.1 as f32);
                    dict.set("position", &position.to_variant());
                    dict.set("structure_type", &structure_type.to_variant());
                    dict.set("owner_id", &owner_id.to_variant());
                }
                WorldEvent::StructureDestroyed { pos, structure_type } => {
                    let position = Vector2::new(pos.0 as f32, pos.1 as f32);
                    dict.set("position", &position.to_variant());
                    dict.set("structure_type", &structure_type.to_variant());
                }
                WorldEvent::ResourceChanged { pos, resource_type, amount } => {
                    let position = Vector2::new(pos.0 as f32, pos.1 as f32);
                    dict.set("position", &position.to_variant());
                    dict.set("resource_type", &resource_type.to_variant());
                    dict.set("amount", &(Variant::from(*amount as i64)));
                }
                WorldEvent::TradeCompleted { from_id, to_id, items } => {
                    dict.set("from_id", &from_id.to_variant());
                    dict.set("to_id", &to_id.to_variant());
                    dict.set("items", &items.to_variant());
                }
                WorldEvent::AllianceFormed { id1, id2 } => {
                    dict.set("id1", &id1.to_variant());
                    dict.set("id2", &id2.to_variant());
                }
                WorldEvent::AllianceBroken { id1, id2, reason } => {
                    dict.set("id1", &id1.to_variant());
                    dict.set("id2", &id2.to_variant());
                    dict.set("reason", &reason.to_variant());
                }
                WorldEvent::MilestoneReached { name, display_name, tick } => {
                    dict.set("name", &name.to_variant());
                    dict.set("display_name", &display_name.to_variant());
                    dict.set("tick", &(Variant::from(*tick as i64)));
                }
                WorldEvent::PressureStarted { pressure_type, description, duration } => {
                    dict.set("pressure_type", &pressure_type.to_variant());
                    dict.set("description", &description.to_variant());
                    dict.set("duration", &(Variant::from(*duration as i64)));
                }
                WorldEvent::PressureEnded { pressure_type, description } => {
                    dict.set("pressure_type", &pressure_type.to_variant());
                    dict.set("description", &description.to_variant());
                }
                WorldEvent::AgentNarrative { narrative } => {
                    dict.set("narrative_tick", &(Variant::from(narrative.tick as i64)));
                    dict.set("narrative_agent_id", &narrative.agent_id.to_variant());
                    dict.set("narrative_agent_name", &narrative.agent_name.to_variant());
                    dict.set("narrative_event_type", &narrative.event_type.to_variant());
                    dict.set("narrative_description", &narrative.description.to_variant());
                    dict.set("narrative_color", &narrative.color_code.to_variant());
                    // 添加频道字段
                    let channel_str = match narrative.channel {
                        agentora_core::snapshot::NarrativeChannel::Local => "local",
                        agentora_core::snapshot::NarrativeChannel::Nearby => "nearby",
                        agentora_core::snapshot::NarrativeChannel::World => "world",
                    };
                    dict.set("narrative_channel", &channel_str.to_variant());
                }
            }
        }
    }
    dict.to_variant()
}

/// 将 ChangeHint 转为字符串
fn change_hint_to_str(hint: &ChangeHint) -> &'static str {
    match hint {
        ChangeHint::Spawned => "spawned",
        ChangeHint::Moved => "moved",
        ChangeHint::ActionExecuted => "action_executed",
        ChangeHint::Died => "died",
        ChangeHint::SurvivalLow => "survival_low",
        ChangeHint::Healed => "healed",
    }
}

/// 将 AgentState 转为 GDScript Dictionary
pub fn agent_to_dict(agent: &AgentState) -> Variant {
    let mut dict: Dictionary<GString, Variant> = Dictionary::new();
    dict.set("id", &agent.id.clone().to_variant());
    dict.set("name", &agent.name.clone().to_variant());
    dict.set("health", &(Variant::from(agent.health as i64)));
    dict.set("max_health", &(Variant::from(agent.max_health as i64)));
    dict.set("satiety", &(Variant::from(agent.satiety as i64)));
    dict.set("hydration", &(Variant::from(agent.hydration as i64)));
    dict.set("is_alive", &agent.is_alive.to_variant());
    dict.set("age", &(Variant::from(agent.age as i64)));
    dict.set("level", &(Variant::from(agent.level as i64)));
    dict.set("current_action", &agent.current_action.clone().to_variant());
    dict.set("action_result", &agent.action_result.clone().to_variant());
    dict.set("reasoning", &agent.reasoning.clone().unwrap_or_default().to_variant());
    let pos = Vector2::new(agent.position.0 as f32, agent.position.1 as f32);
    dict.set("position", &pos.to_variant());
    let mut inv_dict: Dictionary<GString, Variant> = Dictionary::new();
    for (k, v) in &agent.inventory_summary {
        inv_dict.set(k, &(Variant::from(*v as i64)));
    }
    dict.set("inventory_summary", &inv_dict.to_variant());
    dict.to_variant()
}

/// 将 WorldSnapshot 转为 GDScript Dictionary
pub fn snapshot_to_dict(snapshot: &agentora_core::WorldSnapshot) -> Variant {
    let mut snapshot_dict: Dictionary<GString, Variant> = Dictionary::new();
    snapshot_dict.set("tick", &(Variant::from(snapshot.tick as i64)));

    // Agents
    let mut agents_dict: Dictionary<GString, Variant> = Dictionary::new();
    for agent in &snapshot.agents {
        let agent_data = agent_to_dict(agent);
        agents_dict.set(agent.id.as_str(), &agent_data);
    }
    snapshot_dict.set("agents", &agents_dict.to_variant());

    // Map changes
    let mut map_changes_array: Array<Variant> = Array::new();
    for change in &snapshot.map_changes {
        let mut change_dict: Dictionary<GString, Variant> = Dictionary::new();
        change_dict.set("x", &(Variant::from(change.x as i64)));
        change_dict.set("y", &(Variant::from(change.y as i64)));
        change_dict.set("terrain", &change.terrain.to_variant());
        if let Some(structure) = &change.structure {
            change_dict.set("structure", &structure.to_variant());
        }
        if let Some(owner_id) = &change.structure_owner_id {
            change_dict.set("owner_id", &owner_id.to_variant());
        }
        if let Some(resource_type) = &change.resource_type {
            change_dict.set("resource_type", &resource_type.to_variant());
        }
        if let Some(resource_amount) = &change.resource_amount {
            change_dict.set("resource_amount", &(Variant::from(*resource_amount as i64)));
        }
        map_changes_array.push(&change_dict.to_variant());
    }
    snapshot_dict.set("map_changes", &map_changes_array.to_variant());

    // Terrain grid
    if let (Some(grid), Some(w), Some(h)) = (&snapshot.terrain_grid, &snapshot.terrain_width, &snapshot.terrain_height) {
        let grid_packed = PackedByteArray::from(grid.as_slice());
        snapshot_dict.set("terrain_grid", &grid_packed.to_variant());
        snapshot_dict.set("terrain_width", &(Variant::from(*w as i64)));
        snapshot_dict.set("terrain_height", &(Variant::from(*h as i64)));
    }

    snapshot_dict.to_variant()
}