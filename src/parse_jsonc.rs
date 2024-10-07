use serde::de::DeserializeOwned;

pub fn parse_jsonc<T: DeserializeOwned>(input: &str) -> Result<T, anyhow::Error> {
    let json = fjson::to_json_compact(input)?;
    serde_json::from_str(&json).map_err(|e| anyhow::anyhow!(e))
}
