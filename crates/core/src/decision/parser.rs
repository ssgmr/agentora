//! JSON 解析逻辑
//!
//! 将 LLM 返回的 JSON 转换为 ActionCandidate

use crate::types::{ActionType, AgentId, Position, ResourceType, StructureType, Direction};
use std::collections::HashMap;
use serde_json::Value;

/// 将 JSON 值转换为 ActionCandidate
pub fn json_to_candidate(
    json: Value,
    agent_pos: Position,
    reasoning: String,
) -> Result<super::ActionCandidate, String> {
    let action_type_str = json["action_type"]
        .as_str()
        .ok_or("缺少 action_type 字段")?;

    // 解析 action_type
    let action_type = parse_action_type(action_type_str, &json, agent_pos)
        .ok_or_else(|| format_action_error(action_type_str, &json))?;

    let target = json["target"].as_str().map(String::from);

    let params = json["params"]
        .as_object()
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();

    Ok(super::ActionCandidate {
        reasoning,
        action_type,
        target,
        params,
    })
}

/// 解析动作类型
pub fn parse_action_type(type_str: &str, json: &Value, agent_pos: Position) -> Option<ActionType> {
    match type_str {
        "Move" | "move" | "移动" => parse_move(json, agent_pos),
        "MoveToward" | "move_toward" | "移动到" | "前往" => {
            parse_target_position(json, agent_pos).map(|target| ActionType::MoveToward { target })
        }
        "Gather" | "gather" | "采集" | "收集" => parse_gather(json),
        "Wait" | "wait" | "等待" => Some(ActionType::Wait),
        "Eat" | "eat" | "进食" | "吃东西" => Some(ActionType::Eat),
        "Drink" | "drink" | "饮水" | "喝水" => Some(ActionType::Drink),
        "Talk" | "talk" | "对话" | "交流" => parse_talk(json),
        "Build" | "build" | "建造" => parse_build(json),
        "Attack" | "attack" | "攻击" => parse_attack(json),
        "TradeOffer" | "trade" | "交易" | "交易提议" => parse_trade_offer(json),
        "TradeAccept" | "交易接受" => parse_trade_accept(json),
        "TradeReject" | "交易拒绝" => parse_trade_reject(json),
        "AllyPropose" | "ally" | "结盟" | "结盟提议" => parse_ally_propose(json),
        "AllyAccept" | "结盟接受" => parse_ally_accept(json),
        "AllyReject" | "结盟拒绝" => parse_ally_reject(json),
        _ => {
            tracing::warn!("未知 action_type: {}, 使用 Wait 兜底", type_str);
            Some(ActionType::Wait)
        }
    }
}

/// 解析 Move 或 MoveToward 动作
fn parse_move(json: &Value, agent_pos: Position) -> Option<ActionType> {
    // 优先尝试方向格式
    if let Some(dir_str) = json["params"]["direction"].as_str() {
        let direction = match dir_str {
            "North" | "north" | "北" | "n" | "N" => Direction::North,
            "South" | "south" | "南" | "s" | "S" => Direction::South,
            "East" | "east" | "东" | "e" | "E" => Direction::East,
            "West" | "west" | "西" | "w" | "W" => Direction::West,
            _ => return None,
        };
        let target = match direction {
            Direction::North => Position::new(agent_pos.x, agent_pos.y.wrapping_sub(1)),
            Direction::South => Position::new(agent_pos.x, agent_pos.y + 1),
            Direction::East => Position::new(agent_pos.x + 1, agent_pos.y),
            Direction::West => Position::new(agent_pos.x.wrapping_sub(1), agent_pos.y),
        };
        Some(ActionType::MoveToward { target })
    } else if let Some(target) = parse_target_position(json, agent_pos) {
        Some(ActionType::MoveToward { target })
    } else {
        None
    }
}

/// 解析 MoveToward 目标位置
///
/// 支持多种格式：
/// - { x: 130, y: 125 }
/// - [130, 125]
/// - "130,125" 或 "(130, 125)"
/// - direction: "north"/"south"/"east"/"west"
pub fn parse_target_position(json: &Value, agent_pos: Position) -> Option<Position> {
    // 优先从 direction 字段解析
    if let Some(dir_str) = json["params"]["direction"].as_str()
        .or_else(|| json["direction"].as_str())
    {
        let direction = match dir_str.trim() {
            "North" | "north" | "北" | "n" | "N" => Some(Direction::North),
            "South" | "south" | "南" | "s" | "S" => Some(Direction::South),
            "East" | "east" | "东" | "e" | "E" => Some(Direction::East),
            "West" | "west" | "西" | "w" | "W" => Some(Direction::West),
            _ => None,
        };
        if let Some(dir) = direction {
            let target = match dir {
                Direction::North => Position::new(agent_pos.x, agent_pos.y.wrapping_sub(1)),
                Direction::South => Position::new(agent_pos.x, agent_pos.y + 1),
                Direction::East => Position::new(agent_pos.x + 1, agent_pos.y),
                Direction::West => Position::new(agent_pos.x.wrapping_sub(1), agent_pos.y),
            };
            return Some(target);
        }
    }

    // 尝试从 params.target 或顶层 target 获取坐标
    let target = json["params"]["target"]
        .as_object()
        .map(|_| &json["params"]["target"])
        .or_else(|| json["target"].as_object().map(|_| &json["target"]));

    if let Some(target_obj) = target {
        // 格式1: { x: 130, y: 125 }
        if let (Some(x), Some(y)) = (target_obj.get("x"), target_obj.get("y")) {
            if let (Some(x), Some(y)) = (x.as_u64(), y.as_u64()) {
                let pos = Position::new(x as u32, y as u32);
                if pos.manhattan_distance(&agent_pos) == 1 {
                    return Some(pos);
                }
                tracing::warn!("MoveToward 目标 ({},{}) 不相邻（距离 {}）", pos.x, pos.y, pos.manhattan_distance(&agent_pos));
                return None;
            }
        }

        // 格式2: [130, 125]
        if let Some(arr) = target_obj.as_array() {
            if arr.len() >= 2 {
                if let (Some(x), Some(y)) = (arr[0].as_u64(), arr[1].as_u64()) {
                    let pos = Position::new(x as u32, y as u32);
                    if pos.manhattan_distance(&agent_pos) == 1 {
                        return Some(pos);
                    }
                    tracing::warn!("MoveToward 目标 [{},{}] 不相邻", pos.x, pos.y);
                    return None;
                }
            }
        }

        // 格式3: "130,125" 或 "(130, 125)"
        if let Some(s) = target_obj.as_str() {
            let cleaned = s.trim_matches(|c| c == '(' || c == ')');
            let parts: Vec<&str> = cleaned.split(',').collect();
            if parts.len() >= 2 {
                if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<u32>(), parts[1].trim().parse::<u32>()) {
                    let pos = Position::new(x, y);
                    if pos.manhattan_distance(&agent_pos) == 1 {
                        return Some(pos);
                    }
                    tracing::warn!("MoveToward 目标字符串 \"{}\" 不相邻", s);
                    return None;
                }
            }
        }
    }

    None
}

/// 解析 Gather 动作
fn parse_gather(json: &Value) -> Option<ActionType> {
    let res = json["params"]["resource"].as_str().unwrap_or("food");
    let resource = match res {
        "iron" | "Iron" | "铁矿" => ResourceType::Iron,
        "food" | "Food" | "食物" => ResourceType::Food,
        "wood" | "Wood" | "木材" => ResourceType::Wood,
        "water" | "Water" | "水源" => ResourceType::Water,
        "stone" | "Stone" | "石材" => ResourceType::Stone,
        _ => ResourceType::Food,
    };
    Some(ActionType::Gather { resource })
}

/// 解析 Talk 动作
fn parse_talk(json: &Value) -> Option<ActionType> {
    let message = json["params"]["message"]
        .as_str()
        .or_else(|| json["params"]["topic"].as_str())
        .unwrap_or("你好");
    Some(ActionType::Talk { message: message.to_string() })
}

/// 解析 Build 动作
fn parse_build(json: &Value) -> Option<ActionType> {
    let structure = json["params"]["structure"].as_str().unwrap_or("Camp");
    let structure_type = match structure {
        "Camp" | "camp" | "营地" => StructureType::Camp,
        "Fence" | "fence" | "围栏" => StructureType::Fence,
        "Warehouse" | "warehouse" | "仓库" => StructureType::Warehouse,
        _ => StructureType::Camp,
    };
    Some(ActionType::Build { structure: structure_type })
}

/// 解析 Attack 动作
fn parse_attack(json: &Value) -> Option<ActionType> {
    let target_id = json["params"]["target_id"]
        .as_str()
        .or_else(|| json["target"].as_str())
        .unwrap_or("unknown");
    Some(ActionType::Attack { target_id: AgentId::new(target_id) })
}

/// 解析 TradeOffer 动作
fn parse_trade_offer(json: &Value) -> Option<ActionType> {
    let target_id = json["params"]["target_id"]
        .as_str()
        .or_else(|| json["target"].as_str())
        .unwrap_or("unknown");
    let offer = parse_resource_map(&json["params"]["offer"]);
    let want = parse_resource_map(&json["params"]["want"]);
    Some(ActionType::TradeOffer {
        offer,
        want,
        target_id: AgentId::new(target_id),
    })
}

/// 解析 TradeAccept 动作
fn parse_trade_accept(json: &Value) -> Option<ActionType> {
    let trade_id = json["params"]["trade_id"].as_str().unwrap_or("default");
    Some(ActionType::TradeAccept { trade_id: trade_id.to_string() })
}

/// 解析 TradeReject 动作
fn parse_trade_reject(json: &Value) -> Option<ActionType> {
    let trade_id = json["params"]["trade_id"].as_str().unwrap_or("default");
    Some(ActionType::TradeReject { trade_id: trade_id.to_string() })
}

/// 解析 AllyPropose 动作
fn parse_ally_propose(json: &Value) -> Option<ActionType> {
    let target_id = json["params"]["target_id"]
        .as_str()
        .or_else(|| json["target"].as_str())
        .unwrap_or("unknown");
    Some(ActionType::AllyPropose { target_id: AgentId::new(target_id) })
}

/// 解析 AllyAccept 动作
fn parse_ally_accept(json: &Value) -> Option<ActionType> {
    let ally_id = json["params"]["ally_id"]
        .as_str()
        .or_else(|| json["target"].as_str())
        .unwrap_or("unknown");
    Some(ActionType::AllyAccept { ally_id: AgentId::new(ally_id) })
}

/// 解析 AllyReject 动作
fn parse_ally_reject(json: &Value) -> Option<ActionType> {
    let ally_id = json["params"]["ally_id"]
        .as_str()
        .or_else(|| json["target"].as_str())
        .unwrap_or("unknown");
    Some(ActionType::AllyReject { ally_id: AgentId::new(ally_id) })
}

/// 解析资源映射 JSON
pub fn parse_resource_map(value: &Value) -> HashMap<ResourceType, u32> {
    let mut map = HashMap::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            let resource = match k.as_str() {
                "iron" | "Iron" | "铁矿" => ResourceType::Iron,
                "food" | "Food" | "食物" => ResourceType::Food,
                "wood" | "Wood" | "木材" => ResourceType::Wood,
                "water" | "Water" | "水源" => ResourceType::Water,
                "stone" | "Stone" | "石材" => ResourceType::Stone,
                _ => continue,
            };
            let amount = v.as_u64().unwrap_or(0) as u32;
            if amount > 0 {
                map.insert(resource, amount);
            }
        }
    }
    map
}

/// 格式化动作解析错误信息
fn format_action_error(action_type_str: &str, json: &Value) -> String {
    // 为 MoveToward 解析失败提供详细的失败原因
    if action_type_str == "MoveToward" || action_type_str == "Move" || action_type_str.contains("移动") || action_type_str.contains("前往") {
        // 检查 direction 字段是否存在但值无效
        if let Some(dir_str) = json["params"]["direction"].as_str()
            .or_else(|| json["direction"].as_str())
        {
            let valid_dirs = ["north", "south", "east", "west", "北", "南", "东", "西"];
            if !valid_dirs.contains(&dir_str.trim().to_lowercase().as_str()) {
                return format!("MoveToward 方向 '{}' 不合法，只支持 north/south/east/west（或 北/南/东/西）", dir_str);
            }
        }
        // 检查是否有 direction 字段但值为斜向
        if let Some(dir_str) = json["params"]["direction"].as_str() {
            let diagonal_dirs = ["northeast", "northwest", "southeast", "southwest", "东北", "西北", "东南", "西南"];
            if diagonal_dirs.contains(&dir_str.trim().to_lowercase().as_str()) {
                return format!("MoveToward 不支持斜向移动 '{}', 请选择单一方向（north/south/east/west）逐步移动", dir_str);
            }
        }
        format!("MoveToward 缺少有效的 direction 参数，请提供 north/south/east/west")
    } else {
        format!("未知的动作类型：{}", action_type_str)
    }
}