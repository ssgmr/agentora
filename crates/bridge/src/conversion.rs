//! 类型转换
//!
//! 将 Rust 类型转换为 GDScript Dictionary

use godot::prelude::*;
use agentora_core::simulation::AgentDelta;
use agentora_core::snapshot::AgentSnapshot;

/// 将 AgentDelta 转为 GDScript Dictionary
pub fn delta_to_dict(delta: &AgentDelta) -> Variant {
    let mut dict: Dictionary<GString, Variant> = Dictionary::new();
    match delta {
        AgentDelta::AgentMoved { id, name, position, health, max_health, is_alive, age } => {
            dict.set("type", &"agent_moved".to_variant());
            dict.set("id", &id.to_variant());
            dict.set("name", &name.to_variant());
            let pos = Vector2::new(position.0 as f32, position.1 as f32);
            dict.set("position", &pos.to_variant());
            dict.set("health", &(Variant::from(*health as i64)));
            dict.set("max_health", &(Variant::from(*max_health as i64)));
            dict.set("is_alive", &is_alive.to_variant());
            dict.set("age", &(Variant::from(*age as i64)));
        }
        AgentDelta::AgentDied { id, name, position, age } => {
            dict.set("type", &"agent_died".to_variant());
            dict.set("id", &id.to_variant());
            dict.set("name", &name.to_variant());
            let pos = Vector2::new(position.0 as f32, position.1 as f32);
            dict.set("position", &pos.to_variant());
            dict.set("age", &(Variant::from(*age as i64)));
        }
        AgentDelta::AgentSpawned { id, name, position, health, max_health } => {
            dict.set("type", &"agent_spawned".to_variant());
            dict.set("id", &id.to_variant());
            dict.set("name", &name.to_variant());
            let pos = Vector2::new(position.0 as f32, position.1 as f32);
            dict.set("position", &pos.to_variant());
            dict.set("health", &(Variant::from(*health as i64)));
            dict.set("max_health", &(Variant::from(*max_health as i64)));
        }
        AgentDelta::StructureCreated { x, y, structure_type, owner_id } => {
            dict.set("type", &"structure_created".to_variant());
            dict.set("structure_type", &structure_type.to_variant());
            dict.set("owner_id", &owner_id.to_variant());
            let pos = Vector2::new(*x as f32, *y as f32);
            dict.set("position", &pos.to_variant());
        }
        AgentDelta::StructureDestroyed { x, y, structure_type } => {
            dict.set("type", &"structure_destroyed".to_variant());
            dict.set("structure_type", &structure_type.to_variant());
            let pos = Vector2::new(*x as f32, *y as f32);
            dict.set("position", &pos.to_variant());
        }
        AgentDelta::ResourceChanged { x, y, resource_type, amount } => {
            dict.set("type", &"resource_changed".to_variant());
            dict.set("resource_type", &resource_type.to_variant());
            dict.set("amount", &(Variant::from(*amount as i64)));
            let pos = Vector2::new(*x as f32, *y as f32);
            dict.set("position", &pos.to_variant());
        }
        AgentDelta::TradeCompleted { from_id, to_id, items } => {
            dict.set("type", &"trade_completed".to_variant());
            dict.set("from_id", &from_id.to_variant());
            dict.set("to_id", &to_id.to_variant());
            dict.set("items", &items.to_variant());
        }
        AgentDelta::AllianceFormed { id1, id2 } => {
            dict.set("type", &"alliance_formed".to_variant());
            dict.set("id1", &id1.to_variant());
            dict.set("id2", &id2.to_variant());
        }
        AgentDelta::AllianceBroken { id1, id2, reason } => {
            dict.set("type", &"alliance_broken".to_variant());
            dict.set("id1", &id1.to_variant());
            dict.set("id2", &id2.to_variant());
            dict.set("reason", &reason.to_variant());
        }
        AgentDelta::HealedByCamp { agent_id, agent_name, hp_restored } => {
            dict.set("type", &"healed_by_camp".to_variant());
            dict.set("agent_id", &agent_id.to_variant());
            dict.set("agent_name", &agent_name.to_variant());
            dict.set("hp_restored", &(Variant::from(*hp_restored as i64)));
        }
        AgentDelta::SurvivalWarning { agent_id, agent_name, satiety, hydration, hp } => {
            dict.set("type", &"survival_warning".to_variant());
            dict.set("agent_id", &agent_id.to_variant());
            dict.set("agent_name", &agent_name.to_variant());
            dict.set("satiety", &(Variant::from(*satiety as i64)));
            dict.set("hydration", &(Variant::from(*hydration as i64)));
            dict.set("hp", &(Variant::from(*hp as i64)));
        }
        AgentDelta::MilestoneReached { name, display_name, tick } => {
            dict.set("type", &"milestone_reached".to_variant());
            dict.set("name", &name.to_variant());
            dict.set("display_name", &display_name.to_variant());
            dict.set("tick", &(Variant::from(*tick as i64)));
        }
        AgentDelta::PressureStarted { pressure_type, description, duration } => {
            dict.set("type", &"pressure_started".to_variant());
            dict.set("pressure_type", &pressure_type.to_variant());
            dict.set("description", &description.to_variant());
            dict.set("duration", &(Variant::from(*duration as i64)));
        }
        AgentDelta::PressureEnded { pressure_type, description } => {
            dict.set("type", &"pressure_ended".to_variant());
            dict.set("pressure_type", &pressure_type.to_variant());
            dict.set("description", &description.to_variant());
        }
    }
    dict.to_variant()
}

/// 将 AgentSnapshot 转为 GDScript Dictionary
pub fn agent_to_dict(agent: &AgentSnapshot) -> Variant {
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