use anyhow::Error;
use arbitrary::Arbitrary;
use lexical::parse_partial;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Arbitrary)]
pub enum EnvValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    Array(Vec<EnvValue>),
    Object(BTreeMap<String, EnvValue>),
}

/// Encodes an `EnvValue` into its string representation.
pub fn encode_env(var: EnvValue) -> String {
    match var {
        EnvValue::String(s) => format!("\"{}\"", escape_string(&s)),
        EnvValue::Bool(b) => b.to_string(),
        EnvValue::Number(n) => n.to_string(),
        EnvValue::Null => "null".to_string(),
        EnvValue::Array(a) => {
            let array_str = a
                .into_iter()
                .map(serialize_value)
                .collect::<Vec<String>>()
                .join(",");
            let escaped_array_str = escape_string(&format!("[{}]", array_str));
            format!("\"{}\"", escaped_array_str)
        }
        EnvValue::Object(o) => {
            let object_str = o
                .into_iter()
                .map(|(k, v)| {
                    let escaped_key = escape_string(&k);
                    let serialized_value = serialize_value(v);
                    format!("\"{}\":{}", escaped_key, serialized_value)
                })
                .collect::<Vec<String>>()
                .join(",");
            let escaped_object_str = escape_string(&format!("{{{}}}", object_str));
            format!("\"{}\"", escaped_object_str)
        }
    }
}

/// Helper function to serialize an EnvValue for arrays and objects.
fn serialize_value(var: EnvValue) -> String {
    match var {
        EnvValue::String(s) => format!("\"{}\"", escape_string(&s)),
        EnvValue::Bool(b) => b.to_string(),
        EnvValue::Number(n) => n.to_string(),
        EnvValue::Null => "null".to_string(),
        EnvValue::Array(a) => {
            let array_str = a
                .into_iter()
                .map(serialize_value)
                .collect::<Vec<String>>()
                .join(",");
            format!("[{}]", array_str)
        }
        EnvValue::Object(o) => {
            let object_str = o
                .into_iter()
                .map(|(k, v)| {
                    let escaped_key = escape_string(&k);
                    let serialized_value = serialize_value(v);
                    format!("\"{}\":{}", escaped_key, serialized_value)
                })
                .collect::<Vec<String>>()
                .join(",");
            format!("{{{}}}", object_str)
        }
    }
}

/// Parses a string into an `EnvValue`.
pub fn parse_env(input: &str) -> Result<EnvValue, Error> {
    let (rest, value) = parse_env_internal(input)?;
    if !rest.trim().is_empty() {
        Err(anyhow::anyhow!("Unexpected trailing input: {}", rest))
    } else {
        Ok(value)
    }
}

/// Parses a boolean value (`true` or `false`).
fn parse_boolean(input: &str) -> Result<(&str, EnvValue), Error> {
    let input = input.trim_start();
    if let Some(rest) = input.strip_prefix("true") {
        Ok((rest, EnvValue::Bool(true)))
    } else if let Some(rest) = input.strip_prefix("false") {
        Ok((rest, EnvValue::Bool(false)))
    } else {
        Err(anyhow::anyhow!("Expected 'true' or 'false'"))
    }
}

/// Parses a floating-point number.
fn parse_number(input: &str) -> Result<(&str, EnvValue), Error> {
    let input = input.trim_start();
    let input_bytes = input.as_bytes();

    match parse_partial(input_bytes) {
        Ok((num, count)) => {
            let rest = &input[count..];
            Ok((rest, EnvValue::Number(num)))
        }
        Err(e) => Err(anyhow::anyhow!("Failed to parse number: {}", e)),
    }
}

/// Parses a string with escape sequences.
fn parse_string(input: &str) -> Result<(&str, EnvValue), Error> {
    let input = input.trim_start();
    if !input.starts_with('"') {
        return Err(anyhow::anyhow!("Expected '\"' at start of string"));
    }

    let mut escaped = String::new();
    let mut chars = input[1..].char_indices();
    let mut escaped_char = false;

    while let Some((idx, c)) = chars.next() {
        if escaped_char {
            let esc_c = match c {
                '\\' => '\\',
                '"' => '"',
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '0' => '\0',
                _ => return Err(anyhow::anyhow!("Invalid escape sequence: \\{}", c)),
            };
            escaped.push(esc_c);
            escaped_char = false;
        } else if c == '\\' {
            escaped_char = true;
        } else if c == '"' {
            // End of string
            let consumed = idx + c.len_utf8() + 1; // +1 for the initial quote
            let rest = &input[consumed..];

            // Now, process the escaped string
            let unescaped_string = escaped;

            // Check for special cases
            if unescaped_string.starts_with('\\') {
                // Starts with '\', treat as raw string
                return Ok((rest, EnvValue::String(unescaped_string)));
            } else if unescaped_string.starts_with('[') || unescaped_string.starts_with('{') {
                // Attempt to parse the unescaped string as an array or object
                match parse_env_internal(&unescaped_string) {
                    Ok((_rest_inner, value)) => {
                        // If parsing succeeds, return the value
                        return Ok((rest, value));
                    }
                    Err(_e) => {
                        // If parsing fails, treat it as a string
                        return Ok((rest, EnvValue::String(unescaped_string)));
                    }
                }
            } else {
                // Not starting with '[' or '{', treat as string
                return Ok((rest, EnvValue::String(unescaped_string)));
            }
        } else {
            escaped.push(c);
        }
    }

    Err(anyhow::anyhow!("Unterminated string"))
}

/// Parses an array of `EnvValue`s.
fn parse_array(input: &str) -> Result<(&str, EnvValue), Error> {
    let mut rest = input.trim_start();
    if !rest.starts_with('[') {
        return Err(anyhow::anyhow!("Expected '[' at start of array"));
    }
    rest = &rest[1..]; // Skip '['
    let mut elements = Vec::new();

    loop {
        rest = rest.trim_start();
        if rest.starts_with(']') {
            rest = &rest[1..]; // Skip ']'
            return Ok((rest, EnvValue::Array(elements)));
        }
        let (new_rest, elem) = parse_env_internal(rest)?;
        elements.push(elem);
        rest = new_rest.trim_start();
        if rest.starts_with(',') {
            rest = &rest[1..]; // Skip ','
        } else if rest.starts_with(']') {
            continue;
        } else {
            return Err(anyhow::anyhow!("Expected ',' or ']' in array"));
        }
    }
}

fn parse_null(input: &str) -> Result<(&str, EnvValue), Error> {
    let input = input.trim_start();
    if let Some(rest) = input.strip_prefix("null") {
        Ok((rest, EnvValue::Null))
    } else {
        Err(anyhow::anyhow!("Expected 'null'"))
    }
}

fn parse_object(input: &str) -> Result<(&str, EnvValue), Error> {
    let mut rest = input.trim_start();
    if !rest.starts_with('{') {
        return Err(anyhow::anyhow!("Expected '{{' at start of object"));
    }
    rest = &rest[1..]; // Skip '{'
    let mut map = BTreeMap::new();

    loop {
        rest = rest.trim_start();
        if rest.starts_with('}') {
            rest = &rest[1..]; // Skip '}'
            return Ok((rest, EnvValue::Object(map)));
        }

        // Parse key
        let (new_rest, key_value) = parse_string(rest)?;
        let key = match key_value {
            EnvValue::String(s) => s,
            _ => return Err(anyhow::anyhow!("Object keys must be strings")),
        };
        rest = new_rest.trim_start();

        // Expect ':'
        if !rest.starts_with(':') {
            return Err(anyhow::anyhow!("Expected ':' after key in object"));
        }
        rest = &rest[1..]; // Skip ':'

        // Parse value
        let (new_rest, value) = parse_env_internal(rest)?;
        map.insert(key, value);
        rest = new_rest.trim_start();

        // Check for ',' or '}'
        if rest.starts_with(',') {
            rest = &rest[1..]; // Skip ','
        } else if rest.starts_with('}') {
            rest = &rest[1..]; // Skip '}'
            return Ok((rest, EnvValue::Object(map)));
        } else {
            return Err(anyhow::anyhow!("Expected ',' or '}}' in object"));
        }
    }
}

fn parse_env_internal(input: &str) -> Result<(&str, EnvValue), Error> {
    let input = input.trim_start();
    if input.starts_with('"') {
        parse_string(input)
    } else if input.starts_with("true") || input.starts_with("false") {
        parse_boolean(input)
    } else if input.starts_with("null") {
        parse_null(input)
    } else if input.starts_with('[') {
        parse_array(input)
    } else if input.starts_with('{') {
        parse_object(input)
    } else {
        parse_number(input)
    }
}

/// Escapes special characters in a string.
fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\t' => result.push_str("\\t"),
            '\r' => result.push_str("\\r"),
            '\0' => result.push_str("\\0"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_env() {
        let val = EnvValue::Null;
        assert_eq!(encode_env(val), "null");

        let val = EnvValue::Object(
            vec![
                ("key1".to_string(), EnvValue::String("value1".to_string())),
                ("key2".to_string(), EnvValue::Number(42.0)),
                ("key3".to_string(), EnvValue::Bool(true)),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            encode_env(val),
            "\"{\\\"key1\\\":\\\"value1\\\",\\\"key2\\\":42,\\\"key3\\\":true}\""
        );
    }

    #[test]
    fn test_parse_env() {
        let input = "null";
        let expected = EnvValue::Null;
        assert_eq!(parse_env(input).unwrap(), expected);

        let input = "\"{\\\"key1\\\": \\\"value1\\\", \\\"key2\\\": 42, \\\"key3\\\": true}\"";
        let expected = EnvValue::Object(
            vec![
                ("key1".to_string(), EnvValue::String("value1".to_string())),
                ("key2".to_string(), EnvValue::Number(42.0)),
                ("key3".to_string(), EnvValue::Bool(true)),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(parse_env(input).unwrap(), expected);
    }

    #[test]
    fn test_parse_and_encode_env() {
        let input = "\"{\\\"key1\\\": \\\"value1\\\", \\\"key2\\\": 42, \\\"key3\\\": true}\"";
        let expected = EnvValue::Object(
            vec![
                ("key1".to_string(), EnvValue::String("value1".to_string())),
                ("key2".to_string(), EnvValue::Number(42.0)),
                ("key3".to_string(), EnvValue::Bool(true)),
            ]
            .into_iter()
            .collect(),
        );
        let parsed = parse_env(input).unwrap();
        assert_eq!(parsed, expected);
        let encoded = encode_env(parsed);
        assert_eq!(
            encoded,
            "\"{\\\"key1\\\":\\\"value1\\\",\\\"key2\\\":42,\\\"key3\\\":true}\""
        );
    }

    #[test]
    fn test_escape_string() {
        let input = "Hello, \"World\"!\n";
        let expected = "Hello, \\\"World\\\"!\\n".to_string();
        assert_eq!(escape_string(input), expected);
    }

    #[test]
    fn test_empty_input() {
        let input = "\"\"";
        let expected = EnvValue::String("".to_string());
        assert_eq!(parse_env(input).unwrap(), expected);
    }

    #[test]
    fn test_nan() {
        let input = r#"[true,NaN]"#;
        parse_env(input).unwrap();
    }

    #[test]
    fn test_integer() {
        let input = "42";
        let expected = EnvValue::Number(42.0);
        assert_eq!(parse_env(input).unwrap(), expected);
    }

    #[test]
    fn test_parse_array_of_strings() {
        let input = "\"[\\\"test\\\",5,2]\"";
        let expected = EnvValue::Array(vec![
            EnvValue::String("test".to_string()),
            EnvValue::Number(5.0),
            EnvValue::Number(2.0),
        ]);
        let parsed = parse_env(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_raw_string() {
        let input = "\"\\\\[\\\"test\\\",5,2]\"";
        let expected = EnvValue::String("\\[\"test\",5,2]".to_string());
        let parsed = parse_env(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_nested_array() {
        let input = "\"[[1,2]]\"";
        let expected = EnvValue::Array(vec![EnvValue::Array(vec![
            EnvValue::Number(1.0),
            EnvValue::Number(2.0),
        ])]);
        let parsed = parse_env(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_string_with_brackets() {
        let input = "\"1,2,[3]\"";
        let expected = EnvValue::String("1,2,[3]".to_string());
        let parsed = parse_env(input).unwrap();
        assert_eq!(parsed, expected);
    }
}
