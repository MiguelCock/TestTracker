use serde_json::Value;

/// Extracts the first complete JSON object from model output.
///
/// Local models often wrap JSON in fences or add prose before or after it, so
/// the slice is taken by walking braces instead of trusting the first and last
/// brace in the reply. Braces inside JSON strings are ignored.
pub fn extract_json_object(content: &str) -> Option<&str> {
    let trimmed = content.trim();
    let start = trimmed.find('{')?;
    let bytes = trimmed.as_bytes();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, byte) in bytes[start..].iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return trimmed.get(start..=start + offset);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn parse_json_object(content: &str) -> Option<Value> {
    let slice = extract_json_object(content)?;
    serde_json::from_str(slice).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_object_from_surrounding_prose() {
        let content = "Next action: list_tests\n{\"tool\":\"list_tests\",\"arguments\":{}}\nDone.";
        let value = parse_json_object(content).expect("object");
        assert_eq!(value["tool"], "list_tests");
    }

    #[test]
    fn extracts_object_from_code_fence() {
        let content = "```json\n{\"message\":\"ok\"}\n```";
        let value = parse_json_object(content).expect("object");
        assert_eq!(value["message"], "ok");
    }

    #[test]
    fn extracts_nested_object_and_arrays() {
        let content = "{\"a\":{\"b\":[1,2,{\"c\":3}]},\"d\":true}";
        let value = parse_json_object(content).expect("object");
        assert_eq!(value["a"]["b"][2]["c"], 3);
    }

    #[test]
    fn ignores_braces_inside_strings() {
        let content = "{\"message\":\"use {braces} and \\\"quotes\\\" inside\"}";
        let value = parse_json_object(content).expect("object");
        assert_eq!(value["message"], "use {braces} and \"quotes\" inside");
    }

    #[test]
    fn returns_first_complete_object() {
        let content = "{\"first\":1}\n{\"second\":2}";
        let value = parse_json_object(content).expect("object");
        assert_eq!(value["first"], 1);
        assert!(value.get("second").is_none());
    }

    #[test]
    fn returns_none_for_prose_only() {
        assert!(parse_json_object("Next action: list_tests").is_none());
        assert!(parse_json_object("{\"unterminated\": true").is_none());
    }
}
