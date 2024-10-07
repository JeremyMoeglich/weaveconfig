use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

pub fn json_value_to_ts_type(value: &Value) -> String {
    json_value_to_ts_type_helper(value, 0)
}

fn json_value_to_ts_type_helper(value: &serde_json::Value, indent: usize) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(_) => "number".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Array(vec) => {
            if vec.is_empty() {
                "never[]".to_string()
            } else {
                let mut element_types_set = HashSet::new();
                for v in vec {
                    let t = json_value_to_ts_type_helper(v, indent);
                    element_types_set.insert(t);
                }

                let mut unique_types: Vec<String> = element_types_set.into_iter().collect();
                unique_types.sort();

                let element_type_str = if unique_types.len() == 1 {
                    unique_types[0].clone()
                } else {
                    format!("({})", unique_types.join(" | "))
                };

                format!("{}[]", element_type_str)
            }
        }
        Value::Object(map) => {
            let indent_str = "    ".repeat(indent);
            let inner_indent_str = "    ".repeat(indent + 1);

            if map.is_empty() {
                "Record<string, undefined>".to_string()
                // using never is unsound, undefined is more accurate
                // const a: Record<string, never> = {};
                // const b: { test: string } = { test: "test" };
                // const some_bool: boolean = false;

                // const c = some_bool ? a : b;

                // c.test // infered as string, should be string | undefined
            } else {
                let mut fields: Vec<String> = vec![];
                for (key, val) in map {
                    let field_type = json_value_to_ts_type_helper(val, indent + 1);
                    let formatted_key = format_ts_key(key);
                    fields.push(format!(
                        "{}{}: {};",
                        inner_indent_str, formatted_key, field_type
                    ));
                }
                let fields_str = fields.join("\n");
                format!("{{\n{}\n{}}}", fields_str, indent_str)
            }
        }
    }
}

/// Formats a key to be a valid TypeScript object key.
fn format_ts_key(key: &str) -> String {
    if is_valid_ts_identifier(key) {
        key.to_string()
    } else {
        format!("'{}'", key.replace("'", "\\'"))
    }
}

/// Checks if a string is a valid TypeScript identifier.
fn is_valid_ts_identifier(s: &str) -> bool {
    let re = Regex::new(r"^[A-Za-z_$][A-Za-z0-9_$]*$").unwrap();
    re.is_match(s)
}
