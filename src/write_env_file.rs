use crate::{
    resolve_spaces::ResolvedSpace,
    serialize_env::encode_env,
    template_value::{TemplateObjectEnv, TemplateObjectExt},
};
use tokio::fs;

pub async fn write_env_file(resolved_space: &ResolvedSpace) -> Result<(), anyhow::Error> {
    if let Some(variables) = &resolved_space.variables {
        let gen_folder = resolved_space.gen_folder().await?;
        let env_file_path = gen_folder.join(".env");
        let env_file_records = variables.clone().to_env_records()?;
        let env_file_content = serialize_env_records(env_file_records)?;
        fs::write(env_file_path, env_file_content).await?;
    }

    Ok(())
}

fn serialize_env_records(env_records: TemplateObjectEnv) -> Result<String, anyhow::Error> {
    let mut env_file_content = String::new();
    for (key, value) in env_records {
        env_file_content.push_str(&format!("{}={}\n", key, encode_env(value)));
    }
    Ok(env_file_content)
}
