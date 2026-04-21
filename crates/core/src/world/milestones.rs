//! 文明里程碑系统
//!
//! 检查和响应里程碑达成，产生世界正反馈。

use crate::world::World;
use crate::world::{Milestone, MilestoneType, resource};
use crate::types::{StructureType, ResourceType, Position};
use crate::agent::RelationType;
use crate::snapshot::NarrativeEvent;

impl World {
    /// 里程碑检查（将在 Task 5.3 完整实现）
    pub fn check_milestones(&mut self) {
        // 简单实现：检测关键里程碑
        let milestones_to_check = [
            (MilestoneType::FirstCamp, self.structures.values().any(|s| s.structure_type == StructureType::Camp)),
            (MilestoneType::FirstTrade, self.total_trades > 0),
            (MilestoneType::FirstFence, self.structures.values().any(|s| s.structure_type == StructureType::Fence)),
            (MilestoneType::FirstAttack, self.total_attacks > 0),
            (MilestoneType::FirstLegacyInteract, self.total_legacy_interacts > 0),
        ];

        for (milestone_type, condition) in &milestones_to_check {
            if *condition {
                let name = format!("{:?}", milestone_type).to_lowercase();
                let display_name = match milestone_type {
                    MilestoneType::FirstCamp => "第一座营地",
                    MilestoneType::FirstTrade => "贸易萌芽",
                    MilestoneType::FirstFence => "领地意识",
                    MilestoneType::FirstAttack => "冲突爆发",
                    MilestoneType::FirstLegacyInteract => "首次传承",
                    MilestoneType::CityState => "城邦雏形",
                    MilestoneType::GoldenAge => "文明黄金期",
                };
                // 检查是否已达成
                let already_achieved = self.milestones.iter().any(|m| m.name == name);
                if !already_achieved {
                    self.milestones.push(Milestone {
                        name: name.clone(),
                        display_name: display_name.to_string(),
                        achieved_tick: self.tick,
                    });
                    tracing::info!("里程碑达成: {} (tick {})", display_name, self.tick);

                    // 世界正反馈：根据里程碑类型产生世界变化
                    self.apply_milestone_feedback(milestone_type);

                    // 添加叙事事件
                    self.tick_events.push(NarrativeEvent {
                        tick: self.tick,
                        agent_id: "system".to_string(),
                        agent_name: "文明".to_string(),
                        event_type: "milestone".to_string(),
                        description: format!("🏆 达成里程碑：【{}】", display_name),
                        color_code: "#FFD700".to_string(),
                    });
                }
            }
        }

        // 城邦雏形：3+ 建筑 + 2+ 盟友对 + 有 Warehouse
        let structure_count = self.structures.len();
        let has_warehouse = self.structures.values().any(|s| s.structure_type == StructureType::Warehouse);
        let ally_count = self.agents.values()
            .flat_map(|a| a.relations.iter())
            .filter(|(_, r)| r.relation_type == RelationType::Ally)
            .count();
        if structure_count >= 3 && ally_count >= 2 && has_warehouse {
            let name = "citystate";
            if !self.milestones.iter().any(|m| m.name == name) {
                self.milestones.push(Milestone {
                    name: name.to_string(),
                    display_name: "城邦雏形".to_string(),
                    achieved_tick: self.tick,
                });
                tracing::info!("里程碑达成: 城邦雏形 (tick {})", self.tick);
                // 添加叙事事件
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "文明".to_string(),
                    event_type: "milestone".to_string(),
                    description: "🏛 达成里程碑：【城邦雏形】".to_string(),
                    color_code: "#FFD700".to_string(),
                });
            }
        }

        // 文明黄金期：前六个全部达成
        if self.milestones.len() >= 6 {
            let name = "goldenage";
            if !self.milestones.iter().any(|m| m.name == name) {
                self.milestones.push(Milestone {
                    name: name.to_string(),
                    display_name: "文明黄金期".to_string(),
                    achieved_tick: self.tick,
                });
                tracing::info!("里程碑达成: 文明黄金期 (tick {})", self.tick);
                // 添加叙事事件
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "文明".to_string(),
                    event_type: "milestone".to_string(),
                    description: "👑 达成里程碑：【文明黄金期】".to_string(),
                    color_code: "#FFD700".to_string(),
                });
            }
        }
    }

    /// 里程碑达成时的世界正反馈
    pub fn apply_milestone_feedback(&mut self, milestone_type: &MilestoneType) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let (map_w, map_h) = self.map.size();

        // 找到最近有动作的 Agent 位置作为反馈中心
        let center_pos = self.agents.values()
            .filter(|a| a.is_alive)
            .next()
            .map(|a| a.position)
            .unwrap_or(Position::new(128, 128));

        match milestone_type {
            MilestoneType::FirstCamp => {
                // 首次建造营地 → 周围生成额外食物和水源（营地带来繁荣）
                for _ in 0..5 {
                    let offset_x = rng.gen_range(-3..=3) as i32;
                    let offset_y = rng.gen_range(-3..=3) as i32;
                    let px = (center_pos.x as i32 + offset_x).clamp(0, map_w as i32 - 1) as u32;
                    let py = (center_pos.y as i32 + offset_y).clamp(0, map_h as i32 - 1) as u32;
                    let pos = Position::new(px, py);
                    let res_type = if rng.gen_bool(0.5) { ResourceType::Food } else { ResourceType::Water };
                    let node = resource::ResourceNode::new(pos, res_type, rng.gen_range(3..=8));
                    self.resources.insert(pos, node);
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: "🌱 营地周围涌现出新的食物和水源！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            MilestoneType::FirstTrade => {
                // 首次交易 → 所有 Agent 获得少量额外资源（贸易繁荣）
                for agent in self.agents.values_mut() {
                    if agent.is_alive {
                        *agent.inventory.entry("food".to_string()).or_default() += 1;
                        *agent.inventory.entry("water".to_string()).or_default() += 1;
                    }
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: " 贸易带来繁荣，所有人获得额外补给！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            MilestoneType::FirstFence => {
                // 首次防御 → 周围生成木材（建设需要材料）
                for _ in 0..5 {
                    let offset_x = rng.gen_range(-3..=3) as i32;
                    let offset_y = rng.gen_range(-3..=3) as i32;
                    let px = (center_pos.x as i32 + offset_x).clamp(0, map_w as i32 - 1) as u32;
                    let py = (center_pos.y as i32 + offset_y).clamp(0, map_h as i32 - 1) as u32;
                    let pos = Position::new(px, py);
                    let node = resource::ResourceNode::new(pos, ResourceType::Wood, rng.gen_range(3..=8));
                    self.resources.insert(pos, node);
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: "🪵 围栏周围发现了新的木材资源！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            MilestoneType::CityState => {
                // 城邦时代 → 大规模资源涌现 + 所有 Agent 恢复 HP
                for agent in self.agents.values_mut() {
                    if agent.is_alive {
                        agent.health = agent.max_health;
                        *agent.inventory.entry("food".to_string()).or_default() += 3;
                        *agent.inventory.entry("water".to_string()).or_default() += 3;
                    }
                }
                for _ in 0..10 {
                    let offset_x = rng.gen_range(-8..=8) as i32;
                    let offset_y = rng.gen_range(-8..=8) as i32;
                    let px = (center_pos.x as i32 + offset_x).clamp(0, map_w as i32 - 1) as u32;
                    let py = (center_pos.y as i32 + offset_y).clamp(0, map_h as i32 - 1) as u32;
                    let pos = Position::new(px, py);
                    let res_types = [ResourceType::Food, ResourceType::Water, ResourceType::Wood, ResourceType::Stone];
                    let res_type = res_types[rng.gen_range(0..res_types.len())];
                    let node = resource::ResourceNode::new(pos, res_type, rng.gen_range(5..=15));
                    self.resources.insert(pos, node);
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: "🏛 城邦崛起！资源涌现，所有人恢复健康！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            MilestoneType::GoldenAge => {
                // 黄金时代 → 所有 Agent 满 HP + 大量资源
                for agent in self.agents.values_mut() {
                    if agent.is_alive {
                        agent.health = agent.max_health;
                        agent.satiety = 100;
                        agent.hydration = 100;
                        *agent.inventory.entry("food".to_string()).or_default() += 5;
                        *agent.inventory.entry("water".to_string()).or_default() += 5;
                        *agent.inventory.entry("wood".to_string()).or_default() += 5;
                        *agent.inventory.entry("stone".to_string()).or_default() += 5;
                    }
                }
                self.tick_events.push(NarrativeEvent {
                    tick: self.tick,
                    agent_id: "system".to_string(),
                    agent_name: "世界".to_string(),
                    event_type: "milestone".to_string(),
                    description: "👑 黄金时代降临！所有人满状态，资源充沛！".to_string(),
                    color_code: "#4CAF50".to_string(),
                });
            }
            _ => {}
        }
    }
}