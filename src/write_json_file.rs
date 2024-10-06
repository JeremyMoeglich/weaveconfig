use std::path::Path;

use crate::resolve_spaces::ResolvedSpace;
use tokio::fs;

pub async fn write_json_file(
    resolved_space: &ResolvedSpace,
    gen_folder: &Path,
) -> Result<(), anyhow::Error> {
    if let Some(variables) = &resolved_space.variables {
        let env_file_path = gen_folder.join("config.json");
        let env_file_content = serde_json::to_string_pretty(variables)?;
        fs::write(env_file_path, env_file_content).await?;
    }

    Ok(())
}
