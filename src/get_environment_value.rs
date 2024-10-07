use serde_json::{Map, Value};

use crate::template_file::value_type;

pub fn get_environment_value(
    variables: &Map<String, Value>,
    environment: &str,
) -> Result<Map<String, Value>, anyhow::Error> {
    let environment_variables = variables.get(environment).ok_or(anyhow::anyhow!(
        "Environment {} not found in variables, this is an internal error",
        environment
    ))?;
    let mut variables = variables.clone();
    if let Value::Object(environment_variables) = environment_variables {
        for (key, value) in environment_variables {
            variables.insert(key.clone(), value.clone());
        }
        return Ok(variables);
    }
    Err(anyhow::anyhow!(
        "Expected an object, got {}",
        value_type(environment_variables)
    ))
}
