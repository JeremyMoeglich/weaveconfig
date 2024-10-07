mod integer;
mod segment;

use integer::parse_integer;
use segment::{parse_segment, ParseSegmentError};
use serde_json::{Map, Value};
use thiserror::Error;

/// Enum representing possible errors during template rendering.
#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("{0}")]
    VariableError(VariableError),
    #[error("Syntax error: {0}")]
    SyntaxError(String),
}

#[derive(Debug, Error)]
pub enum VariableError {
    #[error("Missing variable: {0}")]
    MissingVariable(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Index out of bounds, tried to access {0} but array length is {1}")]
    IndexOutOfBounds(usize, usize),
    #[error("Invalid type, expected {0}, got {1}")]
    InvalidType(String, String),
}

pub fn value_type(value: &Value) -> String {
    match value {
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Null => "null",
    }
    .to_string()
}

fn render_variable(
    variable: &Variable,
    variables: &Map<String, Value>,
) -> Result<String, VariableError> {
    let mut value = variables
        .get(&variable.base)
        .ok_or(VariableError::MissingVariable(variable.base.clone()))?;

    for modifier in variable.modifiers.iter() {
        match modifier {
            Modifier::Index(index) => {
                value = match value {
                    Value::Array(array) => array.get(*index as usize).ok_or_else(|| {
                        VariableError::IndexOutOfBounds(*index as usize, array.len())
                    })?,
                    _ => {
                        return Err(VariableError::InvalidType(
                            "array".to_string(),
                            value_type(value),
                        ))
                    }
                }
            }
            Modifier::Key(key) => {
                value = match value {
                    Value::Object(object) => object
                        .get(key)
                        .ok_or(VariableError::KeyNotFound(key.clone()))?,
                    _ => {
                        return Err(VariableError::InvalidType(
                            "object".to_string(),
                            value_type(value),
                        ))
                    }
                }
            }
        }
    }

    Ok(match value {
        Value::String(s) => s.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap(),
    })
}

pub fn template_file(
    content: &str,
    variables: &Map<String, Value>,
) -> Result<String, TemplateError> {
    enum State {
        Text,
        Brace,
        Escape,
        DoubleEscape,
        EscapeBrace1,
        VariableEnd1,
        VariableEnd2,
    }

    let mut state = State::Text;
    let mut output = String::new();
    let mut input = content;

    while let Some((char, rest)) = take_first(input) {
        input = rest;
        match state {
            State::Text => match char {
                '{' => {
                    state = State::Brace;
                }
                '\\' => {
                    state = State::Escape;
                }
                _ => output.push(char),
            },
            State::Brace => match char {
                '{' => {
                    let rest = strip_whitespace_left(input);
                    let (var, rest) = parse_variable(rest)?;
                    input = rest;
                    output.push_str(
                        &render_variable(&var, variables)
                            .map_err(|e| TemplateError::VariableError(e))?,
                    );
                    state = State::VariableEnd1;
                }
                '\\' => {
                    output.push('{');
                    state = State::Escape;
                }
                _ => {
                    state = State::Text;
                    output.push('{');
                    output.push(char);
                }
            },
            State::Escape => match char {
                '{' => {
                    state = State::EscapeBrace1;
                }
                '\\' => {
                    state = State::DoubleEscape;
                }
                _ => {
                    output.push('\\');
                    output.push(char);
                    state = State::Text;
                }
            },
            State::DoubleEscape => match char {
                '{' => {
                    output.push('\\');
                    state = State::Brace;
                }
                _ => {
                    output.push('\\');
                    output.push('\\');
                    output.push(char);
                    state = State::Text;
                }
            },
            State::EscapeBrace1 => match char {
                '{' => {
                    output.push('{');
                    output.push('{');
                    state = State::Text;
                }
                _ => {
                    output.push('\\');
                    output.push('{');
                    output.push(char);
                    state = State::Text;
                }
            },
            State::VariableEnd1 => match char {
                '}' => {
                    state = State::VariableEnd2;
                }
                char if char.is_whitespace() => {}
                _ => {
                    return Err(TemplateError::SyntaxError(format!(
                        "Unexpected character: {}",
                        char
                    )));
                }
            },
            State::VariableEnd2 => match char {
                '}' => {
                    state = State::Text;
                }
                char if char.is_whitespace() => {}
                _ => {
                    return Err(TemplateError::SyntaxError(format!(
                        "Unexpected character: {}",
                        char
                    )));
                }
            },
        }
    }

    match state {
        State::Text => Ok(output),
        State::Brace => {
            output.push('{');
            Ok(output)
        }
        State::Escape => {
            output.push('\\');
            Ok(output)
        }
        State::DoubleEscape => {
            output.push('\\');
            output.push('\\');
            Ok(output)
        }
        State::EscapeBrace1 => {
            output.push('\\');
            output.push('{');
            Ok(output)
        }
        State::VariableEnd1 | State::VariableEnd2 => {
            Err(TemplateError::SyntaxError("Unclosed variable".to_string()))
        }
    }
}

fn take_first(span: &str) -> Option<(char, &str)> {
    let mut chars = span.chars();
    let first = chars.next()?;
    let rest = chars.as_str();
    Some((first, rest))
}

fn strip_whitespace_left(span: &str) -> &str {
    let index = span.find(|c: char| !c.is_whitespace());
    match index {
        Some(index) => &span[index..],
        None => "",
    }
}

struct Variable {
    base: String,
    modifiers: Vec<Modifier>,
}

enum Modifier {
    Index(u64),
    Key(String),
}

fn parse_variable(input: &str) -> Result<(Variable, &str), TemplateError> {
    let (segment, input) = parse_segment(input).map_err(|e| match e {
        ParseSegmentError::UnclosedQuote => {
            TemplateError::SyntaxError("Unclosed quote".to_string())
        }
        ParseSegmentError::NoSegment => TemplateError::SyntaxError("Missing segment".to_string()),
    })?;
    let (modifiers, input) = parse_modifiers(input)?;
    Ok((
        Variable {
            base: segment,
            modifiers,
        },
        input,
    ))
}

fn parse_segment_template(input: &str) -> Result<(String, &str), TemplateError> {
    parse_segment(input).map_err(|e| match e {
        ParseSegmentError::UnclosedQuote => {
            TemplateError::SyntaxError("Unclosed quote".to_string())
        }
        ParseSegmentError::NoSegment => TemplateError::SyntaxError("Missing segment".to_string()),
    })
}

fn parse_modifiers(input: &str) -> Result<(Vec<Modifier>, &str), TemplateError> {
    let mut modifiers = Vec::new();
    let mut span = input;

    while let Ok((modifier, rest)) = parse_modifier(span) {
        modifiers.push(modifier);
        span = rest;
    }

    Ok((modifiers, span))
}

fn parse_modifier(input: &str) -> Result<(Modifier, &str), TemplateError> {
    match take_first(input) {
        Some(('.', input)) => {
            let (segment, input) = parse_segment_template(input)?;
            Ok((Modifier::Key(segment), input))
        }
        Some(('[', input)) => {
            let (modifier, input) = parse_access(input)?;
            if let Some((char, input)) = take_first(input) {
                if char == ']' {
                    Ok((modifier, input))
                } else {
                    Err(TemplateError::SyntaxError(format!(
                        "Unexpected character: {}",
                        char
                    )))
                }
            } else {
                Err(TemplateError::SyntaxError("Unexpected EOF".to_string()))
            }
        }
        Some((char, _)) => Err(TemplateError::SyntaxError(format!(
            "Unexpected character: {}",
            char
        ))),
        None => Err(TemplateError::SyntaxError("Unexpected EOF".to_string())),
    }
}

fn parse_access(input: &str) -> Result<(Modifier, &str), TemplateError> {
    match parse_integer(input) {
        Ok((index, input)) => {
            if index < 0 {
                Err(TemplateError::SyntaxError("Negative index".to_string()))
            } else {
                Ok((Modifier::Index(index as u64), input))
            }
        }
        Err(_) => {
            let (segment, input) = parse_segment_template(input)?;
            Ok((Modifier::Key(segment), input))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serde_json::Map;

    fn map(vars: &[(&str, Value)]) -> Map<String, Value> {
        vars.iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn test_variable_interpolation() {
        let content = "Hello, {{ user.name }}!";
        let variables = map(&[("user", json!({"name": "Alice"}))]);

        assert_eq!(template_file(content, &variables).unwrap(), "Hello, Alice!");
    }

    #[test]
    fn test_nested_variable_interpolation() {
        let content = "User age: {{ user.details.age }}";
        let variables = map(&[("user", json!({"details": {"age": 30}}))]);

        assert_eq!(template_file(content, &variables).unwrap(), "User age: 30");
    }

    #[test]
    fn test_array_indexing() {
        let content = "First item: {{ items[0] }}";
        let variables = map(&[("items", json!(["apple", "banana"]))]);

        assert_eq!(
            template_file(content, &variables).unwrap(),
            "First item: apple"
        );
    }

    #[test]
    fn test_complex_key() {
        let content = "Complex key: {{ object[\"complex key\"] }}";
        let variables = map(&[("object", json!({"complex key": "value"}))]);

        assert_eq!(
            template_file(content, &variables).unwrap(),
            "Complex key: value"
        );
    }

    #[test]
    fn test_escaped_double_braces() {
        let content = "\\{{ not_a_variable }}";
        let variables = map(&[]);

        assert_eq!(
            template_file(content, &variables).unwrap(),
            "{{ not_a_variable }}"
        );
    }

    #[test]
    fn test_literal_backslash() {
        let content = "A literal backslash: \\";
        let variables = map(&[]);

        assert_eq!(
            template_file(content, &variables).unwrap(),
            "A literal backslash: \\"
        );
    }

    #[test]
    fn test_unescaped_single_brace() {
        let content = "Something { with single braces }";
        let variables = map(&[]);

        assert_eq!(
            template_file(content, &variables).unwrap(),
            "Something { with single braces }"
        );
    }

    #[test]
    fn test_unclosed_variable() {
        let content = "Unclosed {{ variable";
        let variables = map(&[]);

        assert!(template_file(content, &variables).is_err());
    }

    #[test]
    fn test_invalid_array_index() {
        let content = "Invalid index: {{ array[-1] }}";
        let variables = map(&[("array", json!([1, 2, 3]))]);

        assert!(template_file(content, &variables).is_err());
    }

    #[test]
    fn test_text_with_no_variable() {
        let content = "Just plain text.";
        let variables = map(&[]);

        assert_eq!(
            template_file(content, &variables).unwrap(),
            "Just plain text."
        );
    }

    #[test]
    fn test_missing_variable_error() {
        let content = "Missing variable: {{ missing_var }}";
        let variables = map(&[("present_var", json!("exists"))]);

        assert!(matches!(
            template_file(content, &variables).unwrap_err(),
            TemplateError::VariableError(VariableError::MissingVariable(_))
        ));
    }

    #[test]
    fn test_type_error_on_indexing_non_array() {
        let content = "Invalid access: {{ object[0] }}";
        let variables = map(&[("object", json!({"key": "value"}))]);

        assert!(matches!(
            template_file(content, &variables).unwrap_err(),
            TemplateError::VariableError(VariableError::InvalidType(_, _))
        ));
    }

    #[test]
    fn test_escaped_variable() {
        let content = "\\\\{{ some_var }}";
        let variables = map(&[("some_var", json!("some_value"))]);

        assert_eq!(template_file(content, &variables).unwrap(), "\\some_value");
    }

    #[test]
    fn test_double_escape() {
        let content = "  \\\\";
        let variables = map(&[]);

        assert_eq!(template_file(content, &variables).unwrap(), "  \\\\");
    }
}
