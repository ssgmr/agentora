//! 攻击系统

use crate::agent::{Relation, RelationType};
use crate::types::AgentId;

impl crate::agent::Agent {
    /// 承受攻击：HP减少，记录攻击者为敌人
    /// 参数：damage 由 World 计算（base_damage * terrain_multiplier）
    pub fn receive_attack(&mut self, damage: u32, attacker_id: &AgentId) {
        self.health = self.health.saturating_sub(damage);

        if let Some(rel) = self.relations.get_mut(attacker_id) {
            rel.relation_type = RelationType::Enemy;
            rel.trust = 0.0;
        } else {
            self.relations.insert(attacker_id.clone(), Relation {
                trust: 0.0,
                relation_type: RelationType::Enemy,
                last_interaction_tick: 0,
            });
        }
    }

    /// 发起攻击：记录目标为敌人
    pub fn initiate_attack(&mut self, target_id: &AgentId) {
        if let Some(rel) = self.relations.get_mut(target_id) {
            rel.relation_type = RelationType::Enemy;
            rel.trust = 0.0;
        } else {
            self.relations.insert(target_id.clone(), Relation {
                trust: 0.0,
                relation_type: RelationType::Enemy,
                last_interaction_tick: 0,
            });
        }
    }
}