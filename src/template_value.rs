use serde_json::Value;
use std::collections::BTreeMap;

use crate::{serialize_env::EnvValue, upper_snake_case::to_upper_snake_case};

fn serde_number_to_f64(value: serde_json::Number) -> f64 {
    value.as_f64().unwrap()
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    Array(Vec<TemplateValue>),
    Object(TemplateObject), // First key: defined name, Second key: environment variable template
}

pub type TemplateObject = BTreeMap<String, BTreeMap<String, TemplateValue>>;

impl TemplateValue {
    /// Converts a serde_json::Value into a TemplateValue.
    pub fn from_value(value: Value) -> TemplateValue {
        match value {
            Value::String(v) => TemplateValue::String(v),
            Value::Number(v) => TemplateValue::Number(serde_number_to_f64(v)),
            Value::Bool(v) => TemplateValue::Bool(v),
            Value::Null => TemplateValue::Null,
            Value::Array(v) => {
                TemplateValue::Array(v.into_iter().map(TemplateValue::from_value).collect())
            }
            Value::Object(v) => {
                let mut object = BTreeMap::new();
                for (k, v) in v {
                    let mut template_map = BTreeMap::new();
                    template_map.insert("{}".to_string(), TemplateValue::from_value(v));
                    object.insert(k, template_map);
                }
                TemplateValue::Object(object)
            }
        }
    }

    /// Applies the template to environment variable keys only.
    pub fn apply_template(&mut self, template: &str) {
        if let TemplateValue::Object(obj) = self {
            for (_name, template_map) in obj.iter_mut() {
                let mut new_template_map = BTreeMap::new();
                for (tmpl, value) in template_map.iter_mut() {
                    let new_tmpl = template.replace("{}", tmpl);
                    new_template_map.insert(new_tmpl, value.clone());
                }
                *template_map = new_template_map;
            }
        }
        // For other types, the function does nothing as templates don't apply
    }

    /// Merges another TemplateValue into the current one.
    /// Returns a Result to handle conflicts.
    pub fn merge_into(&mut self, other: &TemplateValue) -> Result<(), anyhow::Error> {
        match (self, other) {
            (TemplateValue::Object(obj1), TemplateValue::Object(obj2)) => obj1.merge_into(obj2),
            (TemplateValue::Array(arr1), TemplateValue::Array(arr2)) => {
                arr1.extend(arr2.clone());
                Ok(())
            }
            _ => Err(anyhow::anyhow!(
                "Merge is only supported for objects and arrays."
            )),
        }
    }

    fn to_env_records(self) -> Result<TemplateValueEnv, anyhow::Error> {
        match self {
            TemplateValue::Null => Ok(TemplateValueEnv::Value(EnvValue::Null)),
            TemplateValue::Bool(b) => Ok(TemplateValueEnv::Value(EnvValue::Bool(b))),
            TemplateValue::Number(n) => Ok(TemplateValueEnv::Value(EnvValue::Number(n))),
            TemplateValue::String(s) => Ok(TemplateValueEnv::Value(EnvValue::String(s))),
            TemplateValue::Array(a) => {
                let mut env_array = Vec::new();
                for v in a {
                    env_array.push(v.to_inner_env_value()?);
                }
                Ok(TemplateValueEnv::Value(EnvValue::Array(env_array)))
            }
            TemplateValue::Object(o) => Ok(TemplateValueEnv::Object(o.to_env_records()?)),
        }
    }

    fn to_inner_env_value(self) -> Result<EnvValue, anyhow::Error> {
        match self {
            TemplateValue::Null => Ok(EnvValue::Null),
            TemplateValue::Bool(b) => Ok(EnvValue::Bool(b)),
            TemplateValue::Number(n) => Ok(EnvValue::Number(n)),
            TemplateValue::String(s) => Ok(EnvValue::String(s)),
            TemplateValue::Array(a) => {
                let mut env_array = Vec::new();
                for v in a {
                    env_array.push(v.to_inner_env_value()?);
                }
                Ok(EnvValue::Array(env_array))
            }
            TemplateValue::Object(o) => {
                let mut env_object = BTreeMap::new();
                for (name, template_to_value) in o {
                    for (_, value) in template_to_value {
                        let value = value.to_inner_env_value()?;
                        if let Some(existing_value) = env_object.get(&name) {
                            if existing_value != &value {
                                return Err(anyhow::anyhow!(
                                    "Conflict detected for variable '{}'. \
                                    Existing value: '{:?}', attempted new value: '{:?}'",
                                    name,
                                    existing_value,
                                    value
                                ));
                            }
                        } else {
                            env_object.insert(name.clone(), value);
                        }
                    }
                }
                Ok(EnvValue::Object(env_object))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateValueEnv {
    Value(EnvValue),
    Object(TemplateObjectEnv),
}

pub type TemplateObjectEnv = BTreeMap<String, EnvValue>;

pub trait TemplateObjectExt {
    fn merge_into(&mut self, other: &TemplateObject) -> Result<(), anyhow::Error>;
    fn to_env_records(self) -> Result<TemplateObjectEnv, anyhow::Error>;
}

impl TemplateObjectExt for TemplateObject {
    fn merge_into(&mut self, other: &TemplateObject) -> Result<(), anyhow::Error> {
        for (name, template_map2) in other {
            match self.get_mut(name) {
                Some(template_map1) => {
                    for (tmpl, value2) in template_map2 {
                        match template_map1.get(tmpl) {
                            Some(value1) if value1 != value2 => {
                                // Conflict detected with differing values
                                return Err(anyhow::anyhow!(
                                    "Conflict detected for variable '{}'. \
                                    Existing value: '{:?}', attempted new value: '{:?}'",
                                    name,
                                    value1,
                                    value2
                                ));
                            }
                            _ => {
                                // No conflict or identical values; insert or skip
                                template_map1.insert(tmpl.clone(), value2.clone());
                            }
                        }
                    }
                }
                None => {
                    // Insert new variable name with its template map
                    self.insert(name.clone(), template_map2.clone());
                }
            }
        }
        Ok(())
    }

    fn to_env_records(self) -> Result<TemplateObjectEnv, anyhow::Error> {
        let mut env_object = BTreeMap::new();
        for (name, template_to_value) in self {
            for (tmpl, value) in template_to_value {
                let env_variable_name = generate_env_variable_name(&name, &tmpl);
                match value.to_env_records()? {
                    TemplateValueEnv::Value(v) => {
                        env_object.insert(env_variable_name, v);
                    }
                    TemplateValueEnv::Object(o) => {
                        env_object.extend(o);
                    }
                }
            }
        }
        Ok(env_object)
    }
}

fn generate_env_variable_name(name: &str, tmpl: &str) -> String {
    // 1. Convert name to SCREAMING_SNAKE_CASE
    let name = to_upper_snake_case(name);

    // 2. Inline into tmpl
    let tmpl = tmpl.replace("{}", &name);

    // 3. tmpl is assumed to already be in SCREAMING_SNAKE_CASE
    tmpl
}

pub trait IntoTemplateObject {
    fn into_template_object(self) -> TemplateObject;
}

impl IntoTemplateObject for serde_json::Map<String, Value> {
    fn into_template_object(self) -> TemplateObject {
        let mut template_object = TemplateObject::new();
        for (key, value) in self {
            let mut template_to_value = BTreeMap::new();
            template_to_value.insert("{}".to_string(), TemplateValue::from_value(value));
            template_object.insert(key, template_to_value);
        }
        template_object
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn merge_tests() {
        let v1 = json!({
            "name": "John",
            "courses": ["Math", "Science", "History"]
        });
        let v2 = json!({
            "age": 30,
            "is_student": false,
        });
        let mut v1 = TemplateValue::from_value(v1);
        let v2 = TemplateValue::from_value(v2);
        assert!(v1.merge_into(&v2).is_ok());
        assert_eq!(
            v1,
            TemplateValue::from_value(json!({
                "name": "John",
                "age": 30,
                "is_student": false,
                "courses": ["Math", "Science", "History"]
            }))
        );
    }
}
