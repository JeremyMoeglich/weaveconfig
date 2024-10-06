use std::path::Path;

use crate::{resolve_spaces::ResolvedSpace, ts_binding::format_ts::format_ts_string};
use anyhow::Error;
use serde_json::Value;

use super::format_ts_type::json_value_to_ts_type;

pub async fn generate_binding(
    resolved_space: &ResolvedSpace,
    output_dir: &Path,
) -> Result<(), Error> {
    if let Some(variables) = &resolved_space.variables {
        let zero_env_content = include_str!("./zero_env.ts");
        let single_env_content = include_str!("./one_env.ts");
        let multi_env_content = include_str!("./multi_env.ts");

        let mut content = String::new();
        let ts_type = json_value_to_ts_type(&Value::Object(variables.clone()));
        content.push_str(&format!("type ConfigType = {};\n\n", ts_type));

        content.push_str("const environments = ");
        content.push_str(&format!(
            "{} as const;",
            serde_json::to_string(&resolved_space.environments)?
        ));

        content.push_str("\n\n// static code starts here, using variant: ");
        if resolved_space.environments.len() == 0 {
            content.push_str("zero_env\n\n");
            content.push_str(zero_env_content);
        } else if resolved_space.environments.len() == 1 {
            content.push_str("one_env\n\n");
            content.push_str(single_env_content);
        } else {
            content.push_str("multi_env\n\n");
            content.push_str(multi_env_content);
        }

        let formatted = format_ts_string(&content)?;

        let output_path = output_dir.join("binding.ts");
        tokio::fs::write(output_path, formatted).await?;
    }
    Ok(())
}
