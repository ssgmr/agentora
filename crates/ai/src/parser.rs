//! 多层JSON兼容解析
//!
//! Layer 0: 清理markdown代码块标记
//! Layer 1: 直接解析
//! Layer 2: 提取{}块
//! Layer 3: 修复常见错误（尾逗号/注释/引号/加号前缀）
//! Layer 4: 激进修复（单引号/布尔值/多余内容截断）

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
    if let Ok(action) = serde_json::from_str::<Value>(&fixed) {
        return Ok(action);
    }
    if let Some(json_block) = extract_first_json_block(&fixed) {
        if let Ok(action) = serde_json::from_str::<Value>(json_block) {
            return Ok(action);
        }
    }

    // Layer 4: 激进修复
    let aggressive = aggressive_fix(&cleaned);
    if let Ok(action) = serde_json::from_str::<Value>(&aggressive) {
        return Ok(action);
    }
    if let Some(json_block) = extract_first_json_block(&aggressive) {
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

/// 提取第一个{}块（处理字符串内的{}}）
fn extract_first_json_block(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let remaining = &text[start..];
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (byte_offset, ch) in remaining.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }

        if ch == '"' && !in_string {
            in_string = true;
            continue;
        }

        if ch == '"' && in_string {
            in_string = false;
            continue;
        }

        if !in_string {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let end = start + byte_offset + ch.len_utf8();
                        return Some(&text[start..end]);
                    }
                }
                _ => {}
            }
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

    // 2. 移除行注释（// 和 # 开头的注释）
    // 需要按行处理，但要避免破坏字符串内的内容
    fixed = remove_line_comments(&fixed);

    // 3. 移除块注释 /* ... */
    while let Some(start) = fixed.find("/*") {
        if let Some(end) = fixed[start..].find("*/") {
            fixed.replace_range(start..start + end + 2, "");
        } else {
            break;
        }
    }

    // 4. 修复数字前的 + 号（+0.25 -> 0.25）
    fixed = fix_plus_prefix_numbers(&fixed);

    // 5. 替换单引号为双引号（简单处理，仅在非字符串内容中）
    fixed = fixed.replace("': '", "\": \"");
    fixed = fixed.replace("': ", "\": ");
    fixed = fixed.replace("',", "\",");

    fixed
}

/// 按行移除注释，但保留字符串内的内容
fn remove_line_comments(text: &str) -> String {
    let mut result = String::new();
    for line in text.lines() {
        let trimmed = remove_comment_from_line(line);
        result.push_str(&trimmed);
        result.push('\n');
    }
    // 移除末尾多余换行
    result.trim_end().to_string()
}

/// 从单行中移除注释（处理 // 和 # 注释）
fn remove_comment_from_line(line: &str) -> String {
    let mut in_string = false;
    let mut string_char = '\0';
    let mut escape_next = false;
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let ch = chars[i];

        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }

        if (ch == '"' || ch == '\'') && !in_string {
            in_string = true;
            string_char = ch;
            continue;
        }

        if in_string && ch == string_char {
            in_string = false;
            continue;
        }

        // 不在字符串中，检查注释标记
        if !in_string && ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            // 找到 // 注释，截断
            return chars[..i].iter().collect::<String>().trim_end().to_string();
        }

        if !in_string && ch == '#' {
            // 找到 # 注释，截断
            return chars[..i].iter().collect::<String>().trim_end().to_string();
        }
    }

    line.to_string()
}

/// 修复数字前的 + 号
fn fix_plus_prefix_numbers(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // 检查是否是数字前的 + 号（前面是 [, , 或空白）
        if chars[i] == '+' && i + 1 < len && (chars[i + 1].is_ascii_digit() || chars[i + 1] == '.') {
            // 检查前面字符是否是允许的上下文
            let prev_ok = if i == 0 {
                true
            } else {
                let prev = chars[i - 1];
                prev == '[' || prev == ',' || prev.is_whitespace() || prev == ':'
            };
            if prev_ok {
                // 跳过 + 号
                i += 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

/// 激进修复：处理更复杂的格式问题
fn aggressive_fix(text: &str) -> String {
    let mut fixed = fix_common_json_errors(text);

    // 尝试提取 JSON 块后再修复
    if let Some(block) = extract_first_json_block(&fixed) {
        fixed = block.to_string();
    }

    // 替换 True/False 为 true/false
    fixed = fixed.replace(": True", ": true");
    fixed = fixed.replace(": False", ": false");
    fixed = fixed.replace(": True,", ": true,");
    fixed = fixed.replace(": False,", ": false,");

    // 替换 None 为 null
    fixed = fixed.replace(": None", ": null");
    fixed = fixed.replace(": None,", ": null,");

    // 处理未转义的控制字符（LLM 可能在字符串中输出裸换行）
    fixed = fixed.replace("\\n\\n", "\\n");

    // 移除 JSON 块后的多余内容（如果 extract_first_json_block 没完全截断）
    if let Some(block) = extract_first_json_block(&fixed) {
        return block.to_string();
    }

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

    // gemma-4 兼容性测试

    #[test]
    fn test_gemma_comment_in_array() {
        // gemma 会在数组元素后添加 // 注释
        let input = r#"{
  "action_type": "Talk",
  "params": {
    "scores": [
      0.0,
      0.25,  // 社交分数
      0.0
    ]
  }
}"#;
        let result = parse_action_json(input);
        assert!(result.is_ok(), "应能解析带注释的数组: {:?}", result);
    }

    #[test]
    fn test_gemma_plus_prefix_number() {
        // gemma 会在正数前加 + 号
        let input = r#"{
  "action_type": "Talk",
  "params": {
    "scores": [
      0.0,
      +0.25,
      +0.10,
      0.0
    ]
  }
}"#;
        let result = parse_action_json(input);
        assert!(result.is_ok(), "应能解析带+号前缀的数字: {:?}", result);
    }

    #[test]
    fn test_gemma_comment_plus_combined() {
        // gemma 同时使用注释和+号
        let input = r#"{
  "action_type": "Talk",
  "params": {
    "scores": [
      0.0,
      +0.25,  // 社交分数
      0.0,
      +0.10   // 认知分数
    ]
  }
}"#;
        let result = parse_action_json(input);
        assert!(result.is_ok(), "应能同时处理注释和+号: {:?}", result);
    }

    #[test]
    fn test_gemma_markdown_with_comments() {
        // gemma 完整输出格式
        let input = r#"```json
{
  "reasoning": "社交需求最重要",
  "action_type": "Talk",
  "params": {
    "scores": [
      0.0,
      +0.25,  // social
      0.0
    ]
  }
}
```"#;
        let result = parse_action_json(input);
        assert!(result.is_ok(), "应能解析 markdown + 注释 + +号: {:?}", result);
    }

    #[test]
    fn test_remove_comment_from_line() {
        assert_eq!(remove_comment_from_line("hello // world"), "hello");
        assert_eq!(remove_comment_from_line("\"hello // world\""), "\"hello // world\"");
        assert_eq!(remove_comment_from_line("value,  // comment"), "value,");
        assert_eq!(remove_comment_from_line("  +0.25,  // 增加"), "  +0.25,");
    }

    #[test]
    fn test_fix_plus_prefix_numbers() {
        assert_eq!(fix_plus_prefix_numbers("+0.25"), "0.25");
        assert_eq!(fix_plus_prefix_numbers("[+0.25, +0.10]"), "[0.25, 0.10]");
        assert_eq!(fix_plus_prefix_numbers(":+0.25,"), ":0.25,");
        // 不修改字符串内的+号
        assert_eq!(fix_plus_prefix_numbers("\"+test\""), "\"+test\"");
    }
}