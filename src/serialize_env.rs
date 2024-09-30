use anyhow::Error;
use arbitrary::Arbitrary;
use lexical::parse_partial;

#[derive(Debug, Clone, PartialEq, Arbitrary)]
pub enum EnvValue {
    String(String),
    Boolean(bool),
    Number(f64),
    Array(Vec<EnvValue>),
}

/// Encodes an `EnvValue` into its string representation.
pub fn encode_env(var: EnvValue) -> String {
    match var {
        EnvValue::String(s) => format!("\"{}\"", escape_string(&s)),
        EnvValue::Boolean(b) => b.to_string(),
        EnvValue::Number(n) => n.to_string(),
        EnvValue::Array(a) => format!(
            "[{}]",
            a.into_iter()
                .map(encode_env)
                .collect::<Vec<String>>()
                .join(",")
        ),
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
        Ok((rest, EnvValue::Boolean(true)))
    } else if let Some(rest) = input.strip_prefix("false") {
        Ok((rest, EnvValue::Boolean(false)))
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
            return Ok((rest, EnvValue::String(escaped)));
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

/// Internal parser that tries to parse any `EnvValue`.
fn parse_env_internal(input: &str) -> Result<(&str, EnvValue), Error> {
    let input = input.trim_start();
    if input.starts_with('"') {
        parse_string(input)
    } else if input.starts_with("true") || input.starts_with("false") {
        parse_boolean(input)
    } else if input.starts_with('[') {
        parse_array(input)
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
        let val = EnvValue::String("Hello, \"World\"!\n".to_string());
        assert_eq!(encode_env(val), "\"Hello, \\\"World\\\"!\\n\"");

        let val = EnvValue::Boolean(true);
        assert_eq!(encode_env(val), "true");

        let val = EnvValue::Number(42.0);
        assert_eq!(encode_env(val), "42");

        let val = EnvValue::Array(vec![
            EnvValue::String("Item1".to_string()),
            EnvValue::Boolean(false),
            EnvValue::Number(3.14),
        ]);
        assert_eq!(encode_env(val), "[\"Item1\",false,3.14]");
    }

    #[test]
    fn test_parse_env() {
        let input = "\"Hello, \\\"World\\\"!\\n\"";
        let expected = EnvValue::String("Hello, \"World\"!\n".to_string());
        assert_eq!(parse_env(input).unwrap(), expected);

        let input = "true";
        let expected = EnvValue::Boolean(true);
        assert_eq!(parse_env(input).unwrap(), expected);

        let input = "3.1415";
        let expected = EnvValue::Number(3.1415);
        assert_eq!(parse_env(input).unwrap(), expected);

        let input = "[\"Item1\", false, 42]";
        let expected = EnvValue::Array(vec![
            EnvValue::String("Item1".to_string()),
            EnvValue::Boolean(false),
            EnvValue::Number(42.0),
        ]);
        assert_eq!(parse_env(input).unwrap(), expected);
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
}
