//! 本地LLM连接测试
//!
//! 测试本地LLM服务 (端口1234) 的连接和推理
//! Model: gemma-4-26b-a4b-it-ud

use agentora_ai::{LlmProvider, LlmRequest, OpenAiProvider};

const MODEL_ID: &str = "gemma-4-26b-a4b-it-ud";
const API_BASE: &str = "http://localhost:1234";

#[tokio::test]
async fn test_local_llm_connection() {
    // 配置本地LLM (OpenAI兼容端点)
    let provider = OpenAiProvider::new(
        API_BASE.to_string(),
        "".to_string(),  // 本地LLM不需要key
        MODEL_ID.to_string(),
    ).with_timeout(300);

    // 检查provider是否可用
    println!("Provider可用: {}", provider.is_available());
    println!("Provider名称: {}", provider.name());
    println!("Model ID: {}", MODEL_ID);

    // 创建测试请求
    let request = LlmRequest {
        prompt: "你好，请用JSON格式回复：{\"message\": \"测试成功\"}".to_string(),
        max_tokens: 100,
        temperature: 0.7,
        response_format: agentora_ai::types::ResponseFormat::Json { schema: None },
        stop_sequences: vec!["\n\n".to_string()],
    };

    // 发送请求
    let result = provider.generate(request).await;

    match result {
        Ok(response) => {
            println!("✅ LLM响应成功!");
            println!("Provider: {}", response.provider_name);
            println!("响应内容: {}", response.raw_text);
            println!("Token使用: prompt={}, completion={}, total={}",
                response.usage.prompt_tokens,
                response.usage.completion_tokens,
                response.usage.total_tokens);
        }
        Err(e) => {
            println!("❌ LLM请求失败: {}", e);
            // 不让测试失败，只是打印错误
            // panic!("LLM连接失败，请确认本地LLM服务在端口1234运行");
        }
    }
}

#[tokio::test]
async fn test_local_llm_decision_format() {
    // 测试决策格式的JSON响应
    let provider = OpenAiProvider::new(
        API_BASE.to_string(),
        "".to_string(),
        MODEL_ID.to_string(),
    ).with_timeout(300);

    // 模拟决策请求
    let request = LlmRequest {
        prompt: r#"你是游戏中的智能体。当前状态：
- 位置: (128, 128)
- 资源: 食物=50, 水=30
- 动机权重: 生存=0.8, 社交=0.4, 认知=0.3

请用以下JSON格式决定下一步行动：
{
  "action_type": "move|gather|trade|rest|explore",
  "target": "目标位置或对象",
  "reasoning": "简短理由"
}"#.to_string(),
        max_tokens: 200,
        temperature: 0.7,
        response_format: agentora_ai::types::ResponseFormat::Json { schema: None },
        stop_sequences: vec!["\n\n".to_string()],
    };

    let result = provider.generate(request).await;

    match result {
        Ok(response) => {
            println!("✅ 决策测试成功!");
            println!("响应: {}", response.raw_text);

            // 尝试解析JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response.raw_text) {
                println!("解析后的JSON: {}", json);
            } else {
                println!("⚠️ JSON解析失败，但LLM响应了内容");
            }
        }
        Err(e) => {
            println!("❌ 决策测试失败: {}", e);
        }
    }
}

#[tokio::test]
async fn test_local_llm_health_check() {
    // 基础健康检查
    let provider = OpenAiProvider::new(
        API_BASE.to_string(),
        "".to_string(),
        MODEL_ID.to_string(),
    ).with_timeout(30);

    let request = LlmRequest {
        prompt: "Say 'OK'".to_string(),
        max_tokens: 10,
        temperature: 0.1,
        response_format: agentora_ai::types::ResponseFormat::Text,
        stop_sequences: vec![],
    };

    let result = provider.generate(request).await;

    match result {
        Ok(response) => {
            println!("✅ 健康检查通过! LLM服务正常运行");
            println!("响应: {}", response.raw_text);
        }
        Err(e) => {
            println!("❌ 健康检查失败: {}", e);
            println!("请确认本地LLM服务正在端口1234运行");
            println!("Model: {}", MODEL_ID);
            println!("端点: {}v1/chat/completions", API_BASE);
        }
    }
}