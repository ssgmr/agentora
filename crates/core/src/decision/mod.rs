//! 决策管道：上下文构建 → LLM 生成 → 规则校验 → 执行
//!
//! 子模块：
//! - perception: 感知构建器

pub mod perception;

pub use perception::PerceptionBuilder;

use crate::agent::inventory::get_config;
use crate::types::{ActionType, AgentId, Position};
use crate::types::ResourceType;
use crate::rule_engine::{RuleEngine, WorldState};
use crate::prompt::PromptBuilder;
use crate::strategy::retrieve::{retrieve_strategy, get_strategy_summary, wrap_strategy_for_prompt};
use crate::strategy::StrategyHub;
use crate::world::vision::direction_description;
use agentora_ai::config::MemoryConfig;
use agentora_ai::provider::LlmProvider;
use agentora_ai::types::{LlmRequest, ResponseFormat};
use agentora_ai::parser::parse_action_json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 动作候选：LLM 生成的决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCandidate {
    pub reasoning: String,
    pub action_type: ActionType,
    pub target: Option<String>,
    pub params: HashMap<String, serde_json::Value>,
}

/// 决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub selected_action: Option<ActionCandidate>,
    pub all_candidates: Vec<ActionCandidate>,
    pub error_info: Option<String>,
    /// LLM 原始动作校验失败的详细信息，用于反馈给 LLM 让其自我修正
    pub validation_failure: Option<String>,
}

/// 决策管道
pub struct DecisionPipeline {
    rule_engine: RuleEngine,
    prompt_builder: PromptBuilder,
    llm_provider: Option<Box<dyn LlmProvider>>,
    strategy_hub: Option<StrategyHub>,
    max_tokens: u32,
    temperature: f32,
}

impl DecisionPipeline {
    /// 从配置初始化
    pub fn from_config(config: &MemoryConfig) -> Self {
        Self {
            rule_engine: RuleEngine::new(),
            prompt_builder: PromptBuilder::from_config(config),
            llm_provider: None,
            strategy_hub: None,
            max_tokens: 500,
            temperature: 0.7,
        }
    }

    /// 使用默认配置初始化（向后兼容）
    pub fn with_defaults() -> Self {
        Self::from_config(&MemoryConfig::default())
    }

    pub fn new() -> Self {
        Self {
            rule_engine: RuleEngine::new(),
            prompt_builder: PromptBuilder::new(),
            llm_provider: None,
            strategy_hub: None,
            max_tokens: 500,
            temperature: 0.7,
        }
    }

    /// 设置策略枢纽
    pub fn with_strategy_hub(mut self, hub: StrategyHub) -> Self {
        self.strategy_hub = Some(hub);
        self
    }

    /// 设置 LLM Provider
    pub fn with_llm_provider(mut self, provider: Box<dyn LlmProvider>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    /// 设置 LLM 生成参数
    pub fn with_llm_params(mut self, max_tokens: u32, temperature: f32) -> Self {
        self.max_tokens = max_tokens;
        self.temperature = temperature;
        self
    }

    /// 执行完整决策管道（接收预构建感知）
    ///
    /// 核心原则：LLM Provider 可用时，决策权完全交给 LLM。
    /// LLM 的决策即使有问题，也应通过 action_feedback 反馈让它自我修正，
    /// 而不是用规则引擎覆盖。规则引擎仅在 LLM Provider 不可用时兜底。
    ///
    /// # 参数
    /// - `agent_id`: Agent ID
    /// - `world_state`: 世界状态
    /// - `perception_summary`: 预构建的感知摘要（由 PerceptionBuilder 生成）
    /// - `memory_summary`: 记忆摘要
    /// - `action_feedback`: 上次动作反馈
    pub async fn execute(
        &self,
        agent_id: &AgentId,
        world_state: &WorldState,
        perception_summary: &str,
        memory_summary: Option<&str>,
        action_feedback: Option<&str>,
    ) -> DecisionResult {
        tracing::info!("开始决策管道执行 for agent {}", agent_id.as_str());

        // 阶段 1: 上下文构建 (Prompt 组装)
        let prompt = self.build_prompt(agent_id, world_state, perception_summary, memory_summary, action_feedback);
        tracing::info!("===== LLM Request Prompt =====\n{}\n===============================", prompt);
        tracing::trace!("Agent {} Prompt 长度：{} chars", agent_id.as_str(), prompt.len());

        // 阶段 2: LLM 调用
        match self.call_llm(&prompt, world_state.agent_position).await {
            Ok(llm_candidates) => {
                tracing::debug!("Agent {} LLM 返回 {} 个候选动作", agent_id.as_str(), llm_candidates.len());

                // 阶段 3: 规则校验
                let mut failure_reasons: Vec<String> = Vec::new();
                let validated: Vec<ActionCandidate> = llm_candidates
                    .into_iter()
                    .filter(|c| {
                        let (is_valid, reason) = self.rule_engine.validate_action(c, world_state);
                        if !is_valid {
                            let action_debug = format!("{:?}", c.action_type);
                            let detail = reason.unwrap_or_else(|| "未知原因".to_string());
                            tracing::warn!("Agent {} 动作校验失败：{}，原因：{}", agent_id.as_str(), action_debug, detail);
                            failure_reasons.push(format!("{}（{}）", action_debug, detail));
                        }
                        is_valid
                    })
                    .collect();

                tracing::debug!("Agent {} 规则校验后剩余 {} 个候选动作", agent_id.as_str(), validated.len());

                // 阶段 4: 选择
                if validated.is_empty() {
                    // LLM 候选均未通过校验，不执行动作，反馈错误让 LLM 下回合修正
                    let failure_detail = format!("动作校验失败：{}。请根据当前状态重新选择有效动作", failure_reasons.join("; "));
                    tracing::warn!("Agent {} {}", agent_id.as_str(), failure_detail);

                    DecisionResult {
                        selected_action: None,
                        all_candidates: vec![],
                        error_info: Some(failure_detail.clone()),
                        validation_failure: Some(failure_detail),
                    }
                } else {
                    // 校验通过，选择第一个候选动作
                    let selected = validated.into_iter().next().unwrap();
                    tracing::debug!("Agent {} 选择动作：{:?}", agent_id.as_str(), selected.action_type);
                    DecisionResult {
                        selected_action: Some(selected),
                        all_candidates: vec![],
                        error_info: None,
                        validation_failure: None,
                    }
                }
            }
            Err(e) => {
                // 判断 LLM Provider 是否可用
                let is_provider_unavailable = self.llm_provider.is_none()
                    || e.contains("未配置 LLM Provider")
                    || e.contains("LLM 调用超时")
                    || e.contains("LLM 调用失败");

                if is_provider_unavailable {
                    // LLM Provider 不可用，使用规则引擎兜底
                    tracing::warn!("Agent {} LLM Provider 不可用，降级到规则引擎: {}", agent_id.as_str(), e);
                    let fallback = self.rule_engine.survival_fallback(world_state);
                    DecisionResult {
                        selected_action: fallback,
                        all_candidates: vec![],
                        error_info: Some(format!("LLM 不可用：{}", e)),
                        validation_failure: None,
                    }
                } else {
                    // LLM Provider 可用但返回了无效的决策（解析失败、校验失败等）
                    // 不执行动作，反馈错误让 LLM 下回合修正
                    tracing::warn!("Agent {} LLM 返回无效决策: {}", agent_id.as_str(), e);

                    DecisionResult {
                        selected_action: None,
                        all_candidates: vec![],
                        error_info: Some(format!("LLM 响应无效：{}", e)),
                        validation_failure: Some(e.clone()),
                    }
                }
            }
        }
    }

    /// 执行决策管道（向后兼容，自行构建感知）
    ///
    /// 仅用于未迁移的调用方，新代码应使用 execute() 并传入预构建感知
    pub async fn execute_with_auto_perception(
        &self,
        agent_id: &AgentId,
        world_state: &WorldState,
        memory_summary: Option<&str>,
        action_feedback: Option<&str>,
    ) -> DecisionResult {
        let perception_summary = self.build_perception_summary(world_state);
        self.execute(agent_id, world_state, &perception_summary, memory_summary, action_feedback).await
    }

    /// 构建 Prompt（接收预构建感知）
    fn build_prompt(
        &self,
        agent_id: &AgentId,
        world_state: &WorldState,
        perception_summary: &str,
        memory_summary: Option<&str>,
        action_feedback: Option<&str>,
    ) -> String {
        // 使用传入的感知摘要（不再自行构建）
        // 使用传入的记忆摘要，默认为空
        let memory_summary = memory_summary.unwrap_or("");

        // 构建策略提示（基于 Agent 当前状态推断模式）
        let strategy_hint = self.strategy_hub.as_ref().and_then(|hub| {
            let state_mode = infer_state_mode(world_state);
            retrieve_strategy(hub, state_mode).map(|strategy| {
                let summary = get_strategy_summary(&strategy);
                wrap_strategy_for_prompt(&summary)
            })
        });

        // 提取附近建筑信息
        let nearby_structures: Vec<&str> = world_state.nearby_structures.iter()
            .map(|s| s.structure_type.as_str())
            .collect();

        // 提取活跃压力事件
        let active_pressures: Vec<&str> = world_state.active_pressures.iter()
            .map(|s| s.as_str())
            .collect();

        self.prompt_builder.build_decision_prompt(
            agent_id.as_str(),
            perception_summary,
            memory_summary,
            strategy_hint.as_deref(),
            action_feedback,
            get_config().max_stack_size,
            world_state.agent_personality.as_ref(),
            world_state.agent_satiety,
            world_state.agent_hydration,
            &nearby_structures,
            &active_pressures,
        ) + &self.build_temp_preferences_prompt(world_state)
    }

    /// 构建临时偏好提示
    fn build_temp_preferences_prompt(&self, world_state: &WorldState) -> String {
        if world_state.temp_preferences.is_empty() {
            return String::new();
        }

        let mut s = String::from("\n<guidance>\n[引导] 当前有外部引导倾向影响你的决策：\n");
        for (key, boost, remaining) in &world_state.temp_preferences {
            let label = match key.as_str() {
                "eat" => "进食",
                "drink" => "饮水",
                "gather" => "采集",
                "explore" => "探索",
                _ => key.as_str(),
            };
            s.push_str(&format!("  - {}（倾向强度: {:.1}, 剩余 {} 回合）\n", label, boost, remaining));
        }
        s.push_str("请适当考虑引导倾向，但你可以自主决定是否完全遵循。\n</guidance>\n");
        s
    }

    /// 构建感知摘要
    fn build_perception_summary(&self, world_state: &WorldState) -> String {
        let mut summary = String::new();

        // ===== 新增：推荐行动路径（最高优先级展示） =====
        self.build_path_recommendation(&mut summary, world_state);

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
                summary.push_str(&format!("  饱食度: {} [{}] — 需要进食！背包food: {}\n",
                    world_state.agent_satiety, satiety_status,
                    world_state.agent_inventory.get(&ResourceType::Food).copied().unwrap_or(0)));
            }
            if world_state.agent_hydration <= 30 {
                summary.push_str(&format!("  水分度: {} [{}] — 需要饮水！背包water: {}\n",
                    world_state.agent_hydration, hydration_status,
                    world_state.agent_inventory.get(&ResourceType::Water).copied().unwrap_or(0)));
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
                format!("每种资源上限{}，仓库附近可达{}，{} 已满（堆叠达上限）", effective_limit, effective_limit * 2, names.join("、"))
            };
            summary.push_str(&format!("背包（{}，当前合计{}）：", limit_note, total));
            let items: Vec<String> = world_state.agent_inventory.iter()
                .map(|(r, count)| format!("{} x{}", r.as_str(), count))
                .collect();
            summary.push_str(&items.join(", "));
            summary.push('\n');
        } else {
            let effective_limit = get_config().max_stack_size;
            summary.push_str(&format!("背包（每种资源上限{}，仓库附近可达{}，当前空）：\n", effective_limit, effective_limit * 2));
        }

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

        // 坐标系说明（帮助 LLM 正确理解方向）
        summary.push_str("方向规则：X增大=向东，X减小=向西，Y增大=向南，Y减小=向北。注意：Y轴向下增大，与数学坐标相反！\n");

        // 相邻格信息（用于 Agent 自主规划移动路线）
        let pos = world_state.agent_position;
        let dirs = [
            (crate::types::Direction::North, "北", "north", 0i32, -1i32),
            (crate::types::Direction::South, "南", "south", 0, 1),
            (crate::types::Direction::East, "东", "east", 1, 0),
            (crate::types::Direction::West, "西", "west", -1, 0),
        ];
        summary.push_str("相邻格（可移动方向，每格需1步）：\n");
        for (_dir, name, eng, dx, dy) in &dirs {
            let nx = pos.x as i32 + dx;
            let ny = pos.y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= world_state.map_size as i32 || ny >= world_state.map_size as i32 {
                summary.push_str(&format!("  {}: 越界(不可移动)\n", name));
            } else {
                let np = crate::types::Position::new(nx as u32, ny as u32);
                let terrain = world_state.terrain_at.get(&np);
                let terrain_icon = terrain.map(|t| format!("{:?}", t)).unwrap_or_else(|| "未知".to_string());

                // 资源标记（简化）
                let res_mark = world_state.resources_at.get(&np)
                    .map(|(r, a)| format!("{:?}×{}", r, a))
                    .unwrap_or_default();

                // Agent标记
                let agent_mark = world_state.nearby_agents.iter()
                    .find(|a| a.position == np)
                    .map(|a| format!(" Agent:{}", a.name))
                    .unwrap_or_default();

                // 明确的direction参数示例
                summary.push_str(&format!("  {}({},{}) {} {} {} → direction:\"{}\"\n",
                    name, nx, ny, terrain_icon, res_mark, agent_mark, eng));
            }
        }

        // 地形概览
        if !world_state.terrain_at.is_empty() {
            let mut terrain_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
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

        // 附近 Agent
        if !world_state.nearby_agents.is_empty() {
            summary.push_str(&format!("附近 Agent ({} 个):\n", world_state.nearby_agents.len()));
            for agent_info in &world_state.nearby_agents {
                let relation_str = match agent_info.relation_type {
                    crate::agent::RelationType::Ally => "盟友",
                    crate::agent::RelationType::Enemy => "敌人",
                    crate::agent::RelationType::Neutral => "陌生人",
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
            // 附近没有其他 Agent，告知自身信息
            summary.push_str("附近无其他 Agent（只有你自己）\n");
        }

        // 资源信息（增强：显示方向、距离、丰富度，按优先级排序）
        if !world_state.resources_at.is_empty() {
            summary.push_str("资源分布:\n");

            // 按生存优先级和距离排序
            let mut resources: Vec<_> = world_state.resources_at.iter().collect();
            let agent_pos = &world_state.agent_position;
            let satiety = world_state.agent_satiety;
            let hydration = world_state.agent_hydration;

            resources.sort_by(|(pos_a, (res_a, _)), (pos_b, (res_b, _))| {
                // 计算距离
                let dist_a = pos_a.manhattan_distance(agent_pos);
                let dist_b = pos_b.manhattan_distance(agent_pos);

                // 优先级函数：饥饿时 Food 优先，口渴时 Water 优先
                fn resource_priority(r: &ResourceType, satiety: u32, hydration: u32) -> u32 {
                    match r {
                        ResourceType::Food if satiety <= 50 => 0,  // 饥饿时食物最高优先
                        ResourceType::Water if hydration <= 50 => 0, // 口渴时水源最高优先
                        ResourceType::Food => 1,
                        ResourceType::Water => 2,
                        ResourceType::Wood => 3,
                        ResourceType::Stone => 4,
                        ResourceType::Iron => 5,
                    }
                }

                let priority_a = resource_priority(res_a, satiety, hydration);
                let priority_b = resource_priority(res_b, satiety, hydration);

                // 先按优先级排序，相同优先级按距离排序
                match priority_a.cmp(&priority_b) {
                    std::cmp::Ordering::Equal => dist_a.cmp(&dist_b),
                    other => other,
                }
            });

            for (pos, (resource, amount)) in resources {
                // 计算方向和距离描述
                let dir_desc = direction_description(agent_pos, pos);

                // 丰富度描述
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

        // 附近建筑
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

        // 附近遗产
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

        // 宏观区域上下文
        let region_x = world_state.agent_position.x / 16;
        let region_y = world_state.agent_position.y / 16;
        let region_id = region_y * 16 + region_x;
        // 计算该区域主导地形（从 vision 数据估算）
        if !world_state.terrain_at.is_empty() {
            let mut terrain_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
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

        summary
    }

    /// 调用 LLM
    async fn call_llm(&self, prompt: &str, agent_pos: Position) -> Result<Vec<ActionCandidate>, String> {
        if let Some(provider) = &self.llm_provider {
            let request = LlmRequest {
                prompt: prompt.to_string(),
                max_tokens: self.max_tokens,
                temperature: self.temperature,
                response_format: ResponseFormat::Json { schema: None },
                stop_sequences: vec![],
            };

            // 使用 tokio 超时确保不会无限期挂起
            let generate_fut = provider.generate(request);
            let response = match tokio::time::timeout(std::time::Duration::from_secs(60), generate_fut).await {
                Ok(Ok(resp)) => {
                    tracing::info!("===== LLM Response =====\n{}\n==========================", resp.raw_text);
                    // 检测空响应：快速失败，避免无意义的 JSON 解析
                    if resp.raw_text.trim().is_empty() {
                        tracing::warn!("LLM 返回空响应，快速降级到规则引擎");
                        return Err("LLM 返回空响应".to_string());
                    }
                    resp
                }
                Ok(Err(e)) => {
                    tracing::error!("LLM 调用失败: {}", e);
                    return Err(format!("LLM 调用失败：{}", e));
                }
                Err(_) => {
                    tracing::warn!("LLM 调用超时（60秒）");
                    return Err("LLM 调用超时（60秒）".to_string());
                }
            };

            // 使用 parser 解析 JSON
            match parse_action_json(&response.raw_text) {
                Ok(json_value) => {
                    tracing::trace!("JSON 解析成功: {}", json_value);
                    // 将 JSON 转换为 ActionCandidate
                    match self.json_to_candidate(json_value, agent_pos) {
                        Ok(candidate) => Ok(vec![candidate]),
                        Err(e) => {
                            tracing::warn!("转换候选动作失败: {}", e);
                            Err(format!("转换候选动作失败：{}", e))
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("JSON 解析失败: {}，原始响应: {}", e, response.raw_text);
                    Err(format!("JSON 解析失败：{}", e))
                }
            }
        } else {
            Err("未配置 LLM Provider".to_string())
        }
    }

    /// 将 JSON 值转换为 ActionCandidate
    fn json_to_candidate(&self, json: serde_json::Value, agent_pos: Position) -> Result<ActionCandidate, String> {
        let reasoning = json["reasoning"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let action_type_str = json["action_type"]
            .as_str()
            .ok_or("缺少 action_type 字段")?;

        // 解析 action_type
        let action_type = self.parse_action_type(action_type_str, &json, agent_pos)
            .ok_or_else(|| {
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
            })?;

        let target = json["target"].as_str().map(String::from);

        let params = json["params"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(ActionCandidate {
            reasoning,
            action_type,
            target,
            params,
        })
    }

    /// 解析动作类型
    fn parse_action_type(&self, type_str: &str, json: &serde_json::Value, agent_pos: Position) -> Option<ActionType> {
        use crate::types::{ResourceType, StructureType};

        match type_str {
            "Move" | "move" | "移动" => {
                // Move 统一转为 MoveToward，支持方向和坐标两种格式
                // 优先尝试方向格式
                if let Some(dir_str) = json["params"]["direction"].as_str() {
                    let direction = match dir_str {
                        "North" | "north" | "北" | "n" | "N" => crate::types::Direction::North,
                        "South" | "south" | "南" | "s" | "S" => crate::types::Direction::South,
                        "East" | "east" | "东" | "e" | "E" => crate::types::Direction::East,
                        "West" | "west" | "西" | "w" | "W" => crate::types::Direction::West,
                        _ => return None,
                    };
                    let target = match direction {
                        crate::types::Direction::North => Position::new(agent_pos.x, agent_pos.y.wrapping_sub(1)),
                        crate::types::Direction::South => Position::new(agent_pos.x, agent_pos.y + 1),
                        crate::types::Direction::East => Position::new(agent_pos.x + 1, agent_pos.y),
                        crate::types::Direction::West => Position::new(agent_pos.x.wrapping_sub(1), agent_pos.y),
                    };
                    Some(ActionType::MoveToward { target })
                } else if let Some(target) = self.parse_target_position(json, agent_pos) {
                    Some(ActionType::MoveToward { target })
                } else {
                    None
                }
            }
            "MoveToward" | "move_toward" | "移动到" | "前往" => {
                let target = self.parse_target_position(json, agent_pos)?;
                Some(ActionType::MoveToward { target })
            }
            "Gather" | "gather" | "采集" | "收集" => {
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
            "Wait" | "wait" | "等待" => Some(ActionType::Wait),
            "Eat" | "eat" | "进食" | "吃东西" => Some(ActionType::Eat),
            "Drink" | "drink" | "饮水" | "喝水" => Some(ActionType::Drink),
            "Explore" | "explore" | "探索" => {
                let region = json["params"]["target_region"].as_u64().unwrap_or(0) as u32;
                Some(ActionType::Explore { target_region: region })
            }
            "Talk" | "talk" | "对话" | "交流" => {
                // 优先从 params.message 获取，其次从 target 获取，最后用默认值
                let message = json["params"]["message"]
                    .as_str()
                    .or_else(|| json["params"]["topic"].as_str())
                    .unwrap_or("你好");
                Some(ActionType::Talk { message: message.to_string() })
            }
            "Build" | "build" | "建造" => {
                let structure = json["params"]["structure"].as_str().unwrap_or("Camp");
                let structure_type = match structure {
                    "Camp" | "camp" | "营地" => StructureType::Camp,
                    "Fence" | "fence" | "围栏" => StructureType::Fence,
                    "Warehouse" | "warehouse" | "仓库" => StructureType::Warehouse,
                    _ => StructureType::Camp,
                };
                Some(ActionType::Build { structure: structure_type })
            }
            "Attack" | "attack" | "攻击" => {
                let target_id = json["params"]["target_id"]
                    .as_str()
                    .or_else(|| json["target"].as_str())
                    .unwrap_or("unknown");
                Some(ActionType::Attack { target_id: AgentId::new(target_id) })
            }
            "TradeOffer" | "trade" | "交易" | "交易提议" => {
                // 解析交易提议参数
                let target_id = json["params"]["target_id"]
                    .as_str()
                    .or_else(|| json["target"].as_str())
                    .unwrap_or("unknown");
                let offer = Self::parse_resource_map(&json["params"]["offer"]);
                let want = Self::parse_resource_map(&json["params"]["want"]);
                Some(ActionType::TradeOffer {
                    offer,
                    want,
                    target_id: AgentId::new(target_id),
                })
            }
            "TradeAccept" | "交易接受" => {
                let trade_id = json["params"]["trade_id"]
                    .as_str()
                    .unwrap_or("default");
                Some(ActionType::TradeAccept { trade_id: trade_id.to_string() })
            }
            "TradeReject" | "交易拒绝" => {
                let trade_id = json["params"]["trade_id"]
                    .as_str()
                    .unwrap_or("default");
                Some(ActionType::TradeReject { trade_id: trade_id.to_string() })
            }
            "AllyPropose" | "ally" | "结盟" | "结盟提议" => {
                let target_id = json["params"]["target_id"]
                    .as_str()
                    .or_else(|| json["target"].as_str())
                    .unwrap_or("unknown");
                Some(ActionType::AllyPropose { target_id: AgentId::new(target_id) })
            }
            "AllyAccept" | "结盟接受" => {
                let ally_id = json["params"]["ally_id"]
                    .as_str()
                    .or_else(|| json["target"].as_str())
                    .unwrap_or("unknown");
                Some(ActionType::AllyAccept { ally_id: AgentId::new(ally_id) })
            }
            "AllyReject" | "结盟拒绝" => {
                let ally_id = json["params"]["ally_id"]
                    .as_str()
                    .or_else(|| json["target"].as_str())
                    .unwrap_or("unknown");
                Some(ActionType::AllyReject { ally_id: AgentId::new(ally_id) })
            }
            _ => {
                tracing::warn!("未知 action_type: {}，使用 Wait 兜底", type_str);
                Some(ActionType::Wait)
            }
        }
    }

    /// 解析 MoveToward 目标位置
    ///
    /// 支持多种格式：
    /// - { x: 130, y: 125 }
    /// - [130, 125]
    /// - "130,125" 或 "(130, 125)"
    fn parse_target_position(&self, json: &serde_json::Value, agent_pos: Position) -> Option<Position> {
        // 优先尝试从 direction 字段解析（LLM 输出方向更可靠）
        // 支持 params.direction 和顶层 direction
        if let Some(dir_str) = json["params"]["direction"].as_str()
            .or_else(|| json["direction"].as_str())
        {
            let direction = match dir_str.trim() {
                "North" | "north" | "北" | "n" | "N" => Some(crate::types::Direction::North),
                "South" | "south" | "南" | "s" | "S" => Some(crate::types::Direction::South),
                "East" | "east" | "东" | "e" | "E" => Some(crate::types::Direction::East),
                "West" | "west" | "西" | "w" | "W" => Some(crate::types::Direction::West),
                _ => None,
            };
            if let Some(dir) = direction {
                let target = match dir {
                    crate::types::Direction::North => Position::new(agent_pos.x, agent_pos.y.wrapping_sub(1)),
                    crate::types::Direction::South => Position::new(agent_pos.x, agent_pos.y + 1),
                    crate::types::Direction::East => Position::new(agent_pos.x + 1, agent_pos.y),
                    crate::types::Direction::West => Position::new(agent_pos.x.wrapping_sub(1), agent_pos.y),
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
                    // 校验：如果坐标不相邻，自动修正为 nearest valid adjacent cell
                    if pos.manhattan_distance(&agent_pos) == 1 {
                        return Some(pos);
                    }
                    // LLM 输出了非相邻坐标，记录警告并返回 None（让调用者处理）
                    tracing::warn!("MoveToward 目标 ({},{}) 不相邻（距离 {}），LLM 不理解相邻约束", pos.x, pos.y, pos.manhattan_distance(&agent_pos));
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

    /// 解析资源映射 JSON
    fn parse_resource_map(value: &serde_json::Value) -> HashMap<ResourceType, u32> {
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

    /// 构建路径推荐（核心新增功能）
    ///
    /// 根据生存状态和资源分布，推荐最优移动路径，帮助LLM做出正确决策
    fn build_path_recommendation(&self, summary: &mut String, world_state: &WorldState) {
        use crate::types::Direction;

        // 确定优先资源类型（基于生存压力）
        let priority_resource = if world_state.agent_satiety <= 50 {
            Some(ResourceType::Food)
        } else if world_state.agent_hydration <= 50 {
            Some(ResourceType::Water)
        } else {
            None // 没有生存压力，可以自由探索或采集
        };

        // 如果有生存压力，找最近的优先资源并推荐路径
        if let Some(priority) = priority_resource {
            let nearest = world_state.resources_at.iter()
                .filter(|(_, (r, _))| *r == priority)
                .min_by_key(|(pos, _)| pos.manhattan_distance(&world_state.agent_position));

            if let Some((pos, (_, amount))) = nearest {
                let dist = pos.manhattan_distance(&world_state.agent_position);
                let dx = pos.x as i32 - world_state.agent_position.x as i32;
                let dy = pos.y as i32 - world_state.agent_position.y as i32;

                // 确定第一步方向（优先走较长轴）
                let (first_dir, dir_name, dir_eng) = if dx.abs() >= dy.abs() {
                    if dx > 0 { (Direction::East, "东", "east") }
                    else { (Direction::West, "西", "west") }
                } else {
                    if dy > 0 { (Direction::South, "南", "south") }
                    else { (Direction::North, "北", "north") }
                };

                // 检查第一步方向是否有效（不越界）
                let delta = first_dir.delta();
                let step_x = world_state.agent_position.x as i32 + delta.0;
                let step_y = world_state.agent_position.y as i32 + delta.1;
                let step_valid = step_x >= 0 && step_y >= 0 &&
                    step_x < world_state.map_size as i32 &&
                    step_y < world_state.map_size as i32;

                // 构建推荐
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
                    // 优先方向被阻挡，找替代方向
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

                // 如果背包有资源且生存危急，提醒可以直接使用
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
                // 视野内没有优先资源
                summary.push_str("【推荐路径】\n");
                summary.push_str(&format!(
                    "  视野内无{:?}资源，建议向任意有效方向探索\n",
                    priority
                ));
                // 找一个有效方向
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
        } else {
            // 没有生存压力，检查当前位置是否有资源
            let current_has_resource = world_state.resources_at.get(&world_state.agent_position);
            if current_has_resource.is_some() {
                let (r, amount) = current_has_resource.unwrap();
                summary.push_str("【推荐路径】\n");
                summary.push_str(&format!(
                    "  当前位置有{:?}×{}，可直接Gather采集\n\n",
                    r, amount
                ));
            } else {
                // 找最近的任意资源
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
}

impl Default for DecisionPipeline {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// 根据 Agent 当前状态推断决策模式，用于策略检索和记忆查询。
///
/// 这替代了原来从动机缺口推导 Spark 的机制，改为直接从
/// health/satiety/hydration/inventory 等状态值推断。
pub fn infer_state_mode(world_state: &WorldState) -> SparkType {
    // 生存优先：饥饿/口渴 → 资源压力
    if world_state.agent_satiety <= 30 || world_state.agent_hydration <= 30 {
        return SparkType::ResourcePressure;
    }
    // 社交模式：附近有其他 Agent
    if !world_state.nearby_agents.is_empty() {
        return SparkType::SocialPressure;
    }
    // 认知/探索模式：无生存压力且无社交 → 探索
    SparkType::Explore
}

/// 决策模式分类（原 SparkType，保留用于策略/记忆分类键）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SparkType {
    ResourcePressure,   // 资源压力（饥饿/口渴/缺资源）
    SocialPressure,     // 社交压力（附近有其他 Agent）
    CognitivePressure,  // 认知压力（学习/发现）
    ExpressivePressure, // 表达压力（创造/建造）
    PowerPressure,      // 权力压力（领导/影响）
    LegacyPressure,     // 传承压力（遗产/教导）
    Explore,            // 探索（无明确压力时）
}

impl SparkType {
    /// 获取模式名称
    pub fn name(&self) -> &str {
        match self {
            SparkType::ResourcePressure => "资源压力",
            SparkType::SocialPressure => "社交压力",
            SparkType::CognitivePressure => "认知压力",
            SparkType::ExpressivePressure => "表达压力",
            SparkType::PowerPressure => "权力压力",
            SparkType::LegacyPressure => "传承压力",
            SparkType::Explore => "探索",
        }
    }
}

impl std::fmt::Display for SparkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
