use crate::{
    graph::{Dependency, Directory, Graph},
    template_value::{IntoTemplateObject, TemplateObject, TemplateObjectExt},
};
use anyhow::{Context, Result};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSpace {
    pub variables: Option<TemplateObject>,
    pub environments: HashSet<String>,
    pub path: PathBuf,
    pub files_to_copy: Vec<PathBuf>,
}

impl ResolvedSpace {
    pub async fn gen_folder(&self) -> Result<PathBuf> {
        let gen_folder = self.path.join("gen");
        if !gen_folder.exists() {
            tokio::fs::create_dir_all(&gen_folder).await?;
        }
        Ok(gen_folder)
    }
}

pub fn resolve_spaces(graph: &Graph) -> Result<HashMap<PathBuf, ResolvedSpace>> {
    let mut resolved_spaces = HashMap::new();
    let mut visited = HashSet::new();

    for path in graph.keys() {
        resolve_space(&path, &mut visited, &mut resolved_spaces, &graph)
            .with_context(|| format!("Failed to resolve space for path: {:?}", path))?;
    }

    Ok(resolved_spaces)
}

fn resolve_space(
    path: &PathBuf,
    visited: &mut HashSet<PathBuf>,
    resolved_spaces: &mut HashMap<PathBuf, ResolvedSpace>,
    graph: &Graph,
) -> Result<()> {
    if resolved_spaces.contains_key(path) {
        return Ok(()); // Space already resolved.
    }

    if visited.contains(path) {
        return Err(anyhow::anyhow!(
            "Cyclic dependency detected for path: {:?}",
            path
        ));
    }

    visited.insert(path.clone());

    let dir = graph
        .get(path)
        .with_context(|| format!("Directory not found for path: {:?}", path))?;

    match &dir.space {
        Some(space) => {
            let mut files_to_copy = Vec::new();
            resolve_files_to_copy(&dir, &mut files_to_copy).with_context(|| {
                format!(
                    "Failed to resolve files to copy for directory: {:?}",
                    dir.path
                )
            })?;

            let mut variables = space.variables.clone().map(|v| v.into_template_object());
            for dependency in &space.dependencies {
                resolve_dependency(
                    &space.mapping,
                    &space.environments,
                    path,
                    dependency,
                    &mut variables,
                    visited,
                    resolved_spaces,
                    graph,
                )
                .with_context(|| {
                    format!(
                        "Failed to resolve dependency: {:?} for space: {:?}",
                        dependency, path
                    )
                })?;
            }

            if let Some(parent) = &dir.parent {
                resolve_parent(
                    &space.mapping,
                    &space.environments,
                    parent,
                    &mut variables,
                    visited,
                    resolved_spaces,
                    graph,
                )
                .with_context(|| format!("Failed to resolve parent for path: {:?}", path))?;
            }

            resolved_spaces.insert(
                path.clone(),
                ResolvedSpace {
                    variables,
                    environments: space.environments.clone(),
                    path: path.clone(),
                    files_to_copy,
                },
            );
        }
        None => {}
    }

    Ok(())
}

fn resolve_parent(
    mapping: &Option<BTreeMap<String, Vec<String>>>,
    environments: &HashSet<String>,
    parent: &PathBuf,
    variables: &mut Option<TemplateObject>,
    visited: &mut HashSet<PathBuf>,
    resolved_spaces: &mut HashMap<PathBuf, ResolvedSpace>,
    graph: &Graph,
) -> Result<()> {
    // traverse up until a directory with a space is found
    let mut current_parent = Some(parent);
    while let Some(parent) = current_parent {
        let parent_dir = graph
            .get(parent)
            .with_context(|| format!("Directory not found for path: {:?}", parent))?;
        if parent_dir.space.is_some() {
            break;
        }
        current_parent = parent_dir.parent.as_ref().map(|p| p);
    }
    if let Some(current_parent) = current_parent {
        resolve_dependency(
            mapping,
            environments,
            &current_parent,
            &Dependency {
                path: current_parent.clone(),
                template: None,
                keys: None,
            },
            variables,
            visited,
            resolved_spaces,
            graph,
        )
        .with_context(|| format!("Failed to resolve parent for path: {:?}", current_parent))?;
        return Ok(());
    }
    Err(anyhow::anyhow!(
        "No parent directory with a space found for path: {:?}",
        parent
    ))
}

fn resolve_files_to_copy(dir: &Directory, files: &mut Vec<PathBuf>) -> Result<()> {
    for file in &dir.rest_to_copy {
        files.push(file.clone());
    }

    for entry in &dir.directories {
        if entry.space.is_none() {
            resolve_files_to_copy(&entry, files).with_context(|| {
                format!(
                    "Failed to resolve files to copy for sub-directory: {:?}",
                    entry.path
                )
            })?;
        }
    }
    Ok(())
}

fn canonicalize_relative_path(base: &PathBuf, relative: &PathBuf) -> std::io::Result<PathBuf> {
    // Join the base path with the relative path to form an absolute path
    let absolute_path = base.join(relative);

    // Canonicalize the resulting absolute path
    std::fs::canonicalize(absolute_path)
}

fn resolve_dependency(
    mapping: &Option<BTreeMap<String, Vec<String>>>,
    environments: &HashSet<String>,
    base_path: &PathBuf,
    dependency: &Dependency,
    variables: &mut Option<TemplateObject>,
    visited: &mut HashSet<PathBuf>,
    resolved_spaces: &mut HashMap<PathBuf, ResolvedSpace>,
    graph: &Graph,
) -> Result<()> {
    let canonicalized_path = canonicalize_relative_path(base_path, &dependency.path)?;
    resolve_space(&canonicalized_path, visited, resolved_spaces, graph)
        .with_context(|| format!("Failed to resolve dependency path: {:?}", dependency.path))?;

    let resolved_space = resolved_spaces
        .get(&canonicalized_path)
        .with_context(|| format!("Resolved space not found for path: {:?}", dependency.path))?;

    let mut to_merge = resolved_space.variables.clone();

    for from_env in &resolved_space.environments {
        if let Some(mapped_envs) = mapping.as_ref().and_then(|m| m.get(from_env)) {
            for to_env in mapped_envs {
                if !environments.contains(to_env) {
                    return Err(anyhow::anyhow!(
                        "The target environment '{}' is not defined in space. Available environments: {:?}",
                        to_env,
                        environments
                    ));
                }

                if let Some(ref mut value) = to_merge {
                    move_key(value, from_env, to_env).with_context(|| {
                        format!(
                            "Failed to move value from environment '{}' to '{}'",
                            from_env, to_env
                        )
                    })?;
                } else {
                    return Err(anyhow::anyhow!(
                        "No variables present to move from '{}' to '{}'",
                        from_env,
                        to_env
                    ));
                }
            }
        }
    }

    if let Some(to_merge) = to_merge {
        if let Some(ref mut value) = variables {
            value.merge_into(&to_merge).with_context(|| {
                format!(
                    "Failed to merge variables for dependency: {:?}",
                    dependency.path
                )
            })?;
        } else {
            *variables = Some(to_merge);
        }
    }

    Ok(())
}

fn move_key(value: &mut TemplateObject, from_env: &str, to_env: &str) -> Result<()> {
    let current = value.remove(from_env).ok_or_else(|| {
        anyhow::anyhow!(
            "No value present for environment '{}'. Cannot move to '{}'.",
            from_env,
            to_env
        )
    })?;
    value.insert(to_env.to_string(), current);
    Ok(())
}
