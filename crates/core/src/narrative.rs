//! 叙事系统：统一管理事件描述格式和颜色编码
//!
//! 设计目标：
//! - 所有叙事描述集中在一处，方便维护和修改
//! - 颜色编码单一数据源：后端定义，通过 NarrativeEvent.color_code 传递给前端
//! - 前端直接使用传来的颜色，不维护本地映射，确保一致性

use crate::types::{ActionType, Direction, Position, ResourceType, StructureType};

/// 事件类型枚举（对应 NarrativeEvent.event_type）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    // 移动类
    Move,
    MoveToward,

    // 资源类
    Gather,
    Eat,
    Drink,

    // 社交类
    Talk,
    TradeOffer,
    TradeAccept,
    TradeReject,

    // 军事类
    Attack,

    // 联盟类
    AllyPropose,
    AllyAccept,
    AllyReject,

    // 建筑类
    Build,

    // 遗产类
    Legacy,
    Death,

    // 状态类
    Wait,
    Error,
    Pressure,
    PressureStart,
    PressureEnd,
    Milestone,
    Healed,
    Survival,
}

impl EventType {
    /// 获取事件类型的颜色编码（与 Godot narrative_feed.gd 保持一致）
    pub fn color_code(&self) -> &'static str {
        match self {
            // 白色 - 基础动作
            EventType::Move | EventType::MoveToward | EventType::Wait => "#FFFFFF",

            // 浅绿 - 探索/采集
            EventType::Gather => "#88CC44",

            // 绿色 - 恢复/治愈
            EventType::Eat | EventType::Drink | EventType::Healed => "#4CAF50",

            // 灰色 - 对话
            EventType::Talk => "#9E9E9E",

            // 绿色 - 交易成功
            EventType::TradeAccept => "#4CAF50",
            EventType::TradeOffer | EventType::TradeReject => "#FFAA88",

            // 红色 - 攻击/死亡
            EventType::Attack => "#F44336",
            EventType::Death => "#9C27B0",

            // 蓝色 - 结盟
            EventType::AllyPropose | EventType::AllyAccept => "#2196F3",
            EventType::AllyReject => "#FFAA88",

            // 粉色 - 建筑
            EventType::Build => "#FF44AA",

            // 紫色 - 遗产
            EventType::Legacy => "#9C27B0",

            // 黄色/橙色 - 压力
            EventType::Pressure => "#FFC107",
            EventType::PressureStart => "#FF9800",
            EventType::PressureEnd => "#8BC34A",

            // 金色 - 里程碑
            EventType::Milestone => "#FFD700",

            // 粉色 - 生存警告
            EventType::Survival => "#E91E63",

            // 红色 - 错误
            EventType::Error => "#FF6666",
        }
    }

    /// 获取事件类型的字符串标识（用于 JSON 序列化）
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::Move => "move",
            EventType::MoveToward => "move_toward",
            EventType::Gather => "gather",
            EventType::Eat => "eat",
            EventType::Drink => "drink",
            EventType::Talk => "talk",
            EventType::TradeOffer => "trade",
            EventType::TradeAccept => "trade_accept",
            EventType::TradeReject => "trade_reject",
            EventType::Attack => "attack",
            EventType::AllyPropose => "ally",
            EventType::AllyAccept => "ally_accept",
            EventType::AllyReject => "ally_reject",
            EventType::Build => "build",
            EventType::Legacy => "legacy",
            EventType::Death => "death",
            EventType::Wait => "wait",
            EventType::Error => "error",
            EventType::Pressure => "pressure",
            EventType::PressureStart => "pressure_start",
            EventType::PressureEnd => "pressure_end",
            EventType::Milestone => "milestone",
            EventType::Healed => "healed",
            EventType::Survival => "survival",
        }
    }
}

/// 叙事描述生成器
pub struct NarrativeBuilder {
    agent_name: String,
}

impl NarrativeBuilder {
    pub fn new(agent_name: String) -> Self {
        Self { agent_name }
    }

    /// 移动叙事：包含方向和坐标变化
    pub fn move_toward(&self, from: Position, to: Position) -> String {
        let dx = to.x as i32 - from.x as i32;
        let dy = to.y as i32 - from.y as i32;
        let direction_name = Direction::from_delta(dx, dy)
            .map(|d| d.as_chinese())
            .unwrap_or("未知方向");

        format!("{} 向{}移动 ({},{})→({},{})",
            self.agent_name, direction_name, from.x, from.y, to.x, to.y)
    }

    /// 已在目标位置叙事
    pub fn already_at_position(&self, pos: Position) -> String {
        format!("{} 已在目标位置 ({},{})，无需移动", self.agent_name, pos.x, pos.y)
    }

    /// 探索叙事
    pub fn explore(&self, steps: u32, from: Position, to: Position) -> String {
        format!("{} 探索周边区域，向{}移动了 {} 步 ({},{})→({},{})",
            self.agent_name, self.random_direction_name(), steps, from.x, from.y, to.x, to.y)
    }

    /// 采集叙事
    pub fn gather(&self, resource_type: ResourceType, amount: u32) -> String {
        format!("{} 采集了 {} 个 {}", self.agent_name, amount, resource_type.as_str())
    }

    /// 进食叙事
    pub fn eat(&self, satiety_restored: u32) -> String {
        format!("{} 进食，恢复饱食度 (+{})", self.agent_name, satiety_restored)
    }

    /// 饮水叙事
    pub fn drink(&self, hydration_restored: u32) -> String {
        format!("{} 饮水，恢复水分度 (+{})", self.agent_name, hydration_restored)
    }

    /// 等待叙事
    pub fn wait(&self) -> String {
        format!("{} 等待了一回合", self.agent_name)
    }

    /// 建造叙事
    pub fn build(&self, structure_type: StructureType, pos: Position) -> String {
        let structure_name = match structure_type {
            StructureType::Camp => "营地",
            StructureType::Fence => "围栏",
            StructureType::Warehouse => "仓库",
        };
        format!("{} 在 ({},{}) 建造了 {}", self.agent_name, pos.x, pos.y, structure_name)
    }

    /// 攻击叙事（命中）
    pub fn attack_hit(&self, target_name: &str, damage: u32) -> String {
        format!("{} 攻击了 {}，造成 {} 点伤害", self.agent_name, target_name, damage)
    }

    /// 攻击叙事（击败）
    pub fn attack_defeated(&self, target_name: &str) -> String {
        format!("{} 攻击了 {} 并将其击败", self.agent_name, target_name)
    }

    /// 对话叙事
    pub fn talk_to(&self, target_names: &[String], message: &str) -> String {
        if target_names.len() == 1 {
            format!("{} 与 {} 交流：「{}」", self.agent_name, target_names[0], message)
        } else {
            format!("{} 向 {} 说：「{}」", self.agent_name, target_names.join("、"), message)
        }
    }

    /// 自言自语叙事
    pub fn talk_self(&self, message: &str) -> String {
        format!("{} 自言自语：「{}」", self.agent_name, message)
    }

    /// 交易提议叙事
    pub fn trade_offer(&self, target_name: &str) -> String {
        format!("{} 向 {} 发起交易请求", self.agent_name, target_name)
    }

    /// 交易完成叙事
    pub fn trade_completed(&self, partner_name: &str) -> String {
        format!("{} 与 {} 完成了交易", self.agent_name, partner_name)
    }

    /// 交易拒绝叙事
    pub fn trade_rejected(&self, proposer_name: &str) -> String {
        format!("{} 拒绝了 {} 的交易请求", self.agent_name, proposer_name)
    }

    /// 结盟提议叙事
    pub fn ally_propose(&self, target_name: &str) -> String {
        format!("{} 向 {} 提议结盟", self.agent_name, target_name)
    }

    /// 结盟成功叙事
    pub fn ally_formed(&self, partner_name: &str) -> String {
        format!("{} 与 {} 结成了联盟", self.agent_name, partner_name)
    }

    /// 结盟拒绝叙事
    pub fn ally_rejected(&self, proposer_name: &str) -> String {
        format!("{} 拒绝了 {} 的结盟请求", self.agent_name, proposer_name)
    }

    /// 死亡叙事
    pub fn death(&self, age: u32, resource_scattered: bool) -> String {
        let suffix = if resource_scattered { "，资源散落在地" } else { "" };
        format!("{} 已死亡（享年 {} 岁）{}", self.agent_name, age, suffix)
    }

    /// 错误叙事
    pub fn error(&self, action_type: ActionType, reason: &str) -> String {
        let action_name = action_type_display(action_type);
        format!("{} 尝试{}失败：{}", self.agent_name, action_name, reason)
    }

    /// 随机方向名称（用于探索等随机移动）
    fn random_direction_name(&self) -> &'static str {
        // 实际方向由调用方传入，这里只是占位
        "某方向"
    }
}

/// 获取动作类型的中文显示名
pub fn action_type_display(action_type: ActionType) -> &'static str {
    match action_type {
        ActionType::MoveToward { .. } => "导航移动",
        ActionType::Gather { .. } => "采集",
        ActionType::Build { .. } => "建造",
        ActionType::Attack { .. } => "攻击",
        ActionType::Talk { .. } => "对话",
        ActionType::TradeOffer { .. } => "交易提议",
        ActionType::TradeAccept { .. } => "交易接受",
        ActionType::TradeReject { .. } => "交易拒绝",
        ActionType::AllyPropose { .. } => "结盟提议",
        ActionType::AllyAccept { .. } => "结盟接受",
        ActionType::AllyReject { .. } => "结盟拒绝",
        ActionType::Wait => "休息",
        ActionType::Eat => "进食",
        ActionType::Drink => "饮水",
        ActionType::InteractLegacy { .. } => "遗产交互",
    }
}

/// 压力事件描述模板
pub mod pressure_templates {
    use super::EventType;

    pub fn drought_start(duration: u32) -> (&'static str, String) {
        (EventType::PressureStart.color_code(),
         format!("☀️ 干旱降临（持续 {} ticks），资源产出减少", duration))
    }

    pub fn drought_end() -> (&'static str, String) {
        (EventType::PressureEnd.color_code(),
         "🌧️ 干旱已结束，资源恢复正常".to_string())
    }

    pub fn abundance_start(duration: u32) -> (&'static str, String) {
        (EventType::PressureStart.color_code(),
         format!("🌾 资源丰饶期（持续 {} ticks），采集收益翻倍", duration))
    }

    pub fn abundance_end() -> (&'static str, String) {
        (EventType::PressureEnd.color_code(),
         "🍃 资源丰饶期已结束".to_string())
    }

    pub fn plague_start(duration: u32) -> (&'static str, String) {
        (EventType::PressureStart.color_code(),
         format!("☠️ 瘟疫蔓延（持续 {} ticks），远离人群可减少感染", duration))
    }

    pub fn plague_end() -> (&'static str, String) {
        (EventType::PressureEnd.color_code(),
         "💚 瘟疫已消退".to_string())
    }
}

/// 里程碑事件描述模板
pub mod milestone_templates {
    use super::EventType;

    const MILESTONE_ICONS: &[(&str, &str)] = &[
        ("FirstCamp", "🏕"),
        ("FirstTrade", "🤝"),
        ("FirstFence", "🚧"),
        ("FirstAttack", "⚔"),
        ("FirstLegacyInteract", "📜"),
        ("CityState", "🏛"),
        ("GoldenAge", "👑"),
    ];

    pub fn milestone_reached(name: &str, display_name: &str) -> (&'static str, String) {
        let icon = MILESTONE_ICONS.iter()
            .find(|(n, _)| *n == name)
            .map(|(_, i)| *i)
            .unwrap_or("🏆");

        (EventType::Milestone.color_code(),
         format!("{} 达成：【{}】", icon, display_name))
    }
}