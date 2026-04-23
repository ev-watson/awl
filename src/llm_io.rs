use std::fmt::Write as _;

pub fn strip_code_fences(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let after_tag = rest.find('\n').map_or(rest, |pos| &rest[pos + 1..]);
        let body = after_tag.strip_suffix("```").unwrap_or(after_tag);
        body.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn sanitize_json_strings(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_string = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_string {
            if ch == '\\' {
                result.push(ch);
                if let Some(escaped) = chars.next() {
                    result.push(escaped);
                }
            } else if ch == '"' {
                in_string = false;
                result.push(ch);
            } else if ch.is_control() {
                match ch {
                    '\n' => result.push_str("\\n"),
                    '\r' => result.push_str("\\r"),
                    '\t' => result.push_str("\\t"),
                    other => {
                        let _ = write!(result, "\\u{:04x}", other as u32);
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            if ch == '"' {
                in_string = true;
            }
            result.push(ch);
        }
    }

    result
}
