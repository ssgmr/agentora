//! 单元测试 - JSON解析
//!
//! 测试多层降级解析、边界情况

use agentora_ai::parse_action_json;

#[test]
fn test_parse_valid_json() {
    let json = r#"{"reasoning": "测试", "action_type": "Move"}"#;
    let result = parse_action_json(json);

    assert!(result.is_ok());
}

#[test]
fn test_parse_embedded_json() {
    // LLM可能在JSON前添加文本
    let text = r#"好的，我的决策是：
{"reasoning": "测试", "action_type": "Move"}
这是我的完整响应"#;

    let result = parse_action_json(text);

    assert!(result.is_ok());
}

#[test]
fn test_parse_trailing_comma() {
    // LLM常见的错误：尾逗号
    let json = r#"{"reasoning": "测试", "action_type": "Move",}"#;

    let result = parse_action_json(json);

    // Layer 3应修复
    assert!(result.is_ok());
}

#[test]
fn test_parse_invalid_json() {
    let text = "这完全不是JSON";

    let result = parse_action_json(text);

    assert!(result.is_err());
}

#[test]
fn test_parse_empty_json() {
    let json = "{}";

    let result = parse_action_json(json);

    // 应能解析空对象
    assert!(result.is_ok());
}

#[test]
fn test_parse_nested_json() {
    let json = r#"{
        "reasoning": "复杂决策",
        "action_type": "TradeOffer",
        "params": {
            "offer": {"iron": 10},
            "want": {"food": 5}
        }
    }"#;

    let result = parse_action_json(json);

    assert!(result.is_ok());
}