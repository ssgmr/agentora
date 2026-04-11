//! 攻击系统

use crate::agent::RelationType;
use crate::types::AgentId;

impl crate::agent::Agent {
    /// 攻击目标Agent
    pub fn attack(&mut self, target: &mut crate::agent::Agent, damage: u32) -> AttackResult {
        target.health = target.health.saturating_sub(damage);

        // 更新关系：标记为敌人
        if let Some(rel) = self.relations.get_mut(&target.id) {
            rel.relation_type = RelationType::Enemy;
            rel.trust = 0.0;
        } else {
            self.relations.insert(target.id.clone(), crate::agent::Relation {
                trust: 0.0,
                relation_type: RelationType::Enemy,
                last_interaction_tick: 0,
            });
        }

        // 目标也会标记攻击者为敌人
        if let Some(rel) = target.relations.get_mut(&self.id) {
            rel.relation_type = RelationType::Enemy;
            rel.trust = 0.0;
        } else {
            target.relations.insert(self.id.clone(), crate::agent::Relation {
                trust: 0.0,
                relation_type: RelationType::Enemy,
                last_interaction_tick: 0,
            });
        }

        AttackResult {
            attacker_id: self.id.clone(),
            target_id: target.id.clone(),
            damage,
            target_alive: target.health > 0,
        }
    }
}

/// 攻击结果
#[derive(Debug, Clone)]
pub struct AttackResult {
    pub attacker_id: AgentId,
    pub target_id: AgentId,
    pub damage: u32,
    pub target_alive: bool,
}