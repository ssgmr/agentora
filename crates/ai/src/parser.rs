//! 多层JSON兼容解析
//!
//! Layer 0: 清理markdown代码块标记
//! Layer 1: 直接解析
//! Layer 2: 提取{}块
//! Layer 3: 修复常见错误

use serde_json::Value;

/// 解析JSON动作
pub fn parse_action_json(raw: &str) -> Result<Value, ParseError> {
    // Layer 0: 清理markdown代码块标记
    let cleaned = strip_markdown_code_block(raw);

    // Layer 1: 直接解析
    if let Ok(action) = serde_json::from_str::<Value>(&cleaned) {
        return Ok(action);
    }

    // Layer 2: 提取{}块
    if let Some(json_block) = extract_first_json_block(&cleaned) {
        if let Ok(action) = serde_json::from_str::<Value>(json_block) {
            return Ok(action);
        }
    }

    // Layer 3: 修复常见错误
    let fixed = fix_common_json_errors(&cleaned);
    if let Some(json_block) = extract_first_json_block(&fixed) {
        if let Ok(action) = serde_json::from_str::<Value>(json_block) {
            return Ok(action);
        }
    }

    Err(ParseError::InvalidJson)
}

/// 清理markdown代码块标记
/// 处理 ```json ... ``` 或 ``` ... ``` 格式
fn strip_markdown_code_block(text: &str) -> String {
    let mut result = text.to_string();

    // 处理 ```json ... ``` 或 ``` ... ``` 格式
    // 查找开始标记
    if let Some(start_idx) = result.find("```") {
        // 找到开始标记后的换行符
        let content_start = result[start_idx..].find('\n')
            .map(|i| start_idx + i + 1)
            .unwrap_or(start_idx + 3);

        // 查找结束标记 ```
        if let Some(end_search_start) = result[content_start..].find("```") {
            let end_idx = content_start + end_search_start;
            // 提取纯JSON内容
            result = result[content_start..end_idx].to_string();
        } else {
            // 没有结束标记，只去掉开始部分
            result = result[content_start..].to_string();
        }
    }

    result.trim().to_string()
}

/// 提取第一个{}块
fn extract_first_json_block(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0;
    for (i, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..start + i + 1]);
                }
            }
            _ => {}
        }
    }
    None
}

/// 修复常见JSON错误
fn fix_common_json_errors(text: &str) -> String {
    let mut fixed = text.to_string();

    // 1. 移除尾逗号
    fixed = fixed.replace(",}", "}");
    fixed = fixed.replace(",]", "]");

    // 2. 替换单引号为双引号（简单处理）
    // 注意：这可能会破坏合法的单引号字符串，但在JSON中不应该有单引号

    // 3. 移除注释（JSON不支持注释，但有时LLM会生成）
    // 移除 /* ... */ 注释
    while let Some(start) = fixed.find("/*") {
        if let Some(end) = fixed.find("*/") {
            fixed = fixed.replace(&fixed[start..end + 2], "");
        }
    }
    // 移除 // ... 行注释
    let lines: Vec<String> = fixed.lines().map(|line| {
        if let Some(comment_start) = line.find("//") {
            line[..comment_start].to_string()
        } else {
            line.to_string()
        }
    }).collect();
    fixed = lines.join("\n");

    fixed
}

/// 解析错误类型
#[derive(Debug, Clone)]
pub enum ParseError {
    InvalidJson,
    MissingField(String),
    InvalidFieldType(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidJson => write!(f, "无效的JSON格式"),
            ParseError::MissingField(field) => write!(f, "缺少字段: {}", field),
            ParseError::InvalidFieldType(field) => write!(f, "字段类型无效: {}", field),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_json_block() {
        let input = "```json\n{\"message\": \"test\"}\n```";
        let output = strip_markdown_code_block(input);
        assert_eq!(output, "{\"message\": \"test\"}");
    }

    #[test]
    fn test_strip_markdown_generic_block() {
        let input = "```\n{\"action\": \"move\"}\n```";
        let output = strip_markdown_code_block(input);
        assert_eq!(output, "{\"action\": \"move\"}");
    }

    #[test]
    fn test_parse_with_markdown() {
        let input = "```json\n{\"action_type\": \"explore\", \"target\": \"(150, 150)\"}\n```";
        let result = parse_action_json(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pure_json() {
        let input = "{\"action_type\": \"move\", \"target\": \"north\"}";
        let result = parse_action_json(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_with_trailing_comma() {
        let input = "{\"action\": \"move\", \"target\": \"north\",}";
        let result = parse_action_json(input);
        assert!(result.is_ok());
    }
}