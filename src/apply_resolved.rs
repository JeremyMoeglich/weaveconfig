use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};
use serde_json::Map;

use crate::{
    map_path::map_path, resolve_spaces::ResolvedSpace, template_file::template_file,
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
    let gen_folder = gen_folder(&real_path).await?;
    write_gitignore(&gen_folder).await?;
    write_json_file(&space, &gen_folder).await?;
    generate_binding(&space, &gen_folder).await?;
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

async fn write_to_copy(space: &ResolvedSpace, real_path: &PathBuf) -> Result<(), anyhow::Error> {
    for origin in &space.files_to_copy {
        let dest_relative = origin.strip_prefix(&space.path).unwrap();
        let dest = real_path.join(dest_relative);

        let content = tokio::fs::read_to_string(origin).await?;
        let content = {
            let variables = space.variables.clone().unwrap_or(Map::new());
            template_file(&content, &variables)
                .with_context(|| format!("Failed to template file: {}", origin.display()))?
        };

        // create parent dirs
        if let Some(parent) = dest.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        tokio::fs::write(dest, content).await?;
    }
    Ok(())
}
