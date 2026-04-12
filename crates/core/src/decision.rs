//! 决策管道：硬约束→上下文→LLM→校验→选择

use crate::motivation::{MotivationVector, DIMENSION_NAMES};
use crate::types::{ActionType, AgentId};
use crate::rule_engine::{RuleEngine, WorldState};
use crate::prompt::PromptBuilder;
use crate::strategy::retrieve::{retrieve_strategy, get_strategy_summary, wrap_strategy_for_prompt};
use crate::strategy::StrategyHub;
use agentora_ai::config::MemoryConfig;
use agentora_ai::provider::LlmProvider;
use agentora_ai::types::{LlmRequest, ResponseFormat};
use agentora_ai::parser::parse_action_json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 候选动作来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateSource {
    Llm,
    RuleEngine,
}

/// 动作候选：统一承载 LLM 生成和规则引擎兜底的候选动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCandidate {
    pub reasoning: String,
    pub action_type: ActionType,
    pub target: Option<String>,
    pub params: HashMap<String, serde_json::Value>,
    pub motivation_delta: [f32; 6],
    pub source: CandidateSource,
}

/// 决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub selected_action: ActionCandidate,
    pub all_candidates: Vec<ActionCandidate>,
    pub error_info: Option<String>,
}

/// Spark类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SparkType {
    ResourcePressure,   // 资源压力
    SocialPressure,     // 社交压力
    CognitivePressure,  // 认知压力
    ExpressivePressure, // 表达压力
    PowerPressure,      // 权力压力
    LegacyPressure,     // 传承压力
    Explore,            // 探索（无明确压力时）
}

impl SparkType {
    /// 从动机维度索引获取对应的Spark类型
    pub fn from_dimension(dim: usize) -> Self {
        match dim {
            0 => SparkType::ResourcePressure,
            1 => SparkType::SocialPressure,
            2 => SparkType::CognitivePressure,
            3 => SparkType::ExpressivePressure,
            4 => SparkType::PowerPressure,
            5 => SparkType::LegacyPressure,
            _ => SparkType::Explore,
        }
    }

    /// 获取Spark类型名称
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

/// Spark：动机缺口驱动的决策触发器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spark {
    pub spark_type: SparkType,
    pub gap_value: f32,
    pub description: String,
}

impl Spark {
    /// 从动机缺口生成Spark
    pub fn from_gap(motivation: &MotivationVector, satisfaction: &[f32; 6]) -> Self {
        let max_dim = motivation.max_gap_dimension(satisfaction);
        let gap = motivation.compute_gap(satisfaction);
        let gap_value = gap[max_dim];

        // 如果所有缺口都很小，生成探索类Spark
        if gap_value < 0.1 {
            // 找动机值最高的维度
            let mut max_dim = 0;
            let mut max_val = motivation[0];
            for i in 1..6 {
                if motivation[i] > max_val {
                    max_val = motivation[i];
                    max_dim = i;
                }
            }
            Self {
                spark_type: SparkType::Explore,
                gap_value: 0.0,
                description: format!("无明确压力，按最高动机{}探索", DIMENSION_NAMES[max_dim]),
            }
        } else {
            Self {
                spark_type: SparkType::from_dimension(max_dim),
                gap_value,
                description: format!("{}缺口 {:.2}", DIMENSION_NAMES[max_dim], gap_value),
            }
        }
    }
}

/// 决策管道
pub struct DecisionPipeline {
    rule_engine: RuleEngine,
    prompt_builder: PromptBuilder,
    llm_provider: Option<Box<dyn LlmProvider>>,
    strategy_hub: Option<StrategyHub>,
}

impl DecisionPipeline {
    /// 从配置初始化
    pub fn from_config(config: &MemoryConfig) -> Self {
        Self {
            rule_engine: RuleEngine::new(),
            prompt_builder: PromptBuilder::from_config(config),
            llm_provider: None,
            strategy_hub: None,
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

    /// 执行完整五阶段决策管道
    pub async fn execute(
        &self,
        agent_id: &AgentId,
        motivation: &MotivationVector,
        spark: &Spark,
        world_state: &WorldState,
    ) -> DecisionResult {
        tracing::info!("开始决策管道执行 for agent {}", agent_id.as_str());

        // 阶段 1: 硬约束过滤 - 生成合法候选动作
        let filtered_actions = self.rule_engine.filter_hard_constraints(world_state);
        println!("[Decision] Agent {} 硬约束过滤后剩余 {} 个候选动作", agent_id.as_str(), filtered_actions.len());

        // 阶段 2: 上下文构建 (Prompt 组装)
        let prompt = self.build_prompt(agent_id, motivation, spark, world_state);
        println!("[Decision] Agent {} Prompt 长度：{} chars", agent_id.as_str(), prompt.len());

        // 阶段 3: LLM 调用
        match self.call_llm(&prompt).await {
            Ok(llm_candidates) => {
                println!("[Decision] Agent {} LLM 返回 {} 个候选动作", agent_id.as_str(), llm_candidates.len());

                // 阶段 4: 规则校验
                let validated: Vec<ActionCandidate> = llm_candidates
                    .into_iter()
                    .filter_map(|c| {
                        let is_valid = self.rule_engine.validate_action(&c, world_state);
                        if !is_valid {
                            println!("[Decision] Agent {} 动作校验失败：{:?}", agent_id.as_str(), c.action_type);
                            // Gather 不合法时，转换为 Explore（向附近移动寻找资源）
                            if let ActionType::Gather { resource } = &c.action_type {
                                println!("[Decision] Agent {} Gather {:?} 当前位置无资源，转换为 Explore 兜底", agent_id.as_str(), resource);
                                return Some(ActionCandidate {
                                    reasoning: format!("LLM建议采集{:?}但当前位置无资源，改为探索寻找", resource),
                                    action_type: ActionType::Explore { target_region: 0 },
                                    target: c.target,
                                    params: c.params,
                                    motivation_delta: c.motivation_delta,
                                    source: c.source,
                                });
                            }
                            return None;
                        }
                        Some(c)
                    })
                    .collect();

                println!("[Decision] Agent {} 规则校验后剩余 {} 个候选动作", agent_id.as_str(), validated.len());

                // 阶段 5: 动机加权选择
                if validated.is_empty() {
                    // 无候选通过校验，使用规则引擎兜底
                    let fallback = self.rule_engine.fallback_action(motivation, world_state);
                    println!("[Decision] Agent {} LLM 候选均未通过校验，使用规则引擎兜底: {:?}", agent_id.as_str(), fallback.action_type);
                    DecisionResult {
                        selected_action: fallback,
                        all_candidates: vec![],
                        error_info: Some("LLM 候选均未通过校验，使用规则引擎兜底".to_string()),
                    }
                } else if validated.len() == 1 {
                    // 唯一候选直接选择
                    let selected = validated.first().unwrap().clone();
                    println!("[Decision] Agent {} 唯一候选直接选择：{:?}", agent_id.as_str(), selected.action_type);
                    DecisionResult {
                        selected_action: selected,
                        all_candidates: validated,
                        error_info: None,
                    }
                } else {
                    // 多候选加权选择
                    let selected = self.select_with_motivation(&validated, motivation);
                    println!("[Decision] Agent {} 动机加权选择：{:?}", agent_id.as_str(), selected.action_type);
                    DecisionResult {
                        selected_action: selected,
                        all_candidates: validated,
                        error_info: None,
                    }
                }
            }
            Err(e) => {
                // LLM 调用失败，降级到规则引擎
                println!("[Decision] Agent {} LLM 调用失败，降级到规则引擎: {}", agent_id.as_str(), e);
                let fallback = self.rule_engine.fallback_action(motivation, world_state);
                println!("[Decision] Agent {} 规则引擎兜底: {:?}", agent_id.as_str(), fallback.action_type);
                DecisionResult {
                    selected_action: fallback,
                    all_candidates: vec![],
                    error_info: Some(format!("LLM 调用失败：{}", e)),
                }
            }
        }
    }

    /// 构建 Prompt
    fn build_prompt(
        &self,
        agent_id: &AgentId,
        motivation: &MotivationVector,
        spark: &Spark,
        world_state: &WorldState,
    ) -> String {
        // 构建感知摘要
        let perception_summary = self.build_perception_summary(world_state);

        // 构建记忆摘要（从记忆系统获取）
        let memory_summary = "";

        // 构建策略提示（任务 5.1-5.4：从策略系统检索并注入）
        let strategy_hint = self.strategy_hub.as_ref().and_then(|hub| {
            retrieve_strategy(hub, spark.spark_type).map(|strategy| {
                let summary = get_strategy_summary(&strategy);
                wrap_strategy_for_prompt(&summary)
            })
        });

        self.prompt_builder.build_decision_prompt(
            agent_id.as_str(),
            motivation,
            spark,
            &perception_summary,
            memory_summary,
            strategy_hint.as_deref(),
        )
    }

    /// 构建 Prompt（带记忆系统）
    pub fn build_prompt_with_memory(
        &self,
        agent_id: &AgentId,
        motivation: &MotivationVector,
        spark: &Spark,
        world_state: &WorldState,
        memory_summary: &str,
        strategy_hint: Option<&str>,
    ) -> String {
        // 构建感知摘要
        let perception_summary = self.build_perception_summary(world_state);

        self.prompt_builder.build_decision_prompt(
            agent_id.as_str(),
            motivation,
            spark,
            &perception_summary,
            memory_summary,
            strategy_hint,
        )
    }

    /// 构建感知摘要
    fn build_perception_summary(&self, world_state: &WorldState) -> String {
        let mut summary = String::new();

        // 位置信息
        summary.push_str(&format!(
            "位置：({}, {})\n",
            world_state.agent_position.x,
            world_state.agent_position.y
        ));

        // 附近 Agent
        if !world_state.existing_agents.is_empty() {
            summary.push_str(&format!("附近 Agent 数量：{}\n", world_state.existing_agents.len()));
        }

        // 资源信息
        if !world_state.resources_at.is_empty() {
            summary.push_str("资源分布:\n");
            for (pos, resource) in &world_state.resources_at {
                summary.push_str(&format!(
                    "  ({}, {}): {:?}\n",
                    pos.x, pos.y, resource
                ));
            }
        }

        summary
    }

    /// 调用 LLM
    async fn call_llm(&self, prompt: &str) -> Result<Vec<ActionCandidate>, String> {
        if let Some(provider) = &self.llm_provider {
            let request = LlmRequest {
                prompt: prompt.to_string(),
                max_tokens: 500,
                temperature: 0.7,
                response_format: ResponseFormat::Json { schema: None },
                stop_sequences: vec![],
            };

            // 使用 tokio 超时确保不会无限期挂起
            let generate_fut = provider.generate(request);
            let response = match tokio::time::timeout(std::time::Duration::from_secs(60), generate_fut).await {
                Ok(Ok(resp)) => {
                    println!("[Decision] LLM 调用成功，raw_text 前200字符: {:.200}", resp.raw_text);
                    resp
                }
                Ok(Err(e)) => {
                    println!("[Decision] LLM 调用失败: {}", e);
                    return Err(format!("LLM 调用失败：{}", e));
                }
                Err(_) => {
                    println!("[Decision] LLM 调用超时（60秒）");
                    return Err("LLM 调用超时（60秒）".to_string());
                }
            };

            // 使用 parser 解析 JSON
            match parse_action_json(&response.raw_text) {
                Ok(json_value) => {
                    println!("[Decision] JSON 解析成功: {}", json_value);
                    // 将 JSON 转换为 ActionCandidate
                    match self.json_to_candidate(json_value) {
                        Ok(candidate) => Ok(vec![candidate]),
                        Err(e) => {
                            println!("[Decision] 转换候选动作失败: {}", e);
                            Err(format!("转换候选动作失败：{}", e))
                        }
                    }
                }
                Err(e) => {
                    println!("[Decision] JSON 解析失败: {}，原始响应: {}", e, response.raw_text);
                    Err(format!("JSON 解析失败：{}", e))
                }
            }
        } else {
            Err("未配置 LLM Provider".to_string())
        }
    }

    /// 将 JSON 值转换为 ActionCandidate
    fn json_to_candidate(&self, json: serde_json::Value) -> Result<ActionCandidate, String> {
        let reasoning = json["reasoning"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let action_type_str = json["action_type"]
            .as_str()
            .ok_or("缺少 action_type 字段")?;

        // 解析 action_type（简化版本，需要完整实现）
        let action_type = self.parse_action_type(action_type_str, &json)
            .ok_or(format!("未知的动作类型：{}", action_type_str))?;

        let target = json["target"].as_str().map(String::from);

        let params = json["params"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let motivation_delta = json["motivation_delta"]
            .as_array()
            .and_then(|arr| {
                if arr.len() == 6 {
                    let mut delta = [0.0; 6];
                    for (i, val) in arr.iter().enumerate() {
                        delta[i] = val.as_f64().unwrap_or(0.0) as f32;
                    }
                    Some(delta)
                } else {
                    None
                }
            })
            .unwrap_or([0.0; 6]);

        Ok(ActionCandidate {
            reasoning,
            action_type,
            target,
            params,
            motivation_delta,
            source: CandidateSource::Llm,
        })
    }

    /// 解析动作类型
    fn parse_action_type(&self, type_str: &str, json: &serde_json::Value) -> Option<ActionType> {
        use crate::types::{Direction, ResourceType, StructureType};

        match type_str {
            "Move" | "move" | "移动" => {
                let dir = json["params"]["direction"].as_str().unwrap_or("north");
                let direction = match dir {
                    "North" | "north" | "北" => Direction::North,
                    "South" | "south" | "南" => Direction::South,
                    "East" | "east" | "东" => Direction::East,
                    "West" | "west" | "西" => Direction::West,
                    _ => Direction::North,
                };
                Some(ActionType::Move { direction })
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
            "TradeOffer" | "trade" | "交易" => {
                // 交易暂时用 Wait 兜底
                Some(ActionType::Wait)
            }
            "AllyPropose" | "ally" | "结盟" => {
                // 结盟暂时用 Wait 兜底
                Some(ActionType::Wait)
            }
            _ => {
                println!("[Decision] 未知 action_type: {}，使用 Wait 兜底", type_str);
                Some(ActionType::Wait)
            }
        }
    }

    /// 动机加权选择（公共方法，供测试使用）
    pub fn select_unique_or_motivated(
        &self,
        candidates: &[ActionCandidate],
        motivation: &MotivationVector,
    ) -> ActionCandidate {
        if candidates.len() == 1 {
            return candidates[0].clone();
        }
        self.select_with_motivation(candidates, motivation)
    }

    /// 点积计算（公共方法，供测试使用）
    pub fn compute_dot_product(&self, a: &[f32; 6], b: &MotivationVector) -> f32 {
        self.dot_product(a, b)
    }

    /// 动机加权选择
    fn select_with_motivation(
        &self,
        candidates: &[ActionCandidate],
        motivation: &MotivationVector,
    ) -> ActionCandidate {
        // 计算每个候选的点积得分
        let scores: Vec<f32> = candidates
            .iter()
            .map(|c| self.dot_product(&c.motivation_delta, motivation))
            .collect();

        // 使用 softmax + temperature 选择
        self.softmax_select(candidates, &scores, 0.1)
    }

    /// 点积计算
    fn dot_product(&self, a: &[f32; 6], b: &MotivationVector) -> f32 {
        let mut sum = 0.0;
        for i in 0..6 {
            sum += a[i] * b[i];
        }
        sum
    }

    /// Softmax 选择（带 temperature）
    fn softmax_select(
        &self,
        candidates: &[ActionCandidate],
        scores: &[f32],
        temperature: f32,
    ) -> ActionCandidate {
        // 计算 exp(score / temperature)
        let max_score = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_scores: Vec<f32> = scores
            .iter()
            .map(|&s| ((s - max_score) / temperature).exp())
            .collect();

        let sum_exp: f32 = exp_scores.iter().sum();

        // 归一化为概率
        let probs: Vec<f32> = exp_scores
            .iter()
            .map(|&e| e / sum_exp)
            .collect();

        // 按概率采样选择（简化版本：选择概率最高的）
        let mut best_idx = 0;
        let mut best_prob = probs[0];
        for (i, &prob) in probs.iter().enumerate() {
            if prob > best_prob {
                best_prob = prob;
                best_idx = i;
            }
        }

        candidates[best_idx].clone()
    }
}

impl Default for DecisionPipeline {
    fn default() -> Self {
        Self::with_defaults()
    }
}