//! 集成测试 - 单Agent决策循环
//!
//! 测试本地LLM模式下的完整决策流程

use agentora_core::{
    Agent, AgentId, Position, World, WorldSeed,
};
use agentora_ai::{LlmProvider, LlmRequest, OpenAiProvider, parse_action_json, ResponseFormat};
use serde_json::Value;

const MODEL_ID: &str = "gemma-4-26b-a4b-it-ud";
const API_BASE: &str = "http://localhost:1234";

/// 单Agent决策循环测试
#[tokio::test]
async fn test_single_agent_decision_loop() {
    // 1. 创建世界
    let seed = WorldSeed::default();
    let mut world = World::new(&seed);

    // 2. 创建Agent并直接插入世界
    let agent_id = AgentId::new("agent-001");
    let position = Position::new(128, 128);
    let agent = Agent::new(agent_id.clone(), "探索者Alice".to_string(), position);

    // 设置库存
    {
        let mut agent = world.agents.entry(agent_id.clone()).or_insert(agent);
        agent.inventory.insert("food".to_string(), 50);
        agent.inventory.insert("water".to_string(), 30);
    }

    let agent = world.agents.get(&agent_id).unwrap();

    println!("=== 单Agent决策循环测试 ===");
    println!("Agent: {} @ ({}, {})", agent.name, agent.position.x, agent.position.y);
    println!("库存: {:?}", agent.inventory);

    // 3. 构建决策Prompt
    let prompt = build_decision_prompt(&agent, &world);
    println!("\n=== 决策Prompt ===");
    println!("{}", prompt);

    // 4. 调用LLM进行决策
    let provider = OpenAiProvider::new(
        API_BASE.to_string(),
        "".to_string(),
        MODEL_ID.to_string(),
    ).with_timeout(300);

    let request = LlmRequest {
        prompt,
        max_tokens: 500,
        temperature: 0.7,
        response_format: ResponseFormat::Json { schema: None },
        stop_sequences: vec!["\n\n".to_string()],
    };

    println!("\n=== 调用LLM ===");
    let result = provider.generate(request).await;

    match result {
        Ok(response) => {
            println!("✅ LLM响应成功!");
            println!("原始响应: {}", response.raw_text);

            // 5. 解析动作
            match parse_action_json(&response.raw_text) {
                Ok(action_json) => {
                    println!("\n✅ JSON解析成功!");
                    println!("解析后的动作: {}", action_json);

                    // 6. 验证动作
                    let has_reasoning = action_json.get("reasoning").and_then(|v| v.as_str()).is_some();
                    let has_action_type = action_json.get("action_type").and_then(|v| v.as_str()).is_some();

                    assert!(has_reasoning || has_action_type,
                        "动作应包含action_type或reasoning字段");

                    // 提取关键信息
                    let action_type = action_json["action_type"].as_str().unwrap_or("Wait");
                    let reasoning = action_json["reasoning"].as_str().unwrap_or("");
                    let target = action_json["target"].as_str().unwrap_or("");

                    println!("\n=== 决策结果 ===");
                    println!("动作类型: {}", action_type);
                    println!("目标: {}", target);
                    println!("理由: {}", reasoning);

                    // 7. 模拟执行动作
                    execute_action(&mut world, &agent_id, &action_json);
                    println!("\n✅ 动作执行完成");
                }
                Err(e) => {
                    println!("⚠️ JSON解析失败: {}", e);
                    println!("LLM响应内容仍可用于人工审核");
                }
            }
        }
        Err(e) => {
            println!("❌ LLM请求失败: {}", e);
            println!("使用fallback规则引擎决策...");

            // Fallback决策
            let fallback_action = fallback_decision(&agent);
            println!("Fallback动作: {:?}", fallback_action);
        }
    }
}

/// 构建决策Prompt
fn build_decision_prompt(agent: &Agent, world: &World) -> String {
    let mut prompt = String::new();

    prompt.push_str("你是一个自主决策的AI Agent，在一个共享世界中生存。\n\n");

    // Agent状态
    prompt.push_str(&format!(
        "【当前状态】\n- 名称: {}\n- 位置: ({}, {})\n- 健康值: {}/{}\n\n",
        agent.name, agent.position.x, agent.position.y, agent.health, agent.max_health
    ));

    // 库存
    prompt.push_str("【库存资源】\n");
    for (item, count) in &agent.inventory {
        prompt.push_str(&format!("- {}: {}\n", item, count));
    }
    prompt.push_str("\n");

    // 感知环境
    prompt.push_str("【感知环境】\n");
    prompt.push_str("- 地形: 平原\n");
    prompt.push_str("- 附近资源: 未知区域，需要探索\n");
    prompt.push_str("- 其他Agent: 无\n\n");

    // 决策格式要求
    prompt.push_str("请做出一个决策。输出格式为JSON:\n");
    prompt.push_str("```json\n");
    prompt.push_str("{\n");
    prompt.push_str("  \"reasoning\": \"决策理由\",\n");
    prompt.push_str("  \"action_type\": \"Move|Gather|TradeOffer|Talk|Attack|Build|Explore|Wait\",\n");
    prompt.push_str("  \"target\": \"目标位置或对象\",\n");
    prompt.push_str("  \"params\": {}\n");
    prompt.push_str("}\n");
    prompt.push_str("```\n");

    prompt
}

/// 执行动作（模拟）
fn execute_action(world: &mut World, agent_id: &AgentId, action: &Value) {
    let action_type = action["action_type"].as_str().unwrap_or("Wait");

    if let Some(agent) = world.agents.get_mut(agent_id) {
        match action_type {
            "Move" | "move" => {
                let direction = action["params"]["direction"].as_str().unwrap_or("north");
                let (dx, dy) = match direction {
                    "north" => (0, -1),
                    "south" => (0, 1),
                    "east" => (1, 0),
                    "west" => (-1, 0),
                    _ => (0, 0),
                };
                agent.position.x = ((agent.position.x as i32) + dx).max(0) as u32;
                agent.position.y = ((agent.position.y as i32) + dy).max(0) as u32;
                println!("Agent移动到 ({}, {})", agent.position.x, agent.position.y);
            }
            "Gather" | "gather" => {
                let resource = action["params"]["resource"].as_str().unwrap_or("food");
                let amount = agent.inventory.get(resource).unwrap_or(&0) + 10;
                agent.inventory.insert(resource.to_string(), amount);
                println!("Agent采集了10单位{}", resource);
            }
            "Explore" | "explore" => {
                let target = action["target"].as_str().unwrap_or("(130, 130)");
                println!("Agent开始探索目标区域: {}", target);
            }
            "Wait" | "wait" => {
                println!("Agent等待观察环境变化");
            }
            _ => {
                println!("未知动作类型: {}", action_type);
            }
        }
    }
}

/// Fallback决策（规则引擎）
fn fallback_decision(agent: &Agent) -> Value {
    let food = agent.inventory.get("food").unwrap_or(&0);
    if *food < 30 {
        serde_json::json!({
            "action_type": "Gather",
            "target": "food",
            "reasoning": "食物储备不足，需要采集",
            "params": {"resource": "food"}
        })
    } else {
        serde_json::json!({
            "action_type": "Explore",
            "target": "未知区域",
            "reasoning": "探索寻找更多资源",
            "params": {}
        })
    }
}

/// 多轮决策循环测试
#[tokio::test]
async fn test_multi_turn_decision_loop() {
    let seed = WorldSeed::default();
    let mut world = World::new(&seed);

    let agent_id = AgentId::new("agent-002");
    let position = Position::new(100, 100);
    let agent = Agent::new(agent_id.clone(), "采集者Bob".to_string(), position);

    // 直接插入并配置库存
    {
        let mut entry = world.agents.entry(agent_id.clone()).or_insert(agent);
        entry.inventory.insert("food".to_string(), 20);
        entry.inventory.insert("wood".to_string(), 10);
    }

    let provider = OpenAiProvider::new(
        API_BASE.to_string(),
        "".to_string(),
        MODEL_ID.to_string(),
    ).with_timeout(300);

    println!("=== 多轮决策循环测试 ===");

    // 执行3轮决策
    for round in 1..=3 {
        println!("\n--- 第 {} 轮 ---", round);
        world.advance_tick();

        let agent = world.agents.get(&agent_id).unwrap();
        let prompt = build_decision_prompt(agent, &world);

        let request = LlmRequest {
            prompt,
            max_tokens: 300,
            temperature: 0.7,
            response_format: ResponseFormat::Json { schema: None },
            stop_sequences: vec!["\n\n".to_string()],
        };

        let result = provider.generate(request).await;

        match result {
            Ok(response) => {
                match parse_action_json(&response.raw_text) {
                    Ok(action) => {
                        let action_type = action["action_type"].as_str().unwrap_or("Wait");
                        println!("决策: {}", action_type);
                        execute_action(&mut world, &agent_id, &action);
                    }
                    Err(_) => {
                        println!("JSON解析失败，使用fallback");
                    }
                }
            }
            Err(_) => {
                let agent = world.agents.get(&agent_id).unwrap();
                let fallback = fallback_decision(agent);
                println!("使用Fallback决策: {}", fallback["action_type"]);
            }
        }

        // 打印Agent状态
        let agent = world.agents.get(&agent_id).unwrap();
        println!("状态: 位置({}, {}) 库存{:?}",
            agent.position.x, agent.position.y, agent.inventory);
    }

    println!("\n✅ 多轮决策测试完成");
}
