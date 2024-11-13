use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};
use serde_json::{Map, Value};

use crate::{
    get_environment_value::get_environment_value,
    map_path::map_path,
    merging::merge_values_consume,
    resolve_spaces::ResolvedSpace,
    space_graph::{CopyTree, ToCopy},
    template_file::template_file,
    ts_binding::generate_binding::generate_binding,
    write_json_file::write_json_file,
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

// Function to write files and directories to be copied
async fn write_to_copy(space: &ResolvedSpace, real_path: &Path) -> Result<(), anyhow::Error> {
    // Copy the tree structure with files and directories
    copy_tree(
        &space.files_to_copy,
        real_path,
        None,
        &space.variables,
        &space.environments,
    )
    .await
    .with_context(|| format!("Failed to copy tree structure for: {}", real_path.display()))?;

    Ok(())
}

// Recursive function to copy a tree of files and directories
async fn copy_tree(
    copytree: &CopyTree,
    copy_into: &Path,
    env: Option<&str>,
    variables: &Option<Map<String, Value>>,
    environments: &HashSet<String>,
) -> Result<(), anyhow::Error> {
    for to_copy in &copytree.to_copy {
        let prefix = "_forenv";
        // Check if the file/directory name needs environment-specific substitution
        if needs_substitution(
            &to_copy
                .last_segment()
                .with_context(|| format!("Failed to get last segment for {:?}", to_copy))?,
            prefix,
        ) {
            match env {
                // If environment is specified, copy with that environment
                Some(env) => {
                    copy_tocopy_with_env(to_copy, copy_into, Some(env), variables, environments)
                        .await
                        .with_context(|| {
                            format!("Failed to copy {:?} with environment: {}", to_copy, env)
                        })?;
                }
                // If no environment is specified, copy for all environments
                None => {
                    for env in environments {
                        // Get environment-specific variables
                        let variables = match variables {
                            Some(variables) => {
                                Some(get_environment_value(variables, env).with_context(|| {
                                    format!(
                                        "Failed to get environment value for '{}' in {:?}",
                                        env, variables
                                    )
                                })?)
                            }
                            None => None,
                        };
                        copy_tocopy_with_env(
                            to_copy,
                            copy_into,
                            Some(env),
                            &variables,
                            environments,
                        )
                        .await
                        .with_context(|| {
                            format!("Failed to copy {:?} for environment: {}", to_copy, env)
                        })?;
                    }
                }
            }
        } else {
            // If no environment substitution is needed, copy without environment
            copy_tocopy_with_env(to_copy, copy_into, None, variables, environments)
                .await
                .with_context(|| {
                    format!(
                        "Failed to copy {:?} without environment substitution",
                        to_copy
                    )
                })?;
        }
    }

    Ok(())
}

// Function to copy a single file or directory with environment-specific handling
async fn copy_tocopy_with_env(
    to_copy: &ToCopy,
    copy_into: &Path,
    env: Option<&str>,
    variables: &Option<Map<String, Value>>,
    environments: &HashSet<String>,
) -> Result<(), anyhow::Error> {
    let last_segment = to_copy
        .last_segment()
        .with_context(|| "Failed to get last segment")?;
    // Substitute environment in the file/directory name if needed
    let substituted_name = match env {
        Some(env) => substitute_path_segment(last_segment, "_forenv", env),
        None => last_segment.to_string(),
    };
    let destination = copy_into.join(substituted_name);

    match to_copy {
        ToCopy::File(file) => {
            // Read file content
            let content = tokio::fs::read_to_string(&file)
                .await
                .with_context(|| format!("Failed to read file: {:?}", file))?;
            // Apply variable substitution if variables are provided
            let content = if let Some(variables) = variables {
                let mut env_value = if let Some(env) = env {
                    get_environment_value(variables, env).with_context(|| {
                        format!(
                            "Failed to get environment value for '{}' in {:?}",
                            env, variables
                        )
                    })?
                } else {
                    variables.clone()
                };
                if let Some(env) = env {
                    env_value.insert("env".to_string(), Value::String(env.to_string()));
                }
                template_file(&content, &env_value)
                    .with_context(|| "Failed to apply variable substitution")?
            } else {
                content
            };
            // Write the processed content to the destination
            tokio::fs::write(&destination, content)
                .await
                .with_context(|| format!("Failed to write to destination: {:?}", destination))?;
        }
        ToCopy::Directory { subtree, .. } => {
            // Create the directory if it doesn't exist
            if !destination.exists() {
                tokio::fs::create_dir(&destination)
                    .await
                    .with_context(|| format!("Failed to create directory: {:?}", destination))?;
            }
            // Recursively copy the subdirectory
            Box::pin(copy_tree(
                subtree,
                &destination,
                env,
                variables,
                environments,
            ))
            .await
            .with_context(|| {
                format!("Failed to recursively copy subdirectory: {:?}", destination)
            })?;
        }
    }

    Ok(())
}

// Function to substitute environment in a path segment
fn substitute_path_segment(segment: &str, from: &str, to: &str) -> String {
    if needs_substitution(segment, from) {
        segment.replacen(from, to, 1)
    } else {
        segment.to_string()
    }
}

// Function to check if a segment needs environment substitution
fn needs_substitution(segment: &str, from: &str) -> bool {
    segment.starts_with(from)
}
