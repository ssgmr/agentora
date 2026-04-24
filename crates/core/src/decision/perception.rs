//! 感知构建器
//!
//! 从 WorldState 构建感知摘要和路径推荐，用于 Prompt 注入。
//! 从 decision.rs 迁移，实现职责单一化。

use crate::rule_engine::WorldState;
use crate::types::{Direction, Position, ResourceType};
use crate::world::vision::direction_description;
use crate::agent::inventory::get_config;
use std::collections::HashMap;

/// 感知构建器
pub struct PerceptionBuilder;

impl PerceptionBuilder {
    /// 从 WorldState 构建感知摘要
    ///
    /// 包含：推荐路径、生存状态、背包、位置、相邻格、地形、Agent、资源、建筑、遗产
    pub fn build_perception_summary(world_state: &WorldState) -> String {
        let mut summary = String::new();

        // ===== 推荐行动路径（最高优先级展示） =====
        Self::build_path_recommendation(&mut summary, world_state);

        // ===== 生存状态 =====
        let satiety_status = if world_state.agent_satiety <= 30 {
            "⚠️饥饿中！"
        } else if world_state.agent_satiety <= 50 {
            "偏低"
        } else {
            "正常"
        };
        let hydration_status = if world_state.agent_hydration <= 30 {
            "⚠️口渴中！"
        } else if world_state.agent_hydration <= 50 {
            "偏低"
        } else {
            "正常"
        };

        // 生存紧迫提示（更突出）
        let survival_urgent = world_state.agent_satiety <= 30 || world_state.agent_hydration <= 30;
        if survival_urgent {
            summary.push_str("【生存状态】\n");
            if world_state.agent_satiety <= 30 {
                summary.push_str(&format!(
                    "  饱食度: {} [{}] — 需要进食！背包food: {}\n",
                    world_state.agent_satiety, satiety_status,
                    world_state.agent_inventory.get(&ResourceType::Food).copied().unwrap_or(0)
                ));
            }
            if world_state.agent_hydration <= 30 {
                summary.push_str(&format!(
                    "  水分度: {} [{}] — 需要饮水！背包water: {}\n",
                    world_state.agent_hydration, hydration_status,
                    world_state.agent_inventory.get(&ResourceType::Water).copied().unwrap_or(0)
                ));
            }
            summary.push_str("\n");
        } else {
            summary.push_str(&format!(
                "当前状态：饱食度 {}/{} [{}] 水分度 {}/{} [{}]\n",
                world_state.agent_satiety, 100, satiety_status,
                world_state.agent_hydration, 100, hydration_status
            ));
        }

        // 背包信息
        Self::build_inventory_section(&mut summary, world_state);

        // 活跃压力事件
        if !world_state.active_pressures.is_empty() {
            summary.push_str("当前世界事件：\n");
            for pressure_desc in &world_state.active_pressures {
                summary.push_str(&format!("  - {}\n", pressure_desc));
            }
        }

        // 位置信息
        summary.push_str(&format!(
            "位置：({}, {})\n",
            world_state.agent_position.x,
            world_state.agent_position.y
        ));

        // 坐标系说明
        summary.push_str("方向规则：X增大=向东，X减小=向西，Y增大=向南，Y减小=向北。注意：Y轴向下增大，与数学坐标相反！\n");

        // 相邻格信息
        Self::build_adjacent_cells_section(&mut summary, world_state);

        // 地形概览
        Self::build_terrain_overview(&mut summary, world_state);

        // 附近 Agent
        Self::build_nearby_agents_section(&mut summary, world_state);

        // 资源信息
        Self::build_resources_section(&mut summary, world_state);

        // 附近建筑
        Self::build_structures_section(&mut summary, world_state);

        // 附近遗产
        Self::build_legacies_section(&mut summary, world_state);

        // 待处理交易
        Self::build_pending_trades_section(&mut summary, world_state);

        // 待处理结盟请求
        Self::build_pending_ally_requests_section(&mut summary, world_state);

        // 宏观区域上下文
        Self::build_region_context(&mut summary, world_state);

        summary
    }

    /// 构建背包部分
    fn build_inventory_section(summary: &mut String, world_state: &WorldState) {
        if !world_state.agent_inventory.is_empty() {
            let total: u32 = world_state.agent_inventory.values().sum();
            let effective_limit = get_config().max_stack_size;
            let full_items: Vec<&ResourceType> = world_state.agent_inventory.iter()
                .filter(|(_, count)| **count >= effective_limit as u32)
                .map(|(r, _)| r)
                .collect();
            let limit_note = if full_items.is_empty() {
                format!("每种资源上限{}，仓库附近可达{}", effective_limit, effective_limit * 2)
            } else {
                let names: Vec<&str> = full_items.iter().map(|r| r.as_str()).collect();
                format!(
                    "每种资源上限{}，仓库附近可达{}，{} 已满（堆叠达上限）",
                    effective_limit, effective_limit * 2, names.join("、")
                )
            };
            summary.push_str(&format!("背包（{}，当前合计{}）：", limit_note, total));
            let items: Vec<String> = world_state.agent_inventory.iter()
                .map(|(r, count)| format!("{} x{}", r.as_str(), count))
                .collect();
            summary.push_str(&items.join(", "));
            summary.push('\n');
        } else {
            let effective_limit = get_config().max_stack_size;
            summary.push_str(&format!(
                "背包（每种资源上限{}，仓库附近可达{}，当前空）：\n",
                effective_limit, effective_limit * 2
            ));
        }
    }

    /// 构建相邻格部分
    fn build_adjacent_cells_section(summary: &mut String, world_state: &WorldState) {
        let pos = world_state.agent_position;
        let dirs = [
            (Direction::North, "北", "north", 0i32, -1i32),
            (Direction::South, "南", "south", 0, 1),
            (Direction::East, "东", "east", 1, 0),
            (Direction::West, "西", "west", -1, 0),
        ];
        summary.push_str("相邻格（可移动方向，每格需1步）：\n");
        for (_dir, name, eng, dx, dy) in &dirs {
            let nx = pos.x as i32 + dx;
            let ny = pos.y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= world_state.map_size as i32 || ny >= world_state.map_size as i32 {
                summary.push_str(&format!("  {}: 越界(不可移动)\n", name));
            } else {
                let np = Position::new(nx as u32, ny as u32);
                let terrain = world_state.terrain_at.get(&np);
                let terrain_icon = terrain.map(|t| format!("{:?}", t)).unwrap_or_else(|| "未知".to_string());

                let res_mark = world_state.resources_at.get(&np)
                    .map(|(r, a)| format!("{:?}×{}", r, a))
                    .unwrap_or_default();

                let agent_mark = world_state.nearby_agents.iter()
                    .find(|a| a.position == np)
                    .map(|a| format!(" Agent:{}", a.name))
                    .unwrap_or_default();

                summary.push_str(&format!(
                    "  {}({},{}) {} {} {} → direction:\"{}\"\n",
                    name, nx, ny, terrain_icon, res_mark, agent_mark, eng
                ));
            }
        }
    }

    /// 构建地形概览
    fn build_terrain_overview(summary: &mut String, world_state: &WorldState) {
        if !world_state.terrain_at.is_empty() {
            let mut terrain_counts: HashMap<String, u32> = HashMap::new();
            for terrain in world_state.terrain_at.values() {
                *terrain_counts.entry(format!("{:?}", terrain)).or_default() += 1;
            }
            if !terrain_counts.is_empty() {
                summary.push_str("地形：");
                let parts: Vec<String> = terrain_counts.iter()
                    .map(|(t, c)| format!("{} {}格", t, c))
                    .collect();
                summary.push_str(&parts.join(", "));
                summary.push('\n');
            }
        }
    }

    /// 构建附近 Agent 部分
    fn build_nearby_agents_section(summary: &mut String, world_state: &WorldState) {
        use crate::agent::RelationType;

        if !world_state.nearby_agents.is_empty() {
            summary.push_str(&format!("附近 Agent ({} 个):\n", world_state.nearby_agents.len()));
            for agent_info in &world_state.nearby_agents {
                let relation_str = match agent_info.relation_type {
                    RelationType::Ally => "盟友",
                    RelationType::Enemy => "敌人",
                    RelationType::Neutral => "陌生人",
                };
                let dir_desc = direction_description(&world_state.agent_position, &agent_info.position);
                summary.push_str(&format!(
                    "  {} ({},{}) [{}] 距离:{}格 关系:{} 信任:{:.1}\n",
                    agent_info.name,
                    agent_info.position.x,
                    agent_info.position.y,
                    dir_desc,
                    agent_info.distance,
                    relation_str,
                    agent_info.trust,
                ));
            }
        } else {
            summary.push_str("附近无其他 Agent（只有你自己）\n");
        }
    }

    /// 构建资源信息部分
    fn build_resources_section(summary: &mut String, world_state: &WorldState) {
        if !world_state.resources_at.is_empty() {
            summary.push_str("资源分布:\n");

            let mut resources: Vec<_> = world_state.resources_at.iter().collect();
            let agent_pos = &world_state.agent_position;
            let satiety = world_state.agent_satiety;
            let hydration = world_state.agent_hydration;

            resources.sort_by(|(pos_a, (res_a, _)), (pos_b, (res_b, _))| {
                let dist_a = pos_a.manhattan_distance(agent_pos);
                let dist_b = pos_b.manhattan_distance(agent_pos);

                fn resource_priority(r: &ResourceType, satiety: u32, hydration: u32) -> u32 {
                    match r {
                        ResourceType::Food if satiety <= 50 => 0,
                        ResourceType::Water if hydration <= 50 => 0,
                        ResourceType::Food => 1,
                        ResourceType::Water => 2,
                        ResourceType::Wood => 3,
                        ResourceType::Stone => 4,
                        ResourceType::Iron => 5,
                    }
                }

                let priority_a = resource_priority(res_a, satiety, hydration);
                let priority_b = resource_priority(res_b, satiety, hydration);

                match priority_a.cmp(&priority_b) {
                    std::cmp::Ordering::Equal => dist_a.cmp(&dist_b),
                    other => other,
                }
            });

            for (pos, (resource, amount)) in resources {
                let dir_desc = direction_description(agent_pos, pos);
                let abundance = if *amount >= 100 {
                    "(大量)"
                } else if *amount >= 50 {
                    "(中等)"
                } else {
                    "(少量)"
                };

                summary.push_str(&format!(
                    "  ({}, {}): {:?} x{} {} [{}]\n",
                    pos.x, pos.y, resource, amount, abundance, dir_desc
                ));
            }
        }
    }

    /// 构建附近建筑部分
    fn build_structures_section(summary: &mut String, world_state: &WorldState) {
        if !world_state.nearby_structures.is_empty() {
            summary.push_str(&format!("附近建筑 ({} 个):\n", world_state.nearby_structures.len()));
            for structure in &world_state.nearby_structures {
                let owner_str = structure.owner_name.as_deref().unwrap_or("无主");
                let dur_status = if structure.durability > 70 {
                    "完好"
                } else if structure.durability > 30 {
                    "受损"
                } else {
                    "破败"
                };
                let dir_desc = direction_description(&world_state.agent_position, &structure.position);
                summary.push_str(&format!(
                    "  ({}, {}): {:?} [{}] ({}: {}, 耐久{})\n",
                    structure.position.x, structure.position.y,
                    structure.structure_type, dir_desc, owner_str, dur_status, structure.distance
                ));
            }
        }
    }

    /// 构建附近遗产部分
    fn build_legacies_section(summary: &mut String, world_state: &WorldState) {
        if !world_state.nearby_legacies.is_empty() {
            summary.push_str(&format!("附近遗迹 ({} 个):\n", world_state.nearby_legacies.len()));
            for legacy in &world_state.nearby_legacies {
                let items_hint = if legacy.has_items { "有物品" } else { "空" };
                let dir_desc = direction_description(&world_state.agent_position, &legacy.position);
                summary.push_str(&format!(
                    "  ({}, {}): {:?} [{}] ({}的遗迹, {})\n",
                    legacy.position.x, legacy.position.y,
                    legacy.legacy_type, dir_desc, legacy.original_agent_name, items_hint
                ));
            }
        }
    }

    /// 构建待处理交易部分
    fn build_pending_trades_section(summary: &mut String, world_state: &WorldState) {
        if !world_state.pending_trades.is_empty() {
            summary.push_str(&format!("待处理交易 ({} 个)：\n", world_state.pending_trades.len()));
            summary.push_str("  【提示】以下是其他Agent向你发起的交易提议。你可以：\n");
            summary.push_str("    - 用 TradeAccept 接受交易（交换双方资源）\n");
            summary.push_str("    - 用 TradeReject 拒绝交易（取消提议）\n");
            for trade in &world_state.pending_trades {
                let offer_str: Vec<String> = trade.offer.iter()
                    .map(|(r, n)| format!("{} x{}", r.as_str(), n))
                    .collect();
                let want_str: Vec<String> = trade.want.iter()
                    .map(|(r, n)| format!("{} x{}", r.as_str(), n))
                    .collect();
                let offer_display = if offer_str.is_empty() { "无".to_string() } else { offer_str.join(" + ") };
                let want_display = if want_str.is_empty() { "无".to_string() } else { want_str.join(" + ") };
                summary.push_str(&format!(
                    "  [trade_id:{}] {} [ID:{}] 提议：用 {} 换你的 {}\n",
                    trade.trade_id,
                    trade.proposer_name,
                    trade.proposer_id.as_str(),
                    offer_display,
                    want_display,
                ));
            }
            summary.push('\n');
        }
    }

    /// 构建待处理结盟请求部分
    fn build_pending_ally_requests_section(summary: &mut String, world_state: &WorldState) {
        if !world_state.pending_ally_requests.is_empty() {
            summary.push_str(&format!("待处理结盟请求 ({} 个)：\n", world_state.pending_ally_requests.len()));
            summary.push_str("  【提示】以下是其他Agent向你发起的结盟请求。你可以：\n");
            summary.push_str("    - 用 AllyAccept 接受结盟（成为盟友，不能互相攻击）\n");
            summary.push_str("    - 用 AllyReject 拒绝结盟（取消请求）\n");
            for request in &world_state.pending_ally_requests {
                summary.push_str(&format!(
                    "  [ally_id:{}] {} 请求与你结盟（使用 AllyAccept ally_id:{} 接受）\n",
                    request.ally_id.as_str(),
                    request.proposer_name,
                    request.ally_id.as_str(),
                ));
            }
            summary.push('\n');
        }
    }

    /// 构建宏观区域上下文
    fn build_region_context(summary: &mut String, world_state: &WorldState) {
        let region_x = world_state.agent_position.x / 16;
        let region_y = world_state.agent_position.y / 16;
        let region_id = region_y * 16 + region_x;

        if !world_state.terrain_at.is_empty() {
            let mut terrain_counts: HashMap<String, u32> = HashMap::new();
            for terrain in world_state.terrain_at.values() {
                *terrain_counts.entry(format!("{:?}", terrain)).or_default() += 1;
            }
            if let Some((dominant, count)) = terrain_counts.iter().max_by_key(|(_, c)| **c) {
                let total: u32 = terrain_counts.values().sum();
                summary.push_str(&format!(
                    "区域：区域{} ({}-{})，主导地形{} ({:.0}%)\n",
                    region_id, region_x, region_y, dominant,
                    (*count as f32 / total as f32) * 100.0
                ));
            }
        }
    }

    /// 构建路径推荐
    ///
    /// 根据生存状态和资源分布，推荐最优移动路径
    fn build_path_recommendation(summary: &mut String, world_state: &WorldState) {
        // 确定优先资源类型（基于生存压力）
        let priority_resource = if world_state.agent_satiety <= 50 {
            Some(ResourceType::Food)
        } else if world_state.agent_hydration <= 50 {
            Some(ResourceType::Water)
        } else {
            None
        };

        if let Some(priority) = priority_resource {
            Self::build_priority_path(summary, world_state, priority);
        } else {
            Self::build_exploration_path(summary, world_state);
        }
    }

    /// 构建优先资源路径（有生存压力时）
    fn build_priority_path(summary: &mut String, world_state: &WorldState, priority: ResourceType) {
        let nearest = world_state.resources_at.iter()
            .filter(|(_, (r, _))| *r == priority)
            .min_by_key(|(pos, _)| pos.manhattan_distance(&world_state.agent_position));

        if let Some((pos, (_, amount))) = nearest {
            let dist = pos.manhattan_distance(&world_state.agent_position);
            let dx = pos.x as i32 - world_state.agent_position.x as i32;
            let dy = pos.y as i32 - world_state.agent_position.y as i32;

            let (first_dir, dir_name, dir_eng) = if dx.abs() >= dy.abs() {
                if dx > 0 { (Direction::East, "东", "east") }
                else { (Direction::West, "西", "west") }
            } else {
                if dy > 0 { (Direction::South, "南", "south") }
                else { (Direction::North, "北", "north") }
            };

            let delta = first_dir.delta();
            let step_x = world_state.agent_position.x as i32 + delta.0;
            let step_y = world_state.agent_position.y as i32 + delta.1;
            let step_valid = step_x >= 0 && step_y >= 0 &&
                step_x < world_state.map_size as i32 &&
                step_y < world_state.map_size as i32;

            summary.push_str("【推荐路径】\n");
            summary.push_str(&format!(
                "  最近的{:?}在{}方向({},{})，距离{}格，存量×{}\n",
                priority, dir_name, pos.x, pos.y, dist, amount
            ));

            if step_valid && dist > 0 {
                summary.push_str(&format!(
                    "  → 建议动作：MoveToward，direction: \"{}\"（向{}移动1格）\n",
                    dir_eng, dir_name
                ));
                if dist > 1 {
                    summary.push_str(&format!(
                        "  → 还需{}步到达，建议持续向{}方向移动\n",
                        dist - 1, dir_name
                    ));
                }
            } else if !step_valid && dist > 0 {
                let alternatives = [
                    (Direction::North, "北", "north"),
                    (Direction::South, "南", "south"),
                    (Direction::East, "东", "east"),
                    (Direction::West, "西", "west"),
                ];
                for (alt_dir, _alt_name, alt_eng) in &alternatives {
                    let alt_delta = alt_dir.delta();
                    let alt_x = world_state.agent_position.x as i32 + alt_delta.0;
                    let alt_y = world_state.agent_position.y as i32 + alt_delta.1;
                    if alt_x >= 0 && alt_y >= 0 &&
                        alt_x < world_state.map_size as i32 &&
                        alt_y < world_state.map_size as i32 {
                        summary.push_str(&format!(
                            "  → {}方向被阻挡，建议绕行：direction: \"{}\"\n",
                            dir_name, alt_eng
                        ));
                        break;
                    }
                }
            }

            let have_in_bag = world_state.agent_inventory.get(&priority).copied().unwrap_or(0);
            if have_in_bag > 0 {
                let urgent = world_state.agent_satiety <= 30 || world_state.agent_hydration <= 30;
                if urgent {
                    summary.push_str(&format!(
                        "  → 或者：背包有{}×{}，可直接{}恢复（优先级更高）\n",
                        priority.as_str(), have_in_bag,
                        if priority == ResourceType::Food { "Eat" } else { "Drink" }
                    ));
                }
            }
            summary.push_str("\n");
        } else {
            Self::build_no_priority_resource_path(summary, world_state, priority);
        }
    }

    /// 视野内无优先资源时的路径推荐
    fn build_no_priority_resource_path(summary: &mut String, world_state: &WorldState, priority: ResourceType) {
        summary.push_str("【推荐路径】\n");
        summary.push_str(&format!(
            "  视野内无{:?}资源，建议向任意有效方向探索\n",
            priority
        ));

        let directions = [
            (Direction::North, "北", "north"),
            (Direction::South, "南", "south"),
            (Direction::East, "东", "east"),
            (Direction::West, "西", "west"),
        ];
        for (dir, name, eng) in &directions {
            let delta = dir.delta();
            let nx = world_state.agent_position.x as i32 + delta.0;
            let ny = world_state.agent_position.y as i32 + delta.1;
            if nx >= 0 && ny >= 0 && nx < world_state.map_size as i32 && ny < world_state.map_size as i32 {
                summary.push_str(&format!(
                    "  → 建议：direction: \"{}\"（向{}探索）\n\n",
                    eng, name
                ));
                break;
            }
        }
    }

    /// 无生存压力时的探索路径
    fn build_exploration_path(summary: &mut String, world_state: &WorldState) {
        let current_has_resource = world_state.resources_at.get(&world_state.agent_position);
        if current_has_resource.is_some() {
            let (r, amount) = current_has_resource.unwrap();
            summary.push_str("【推荐路径】\n");
            summary.push_str(&format!(
                "  当前位置有{:?}×{}，可直接Gather采集\n\n",
                r, amount
            ));
        } else {
            let nearest_any = world_state.resources_at.iter()
                .min_by_key(|(pos, _)| pos.manhattan_distance(&world_state.agent_position));

            if let Some((pos, (r, amount))) = nearest_any {
                let dist = pos.manhattan_distance(&world_state.agent_position);
                let dx = pos.x as i32 - world_state.agent_position.x as i32;
                let dy = pos.y as i32 - world_state.agent_position.y as i32;
                let (dir_name, dir_eng) = if dx.abs() >= dy.abs() {
                    (if dx > 0 { "东" } else { "西" }, if dx > 0 { "east" } else { "west" })
                } else {
                    (if dy > 0 { "南" } else { "北" }, if dy > 0 { "south" } else { "north" })
                };
                summary.push_str("【推荐路径】\n");
                summary.push_str(&format!(
                    "  最近的{:?}×{}在{}方向({},{})，距离{}格\n",
                    r, amount, dir_name, pos.x, pos.y, dist
                ));
                if dist <= 5 {
                    summary.push_str(&format!(
                        "  → 建议：direction: \"{}\"（向{}移动）\n\n",
                        dir_eng, dir_name
                    ));
                } else {
                    summary.push_str("\n");
                }
            }
        }
    }
}