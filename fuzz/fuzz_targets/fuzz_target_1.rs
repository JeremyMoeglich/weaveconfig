#![no_main]
use envoyr::serialize_env::{encode_env, parse_env, EnvValue};
use libfuzzer_sys::fuzz_target;

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
        (EnvValue::Boolean(a), EnvValue::Boolean(b)) => a == b,
        (EnvValue::Array(a), EnvValue::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| env_value_eq(x, y))
        }
        _ => false,
    }
}
