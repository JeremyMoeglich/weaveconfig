use anyhow::Error;
use serde_json::{Map, Value};

pub fn merge_values_consume(v1: &mut Value, v2: Value) -> Result<(), Error> {
    match (v1, v2) {
        (Value::Object(ref mut o1), Value::Object(o2)) => {
            merge_map_consume(o1, o2)?;
            Ok(())
        }
        (v1, v2) => {
            if v1 != &v2 {
                return Err(anyhow::anyhow!("Conflicting values: {:?} and {:?}", v1, v2));
            }
            Ok(())
        }
    }
}

pub fn merge_map_consume(m1: &mut Map<String, Value>, m2: Map<String, Value>) -> Result<(), Error> {
    for (k, v) in m2 {
        if let Some(existing_value) = m1.get_mut(&k) {
            merge_values_consume(existing_value, v)?;
        } else {
            m1.insert(k, v);
        }
    }
    Ok(())
}
