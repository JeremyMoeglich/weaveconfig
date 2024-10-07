use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};

use crate::{
    get_environment_value::get_environment_value, map_path::map_path,
    resolve_spaces::ResolvedSpace, template_file::template_file,
    ts_binding::generate_binding::generate_binding, write_json_file::write_json_file,
};

async fn gen_folder(real_path: &PathBuf) -> Result<PathBuf, anyhow::Error> {
    let gen_folder = real_path.join("gen");
    if !gen_folder.exists() {
        tokio::fs::create_dir_all(&gen_folder).await?;
    }
    Ok(gen_folder)
}

pub async fn apply_resolved(
    spaces: HashMap<String, ResolvedSpace>,
    weave_config_root: &Path,
) -> Result<(), anyhow::Error> {
    let mut futures = FuturesUnordered::new();
    for (_, space) in spaces {
        let real_path = map_path(weave_config_root, &space.path)?;
        futures.push(apply_space(space, real_path));
    }
    while let Some(result) = futures.next().await {
        result?;
    }
    Ok(())
}

async fn apply_space(space: ResolvedSpace, real_path: PathBuf) -> Result<(), anyhow::Error> {
    if !real_path.exists() {
        return Err(anyhow::anyhow!(
            "Could not output to path, does not exist: {}",
            real_path.display()
        ));
    }
    if space.generate.generate && space.variables.is_some() {
        let gen_folder = gen_folder(&real_path).await?;
        write_gitignore(&gen_folder).await?;
        write_json_file(&space, &gen_folder).await?;
        if space.generate.typescript {
            generate_binding(&space, &gen_folder).await?;
        }
    }
    write_to_copy(&space, &real_path).await?;
    Ok(())
}

async fn write_gitignore(gen_folder: &PathBuf) -> Result<(), anyhow::Error> {
    let gitignore_path = gen_folder.join(".gitignore");
    if !gitignore_path.exists() {
        tokio::fs::write(gitignore_path, "config.json\nbinding.ts\n").await?;
    }
    Ok(())
}

async fn write_to_copy(space: &ResolvedSpace, real_path: &Path) -> Result<(), anyhow::Error> {
    // Prepare variables once to avoid cloning multiple times
    let variables = space.variables.clone().unwrap_or_default();

    for to_copy in &space.files_to_copy {
        // Determine the relative destination path
        let dest_relative = to_copy
            .path
            .strip_prefix(&space.path)
            .with_context(|| format!("Failed to strip prefix for {}", to_copy.path.display()))?;

        // Read the file content asynchronously
        let content = tokio::fs::read_to_string(&to_copy.path)
            .await
            .with_context(|| format!("Failed to read file: {}", to_copy.path.display()))?;

        // Compute the full destination path
        let mapped_dist = real_path.join(dest_relative);

        // Create parent directories if they don't exist
        if let Some(parent) = mapped_dist.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("Failed to create directories: {}", parent.display()))?;
        }

        if to_copy.for_each_env {
            // Iterate over each environment and write templated files
            for env in &space.environments {
                let env_variables = get_environment_value(&variables, env)
                    .with_context(|| format!("Failed to get variables for environment: {}", env))?;

                let templated_content =
                    template_file(&content, &env_variables).with_context(|| {
                        format!(
                            "Failed to template file for environment {}: {}",
                            env,
                            to_copy.path.display()
                        )
                    })?;

                let dest = mapped_dist.with_file_name(format!("{}.{}", env, to_copy.dest_filename));

                tokio::fs::write(&dest, templated_content)
                    .await
                    .with_context(|| format!("Failed to write file: {}", dest.display()))?;
            }
        } else {
            // Template the content once and write to the destination
            let templated_content = template_file(&content, &variables)
                .with_context(|| format!("Failed to template file: {}", to_copy.path.display()))?;

            let dest = mapped_dist.with_file_name(&to_copy.dest_filename);

            tokio::fs::write(&dest, templated_content)
                .await
                .with_context(|| format!("Failed to write file: {}", dest.display()))?;
        }
    }

    Ok(())
}
