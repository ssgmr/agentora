//! 世界 tick 循环逻辑
//!
//! 包含生存消耗、建筑效果、死亡处理、遗产衰减等 tick 相关方法。

use crate::world::World;
use crate::world::resource;
use crate::world::legacy::{Legacy, LegacyEvent};
use crate::types::{AgentId, ResourceType, StructureType, Position};
use crate::snapshot::NarrativeEvent;

impl World {
    /// 生存消耗 tick：饱食度和水分度衰减，耗尽时掉血
    /// 每 tick 衰减 1 点（tick 间隔由配置决定，默认 5 秒）
    pub fn survival_consumption_tick(&mut self) {
        for (_, agent) in self.agents.iter_mut() {
            if !agent.is_alive {
                continue;
            }
            // 每 tick 衰减 1 点
            agent.satiety = agent.satiety.saturating_sub(1);
            agent.hydration = agent.hydration.saturating_sub(1);

            // 饱食度耗尽：HP -1/tick
            if agent.satiety == 0 {
                agent.health = agent.health.saturating_sub(1);
            }
            // 水分度耗尽：HP -1/tick
            if agent.hydration == 0 {
                agent.health = agent.health.saturating_sub(1);
            }
        }
    }

    /// 建筑效果 tick
    pub fn structure_effects_tick(&mut self) {
        // Camp 回血效果：曼哈顿距离 ≤ 1 的存活 Agent HP +2
        let camp_positions: Vec<Position> = self.structures.iter()
            .filter(|(_, s)| s.structure_type == StructureType::Camp)
            .map(|(pos, _)| *pos)
            .collect();

        for camp_pos in &camp_positions {
            let mut healed_agents: Vec<(AgentId, u32)> = Vec::new();
            for (_, agent) in self.agents.iter() {
                if !agent.is_alive { continue; }
                if agent.position.manhattan_distance(camp_pos) <= 1 && agent.health < agent.max_health {
                    let restored = 2.min(agent.max_health - agent.health);
                    healed_agents.push((agent.id.clone(), restored));
                }
            }
            for (agent_id, hp_restored) in healed_agents {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.health = (agent.health + hp_restored).min(agent.max_health);
                }
            }
        }
    }

    /// 检查 Agent 死亡并产生遗产（任务 3.2）
    pub fn check_agent_death(&mut self) {
        let dead_agent_ids: Vec<AgentId> = self.agents
            .iter()
            .filter(|(_, agent)| agent.is_alive && (agent.age >= agent.max_age || agent.health == 0))
            .map(|(id, _)| id.clone())
            .collect();

        for agent_id in dead_agent_ids {
            let agent = self.agents.get(&agent_id).unwrap();
            if !agent.is_alive {
                continue;
            }

            let agent_name = agent.name.clone();
            let agent_pos = agent.position;

            // 资源散落：将背包资源散落在当前位置
            let scattered: Vec<(String, u32)> = agent.inventory.iter()
                .filter(|(_, v)| **v > 0)
                .map(|(k, v)| (k.clone(), *v))
                .collect();

            for (res_type, amount) in &scattered {
                if let Some(node) = self.resources.get_mut(&agent_pos) {
                    // 如果当前位置已有资源节点，增加数量
                    if format!("{:?}", node.resource_type) == *res_type {
                        node.current_amount += amount;
                    }
                } else {
                    // 创建新资源节点
                    let resource_type = match res_type.as_str() {
                        "iron" => ResourceType::Iron,
                        "food" => ResourceType::Food,
                        "wood" => ResourceType::Wood,
                        "water" => ResourceType::Water,
                        "stone" => ResourceType::Stone,
                        _ => ResourceType::Food,
                    };
                    let node = resource::ResourceNode::new(agent_pos, resource_type, *amount);
                    self.resources.insert(agent_pos, node);
                }
            }

            // 创建遗产
            let legacy = Legacy::from_agent(agent, self.tick);
            let legacy_event = LegacyEvent::from_legacy(&legacy);

            // 添加到遗产列表
            self.legacies.push(legacy);

            // 标记 Agent 为死亡
            let agent = self.agents.get_mut(&agent_id).unwrap();
            agent.is_alive = false;

            // 清理死亡 Agent 的位置记录
            if let Some(ids) = self.agent_positions.get_mut(&agent_pos) {
                ids.retain(|id| *id != agent_id);
                if ids.is_empty() {
                    self.agent_positions.remove(&agent_pos);
                }
            }

            // 记录死亡事件
            let res_desc = if scattered.is_empty() {
                String::new()
            } else {
                format!("，留下: {}", scattered.iter().map(|(r, a)| format!("{}x{}", r, a)).collect::<Vec<_>>().join(", "))
            };
            self.tick_events.push(NarrativeEvent {
                tick: self.tick,
                agent_id: agent_id.as_str().to_string(),
                agent_name: agent_name.clone(),
                event_type: "death".to_string(),
                description: format!("{} 已死亡{}{}", agent_name, res_desc, if !scattered.is_empty() { "，资源散落在地".to_string() } else { String::new() }),
                color_code: "#FF0000".to_string(),
            });

            tracing::info!("Agent {} 死亡，产生遗产 {}", agent_name, legacy_event.legacy_id);

            // 3.2 广播到"legacy"topic（简化实现，实际应通过网络层广播）
            // TODO: 调用网络层 broadcast_to_topic("legacy", legacy_event)
        }
    }

    /// 遗产衰减
    pub fn decay_legacies(&mut self) {
        for legacy in &mut self.legacies {
            if legacy.is_decaying(self.tick) {
                legacy.decay_items();
            }
        }

        // 4.4 清理空遗迹（物品全部消失且超过 100 tick）
        self.legacies.retain(|legacy| {
            !legacy.items.is_empty() || (self.tick - legacy.created_tick) < 100
        });
    }
}