//! 遗产交互动作处理器
//!
//! InteractLegacy

use crate::types::AgentId;
use crate::world::{ActionResult, World};

impl World {
    /// InteractLegacy：与遗产交互
    pub fn handle_legacy_interaction(&mut self, agent_id: &AgentId, legacy_id: &str, interaction: &crate::types::LegacyInteraction) -> ActionResult {
        let agent_pos = self.agents.get(agent_id).unwrap().position;

        let legacy_index = self.legacies.iter().position(|l| l.id == legacy_id);
        if legacy_index.is_none() {
            return ActionResult::InvalidAgent;
        }

        if self.legacies[legacy_index.unwrap()].position != agent_pos {
            return ActionResult::Blocked("不在遗产位置，无法交互".into());
        }

        match interaction {
            crate::types::LegacyInteraction::Worship => {
                self.total_legacy_interacts += 1;
                ActionResult::SuccessWithDetail("legacy:worship".into())
            }
            crate::types::LegacyInteraction::Pickup => {
                let legacy = &mut self.legacies[legacy_index.unwrap()];
                if legacy.items.is_empty() {
                    return ActionResult::Blocked("遗产无物品可拾取".into());
                }

                let mut items_to_transfer = Vec::new();
                for (item_name, amount) in &legacy.items {
                    if *amount > 0 {
                        items_to_transfer.push((item_name.clone(), *amount));
                        break;
                    }
                }

                if items_to_transfer.is_empty() {
                    return ActionResult::Blocked("拾取失败".into());
                }

                let (item_name, amount) = items_to_transfer[0].clone();
                let agent = self.agents.get_mut(agent_id).unwrap();
                let current = *agent.inventory.get(&item_name).unwrap_or(&0);
                agent.inventory.insert(item_name.clone(), current + amount);

                let legacy = &mut self.legacies[legacy_index.unwrap()];
                legacy.items.insert(item_name.clone(), amount - 1);

                self.total_legacy_interacts += 1;
                ActionResult::SuccessWithDetail(format!("legacy:pickup {}x{}", item_name, amount))
            }
        }
    }
}