//! 移动相关动作处理器
//!
//! MoveToward

use crate::agent::RelationType;
use crate::types::{AgentId, Position, StructureType};
use crate::world::{ActionResult, World};
use crate::narrative::{NarrativeBuilder, EventType};

impl World {
    /// MoveToward：单步移动到相邻格
    pub fn handle_move_toward(&mut self, agent_id: &AgentId, target: Position) -> ActionResult {
        let (agent_name, current_pos) = {
            let agent = self.agents.get(agent_id).unwrap();
            (agent.name.clone(), agent.position)
        };
        let builder = NarrativeBuilder::new(agent_name.clone());

        // 如果已在目标位置
        if current_pos == target {
            self.record_event(agent_id, &agent_name, EventType::MoveToward.as_str(),
                &builder.already_at_position(target), "#888888");
            return ActionResult::AlreadyAtPosition(
                format!("你已经在 ({},{})，不需要再移动。请选择其他动作（如采集附近资源、探索其他方向等）", target.x, target.y));
        }

        // 校验：目标必须与当前位置相邻（World职责）
        let dist = current_pos.manhattan_distance(&target);
        if dist != 1 {
            return ActionResult::Blocked(
                format!("目标 ({},{}) 不相邻（距离 {} 格），每次只能移动 1 格", target.x, target.y, dist));
        }

        // 边界检查（World职责）
        if target.x >= self.map.size().0 || target.y >= self.map.size().1 {
            return ActionResult::OutOfBounds;
        }

        // Fence 碰撞检查（World职责）
        if let Some(fence) = self.structures.get(&target) {
            if fence.structure_type == StructureType::Fence {
                if let Some(ref owner_id) = fence.owner_id {
                    let is_enemy = self.agents.get(agent_id)
                        .and_then(|a| a.relations.get(owner_id))
                        .map(|r| r.relation_type == RelationType::Enemy)
                        .unwrap_or(false);
                    if is_enemy {
                        return ActionResult::Blocked("被围栏阻挡，无法通过敌对领地".into());
                    }
                }
            }
        }

        // 执行移动：调用 Agent 方法
        let agent = self.agents.get_mut(agent_id).unwrap();
        let (_, old_pos, new_pos) = agent.move_to(target);

        // 使用 NarrativeBuilder 生成描述
        self.record_event(agent_id, &agent_name, EventType::MoveToward.as_str(),
            &builder.move_toward(old_pos, new_pos), EventType::MoveToward.color_code());

        // 返回成功详情
        ActionResult::SuccessWithDetail(format!("move:{},{}→({},{})", old_pos.x, old_pos.y, new_pos.x, new_pos.y))
    }
}