//! 动作反馈生成
//!
//! 从 ActionResult 解析生成人类可读的反馈字符串。

use crate::world::World;
use crate::world::ActionResult;
use crate::types::ActionType;
use crate::types::Position;

impl World {
    /// 统一生成动作反馈（从 ActionResult 提取信息）
    pub fn generate_action_feedback(&self, result: &ActionResult, action_type: &ActionType, old_position: Option<Position>) -> String {
        match result {
            ActionResult::SuccessWithDetail(detail) => {
                // 解析 detail 格式，生成人类可读的反馈
                self.parse_success_detail(detail, action_type, old_position)
            }
            ActionResult::AlreadyAtPosition(msg) => msg.clone(),
            ActionResult::Blocked(reason) => {
                format!("{} 失败：{}", self.action_type_name(action_type), reason)
            }
            ActionResult::OutOfBounds => format!("{} 失败：超出地图边界", self.action_type_name(action_type)),
            ActionResult::AgentDead => format!("{} 失败：Agent 已死亡", self.action_type_name(action_type)),
            ActionResult::InvalidAgent => format!("{} 失败：Agent 不存在", self.action_type_name(action_type)),
            ActionResult::NotImplemented => format!("{} 失败：未实现", self.action_type_name(action_type)),
        }
    }

    /// 将动作类型转换为简短描述（用于 UI 显示）
    pub fn action_type_to_short_desc(action_type: &ActionType) -> String {
        use crate::types::StructureType;
        match action_type {
            ActionType::MoveToward { target } => {
                format!("移动→({},{})", target.x, target.y)
            }
            ActionType::Gather { resource } => format!("采集 {}", resource.as_str()),
            ActionType::Eat => "进食".to_string(),
            ActionType::Drink => "饮水".to_string(),
            ActionType::Build { structure } => {
                let struct_name = match structure {
                    StructureType::Camp => "营地",
                    StructureType::Fence => "围栏",
                    StructureType::Warehouse => "仓库",
                };
                format!("建造 {}", struct_name)
            }
            ActionType::Attack { target_id } => format!("攻击 {}", target_id.as_str()),
            ActionType::Talk { .. } => "对话".to_string(),
            ActionType::Explore { .. } => "探索".to_string(),
            ActionType::TradeOffer { .. } => "交易".to_string(),
            ActionType::TradeAccept { .. } => "接受交易".to_string(),
            ActionType::TradeReject { .. } => "拒绝交易".to_string(),
            ActionType::AllyPropose { .. } => "结盟".to_string(),
            ActionType::AllyAccept { .. } => "接受结盟".to_string(),
            ActionType::AllyReject { .. } => "拒绝结盟".to_string(),
            ActionType::InteractLegacy { .. } => "互动遗产".to_string(),
            ActionType::Wait => "等待".to_string(),
        }
    }

    /// 解析成功详情，生成人类可读反馈
    pub fn parse_success_detail(&self, detail: &str, action_type: &ActionType, _old_position: Option<Position>) -> String {
        // detail 格式: "动作类型:具体数据"
        // 如 "move:121,113→(131,142)" 或 "gather:waterx2,remain:184"

        if detail.starts_with("move:") {
            // 格式: move:old_x,old_y→(new_x,new_y)
            let parts = detail.strip_prefix("move:").unwrap_or("");
            if let Some((old, new)) = parts.split_once("→") {
                let old_coords: Vec<&str> = old.split(',').collect();
                let new_coords: Vec<&str> = new.trim_matches(|c| c == '(' || c == ')').split(',').collect();
                if old_coords.len() == 2 && new_coords.len() == 2 {
                    if let (Ok(ox), Ok(oy), Ok(nx), Ok(ny)) = (
                        old_coords[0].parse::<i32>(),
                        old_coords[1].parse::<i32>(),
                        new_coords[0].parse::<i32>(),
                        new_coords[1].parse::<i32>(),
                    ) {
                        // 直接计算方向名称
                        let dx = nx - ox;
                        let dy = ny - oy;
                        let dir_name = match (dx.cmp(&0), dy.cmp(&0)) {
                            (std::cmp::Ordering::Greater, std::cmp::Ordering::Less) => "东北",
                            (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => "东南",
                            (std::cmp::Ordering::Greater, std::cmp::Ordering::Equal) => "东",
                            (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => "西北",
                            (std::cmp::Ordering::Less, std::cmp::Ordering::Greater) => "西南",
                            (std::cmp::Ordering::Less, std::cmp::Ordering::Equal) => "西",
                            (std::cmp::Ordering::Equal, std::cmp::Ordering::Greater) => "南",
                            (std::cmp::Ordering::Equal, std::cmp::Ordering::Less) => "北",
                            (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal) => "原地",
                        };
                        return format!("向{}移动到 ({}, {})", dir_name, nx, ny);
                    }
                }
            }
            return format!("移动成功");
        }

        if detail.starts_with("gather:") {
            // 新格式: gather:resource x amount,node_remain: count,inv: old→new
            let parts = detail.strip_prefix("gather:").unwrap_or("");

            // 尝试解析新格式
            if parts.contains(",node_remain:") && parts.contains(",inv:") {
                // 格式: resource x amount,node_remain: count,inv: old→new
                if let Some((gather_part, rest)) = parts.split_once(",node_remain:") {
                    let resource_amount: Vec<&str> = gather_part.split('x').collect();
                    if resource_amount.len() == 2 {
                        let resource = resource_amount[0].trim();
                        if let Ok(amount) = resource_amount[1].trim().parse::<u32>() {
                            if let Some((remain_part, inv_part)) = rest.split_once(",inv:") {
                                if let Ok(node_remain) = remain_part.trim().parse::<u32>() {
                                    let inv_parts: Vec<&str> = inv_part.trim().split('→').collect();
                                    if inv_parts.len() == 2 {
                                        if let (Ok(old_inv), Ok(new_inv)) =
                                            (inv_parts[0].parse::<u32>(), inv_parts[1].parse::<u32>()) {
                                            return format!(
                                                "Gather成功：获得 {} x{}。当前位置 {} 资源剩余 x{}。背包 {} 从 x{} 增至 x{}",
                                                resource, amount, resource, node_remain, resource, old_inv, new_inv);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 旧格式兼容: gather:resource x amount,remain: count
            if let Some((gather_part, remain_part)) = parts.split_once(",remain:") {
                let resource_amount: Vec<&str> = gather_part.split('x').collect();
                if resource_amount.len() == 2 {
                    let resource = resource_amount[0];
                    if let Ok(amount) = resource_amount[1].parse::<u32>() {
                        if let Ok(remain) = remain_part.parse::<u32>() {
                            return format!("采集了 {} 个 {}，剩余 {}", amount, resource, remain);
                        }
                    }
                }
            }
            return format!("采集成功");
        }

        if detail.starts_with("eat:") {
            // 新格式: eat:satiety+gain(before→after),food_remain=count
            let parts = detail.strip_prefix("eat:").unwrap_or("");

            if parts.contains("satiety+") && parts.contains(",food_remain=") {
                if let Some((satiety_part, remain_part)) = parts.split_once(",food_remain=") {
                    // 解析 satiety+gain(before→after)
                    if let Some(gain_part) = satiety_part.strip_prefix("satiety+") {
                        let satiety_parts: Vec<&str> = gain_part.split('→').collect();
                        if satiety_parts.len() == 2 {
                            if let (Ok(before), Ok(after)) = (satiety_parts[0].parse::<u32>(), satiety_parts[1].parse::<u32>()) {
                                if let Ok(food_remain) = remain_part.trim().parse::<u32>() {
                                    return format!(
                                        "Eat成功：消耗 food x1，饱食度+{}（从{}增至{}）。背包 food 剩余 x{}",
                                        after - before, before, after, food_remain);
                                }
                            }
                        }
                    }
                }
            }

            // 旧格式兼容: eat:satiety=XX/100
            if let Some(satiety_str) = parts.strip_prefix("satiety=") {
                return format!("进食成功，饱食度恢复至 {}", satiety_str);
            }
            return format!("进食成功");
        }

        if detail.starts_with("drink:") {
            // 新格式: drink:hydration+gain(before→after),water_remain=count
            let parts = detail.strip_prefix("drink:").unwrap_or("");

            if parts.contains("hydration+") && parts.contains(",water_remain=") {
                if let Some((hydration_part, remain_part)) = parts.split_once(",water_remain=") {
                    // 解析 hydration+gain(before→after)
                    if let Some(gain_part) = hydration_part.strip_prefix("hydration+") {
                        let hydration_parts: Vec<&str> = gain_part.split('→').collect();
                        if hydration_parts.len() == 2 {
                            if let (Ok(before), Ok(after)) = (hydration_parts[0].parse::<u32>(), hydration_parts[1].parse::<u32>()) {
                                if let Ok(water_remain) = remain_part.trim().parse::<u32>() {
                                    return format!(
                                        "Drink成功：消耗 water x1，水分度+{}（从{}增至{}）。背包 water 剩余 x{}",
                                        after - before, before, after, water_remain);
                                }
                            }
                        }
                    }
                }
            }

            // 旧格式兼容: drink:hydration=XX/100
            if let Some(hydration_str) = parts.strip_prefix("hydration=") {
                return format!("饮水成功，水分度恢复至 {}", hydration_str);
            }
            return format!("饮水成功");
        }

        if detail.starts_with("build:") {
            // 格式: build:StructureTypeat(x,y)
            let parts = detail.strip_prefix("build:").unwrap_or("");
            if let Some((struct_part, pos_part)) = parts.split_once("at") {
                let coords = pos_part.trim_matches(|c| c == '(' || c == ')');
                return format!("在 {} 建造了 {}", coords, struct_part);
            }
            return format!("建造成功");
        }

        if detail.starts_with("attack:") {
            // 格式: attack:target_namehit,damage=10 或 attack:target_namedefeated,damage=10
            let parts = detail.strip_prefix("attack:").unwrap_or("");
            if let Some((name_part, outcome)) = parts.split_once(",") {
                if outcome.starts_with("defeated") {
                    return format!("攻击 {} 并将其击败", name_part);
                } else if outcome.starts_with("hit") {
                    return format!("攻击 {}，造成 10 点伤害", name_part);
                }
            }
            return format!("攻击成功");
        }

        if detail.starts_with("explore:") {
            // 格式: explore:Nsteps,old_x,old_y→(new_x,new_y)
            let parts = detail.strip_prefix("explore:").unwrap_or("");
            if let Some((steps_part, _)) = parts.split_once("steps") {
                if let Ok(steps) = steps_part.parse::<u32>() {
                    return format!("探索了 {} 步", steps);
                }
            }
            return format!("探索成功");
        }

        if detail.starts_with("talk:") {
            let parts = detail.strip_prefix("talk:").unwrap_or("");
            if parts == "self" {
                return format!("自言自语");
            }
            return format!("与 {} 交流", parts);
        }

        if detail.starts_with("trade_offer:") {
            let target = detail.strip_prefix("trade_offer:").unwrap_or("");
            return format!("向 {} 发起交易请求", target);
        }

        if detail.starts_with("trade_accept:") {
            let parts = detail.strip_prefix("trade_accept:").unwrap_or("");
            return format!("与 {} 完成交易", parts.replace(" ↔ ", " 和 "));
        }

        if detail.starts_with("trade_reject:") {
            let proposer = detail.strip_prefix("trade_reject:").unwrap_or("");
            return format!("拒绝了 {} 的交易请求", proposer);
        }

        if detail.starts_with("ally_propose:") {
            let target = detail.strip_prefix("ally_propose:").unwrap_or("");
            return format!("向 {} 提议结盟", target);
        }

        if detail.starts_with("ally_accept:") {
            let parts = detail.strip_prefix("ally_accept:").unwrap_or("");
            return format!("与 {} 结成联盟", parts.replace(" ↔ ", " 和 "));
        }

        if detail.starts_with("ally_reject:") {
            let proposer = detail.strip_prefix("ally_reject:").unwrap_or("");
            return format!("拒绝了 {} 的结盟请求", proposer);
        }

        if detail.starts_with("legacy:") {
            let parts = detail.strip_prefix("legacy:").unwrap_or("");
            if parts == "worship" {
                return format!("祭拜遗产");
            }
            if parts == "explore" {
                return format!("探索遗产");
            }
            if parts.starts_with("pickup") {
                return format!("拾取了 {}", parts.strip_prefix("pickup ").unwrap_or("物品"));
            }
            return format!("遗产交互成功");
        }

        if detail == "wait" {
            return format!("等待了一回合");
        }

        // 兜底
        format!("{} 执行成功", self.action_type_name(action_type))
    }
}