use crate::{merging::merge_map_consume, space_graph::SpaceGraph};
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSpace {
    pub variables: Option<Map<String, Value>>,
    pub environments: HashSet<String>,
    pub path: PathBuf,
    pub files_to_copy: Vec<PathBuf>,
}

pub fn resolve_spaces(space_graph: SpaceGraph) -> Result<HashMap<String, ResolvedSpace>> {
    let mut resolved_spaces = HashMap::new();
    let mut visited = HashSet::new();

    for space_name in space_graph.keys() {
        resolve_space(
            &space_name,
            &mut visited,
            &mut resolved_spaces,
            &space_graph,
        )
        .with_context(|| format!("Failed to resolve space for path: {:?}", space_name))?;
    }

    Ok(resolved_spaces)
}

fn resolve_space(
    name: &str,
    visited: &mut HashSet<String>,
    resolved_spaces: &mut HashMap<String, ResolvedSpace>,
    space_graph: &SpaceGraph,
) -> Result<()> {
    let space = space_graph
        .get(name)
        .with_context(|| format!("Space not found for name: {:?}", name))?;

    if resolved_spaces.contains_key(name) {
        return Ok(()); // Space already resolved.
    }

    if visited.contains(name) {
        return Err(anyhow::anyhow!(
            "Cyclic dependency detected for name: {:?}",
            name
        ));
    }

    visited.insert(name.to_string());

    let mut variables = space.variables.clone();
    for dependency in &space.dependencies {
        resolve_dependency(
            name,
            &space.mapping,
            &space.environments,
            &mut variables,
            visited,
            resolved_spaces,
            space_graph,
        )
        .with_context(|| {
            format!(
                "Failed to resolve dependency: {:?} for space: {:?}",
                dependency, name
            )
        })?;
    }

    if let Some(parent_space) = &space.parent_space {
        resolve_dependency(
            parent_space,
            &space.mapping,
            &space.environments,
            &mut variables,
            visited,
            resolved_spaces,
            space_graph,
        )
        .with_context(|| format!("Failed to resolve parent for path: {:?}", name))?;
    }

    if let Some(variables) = &mut variables {
        // insert empty object for each environment if not present
        for env in &space.environments {
            variables
                .entry(env.clone())
                .or_insert(Value::Object(Map::new()));
        }
    }

    resolved_spaces.insert(
        name.to_string(),
        ResolvedSpace {
            variables,
            environments: space.environments.clone(),
            path: space.path.clone(),
            files_to_copy: space.files_to_copy.clone(),
        },
    );

    Ok(())
}

fn resolve_dependency(
    dependency_name: &str,
    mapping: &Option<HashMap<String, Vec<String>>>,
    environments: &HashSet<String>,
    variables: &mut Option<Map<String, Value>>,
    visited: &mut HashSet<String>,
    resolved_spaces: &mut HashMap<String, ResolvedSpace>,
    space_graph: &SpaceGraph,
) -> Result<()> {
    resolve_space(dependency_name, visited, resolved_spaces, space_graph)
        .with_context(|| format!("Failed to resolve dependency path: {:?}", dependency_name))?;

    let resolved_space = resolved_spaces
        .get(dependency_name)
        .with_context(|| format!("Resolved space not found for path: {:?}", dependency_name))?;

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
            merge_map_consume(value, to_merge).with_context(|| {
                format!(
                    "Failed to merge variables for dependency: {:?}",
                    dependency_name
                )
            })?;
        } else {
            *variables = Some(to_merge);
        }
    }

    Ok(())
}

fn move_key(value: &mut Map<String, Value>, from_key: &str, to_key: &str) -> Result<()> {
    let current = value.remove(from_key).ok_or_else(|| {
        anyhow::anyhow!(
            "No value present for environment '{}'. Cannot move to '{}'.",
            from_key,
            to_key
        )
    })?;
    value.insert(to_key.to_string(), current);
    Ok(())
}
