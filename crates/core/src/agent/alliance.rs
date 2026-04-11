//! 结盟系统

use crate::agent::{Relation, RelationType};
use crate::types::AgentId;

const ALLY_TRUST_THRESHOLD: f32 = 0.5;
const ALLY_TRADE_BONUS: f32 = 0.1;

impl crate::agent::Agent {
    /// 发起结盟提议
    /// 需要信任值 > 0.5
    pub fn propose_alliance(&self, target: AgentId) -> bool {
        if let Some(rel) = self.relations.get(&target) {
            rel.trust > ALLY_TRUST_THRESHOLD
        } else {
            false
        }
    }

    /// 接受结盟
    pub fn accept_alliance(&mut self, ally_id: AgentId) {
        self.relations.insert(ally_id.clone(), Relation {
            trust: 0.7,
            relation_type: RelationType::Ally,
            last_interaction_tick: 0,
        });
    }

    /// 拒绝结盟
    pub fn reject_alliance(&mut self, ally_id: AgentId) {
        // 略微降低信任
        if let Some(rel) = self.relations.get_mut(&ally_id) {
            rel.trust = (rel.trust - 0.1).max(0.0);
        }
    }

    /// 解除结盟（背叛）
    pub fn break_alliance(&mut self, ally_id: AgentId) {
        if let Some(rel) = self.relations.get_mut(&ally_id) {
            rel.relation_type = RelationType::Neutral;
            rel.trust = 0.1; // 大幅降低信任
        }
    }

    /// 获取盟友交易效率加成
    pub fn get_trade_bonus_for_allies(&self) -> f32 {
        ALLY_TRADE_BONUS
    }
}