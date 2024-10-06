#![no_main]
use weaveconfig::serialize_env::{encode_env, parse_env, EnvValue};
use libfuzzer_sys::fuzz_target;

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

fuzz_target!(|data: EnvValue| {
    let encoded = encode_env(data.clone());
    let decoded = parse_env(&encoded)
        .map_err(|e| format!("Failed to parse env: {} on input {}", e, encoded))
        .unwrap();
    assert!(
        env_value_eq(&data, &decoded),
        "Values differ: {:?} != {:?}",
        data,
        decoded
    );
});

fn env_value_eq(a: &EnvValue, b: &EnvValue) -> bool {
    match (a, b) {
        (EnvValue::Number(a), EnvValue::Number(b)) => {
            // Treat NaNs as equal
            (a.is_nan() && b.is_nan()) || (a == b)
        }
        (EnvValue::String(a), EnvValue::String(b)) => a == b,
        (EnvValue::Bool(a), EnvValue::Bool(b)) => a == b,
        (EnvValue::Array(a), EnvValue::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| env_value_eq(x, y))
        }
        (EnvValue::Null, EnvValue::Null) => true,
        (EnvValue::Object(a), EnvValue::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|((k1, v1), (k2, v2))| k1 == k2 && env_value_eq(v1, v2))
        }
        _ => false,
    }
}
