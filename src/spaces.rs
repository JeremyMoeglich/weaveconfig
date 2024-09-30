use serde_json::{map::Entry, Value};

use crate::graph::{Dependency, Directory, Graph};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

pub struct ResolvedSpace {
    variables: Option<serde_json::Value>,
    environments: HashSet<String>,
    path: PathBuf,
    files_to_copy: Vec<PathBuf>,
}

pub fn resolve_spaces(graph: &Graph) -> Result<HashMap<PathBuf, ResolvedSpace>, anyhow::Error> {
    let mut resolved_spaces = HashMap::new();
    let mut visited = HashSet::new();
    for path in graph.keys() {
        resolve_space(&path, &mut visited, &mut resolved_spaces, &graph)?;
    }

    Ok(resolved_spaces)
}

fn resolve_space(
    path: &PathBuf,
    visited: &mut HashSet<PathBuf>,
    resolved_spaces: &mut HashMap<PathBuf, ResolvedSpace>,
    graph: &Graph,
) -> Result<(), anyhow::Error> {
    if let Some(_) = resolved_spaces.get(path) {
        return Ok(());
    }

    if visited.contains(path) {
        return Err(anyhow::anyhow!("Spaces have cyclic dependencies"));
    }

    visited.insert(path.clone());

    let dir = graph
        .get(path)
        .ok_or(anyhow::anyhow!("Directory not found"))?;

    let mut files_to_copy = Vec::new();
    resolve_files_to_copy(&dir, &mut files_to_copy)?;

    match &dir.space {
        Some(space) => {
            let mut variables = space.variables.clone();
            for dependency in &space.dependencies {
                resolve_dependency(
                    &space.mapping,
                    &space.environments,
                    dependency,
                    &mut variables,
                    visited,
                    resolved_spaces,
                    graph,
                )?;
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
            Ok(())
        }
        None => Err(anyhow::anyhow!("No space found for directory")),
    }
}

fn resolve_files_to_copy(dir: &Directory, files: &mut Vec<PathBuf>) -> Result<(), anyhow::Error> {
    for file in &dir.rest_to_copy {
        files.push(file.clone());
    }

    for entry in &dir.directories {
        if entry.space.is_none() {
            resolve_files_to_copy(&entry, files)?;
        }
    }
    Ok(())
}

fn resolve_dependency(
    mapping: &Option<HashMap<String, Vec<String>>>,
    environments: &HashSet<String>,
    dependency: &Dependency,
    variables: &mut Option<Value>,
    visited: &mut HashSet<PathBuf>,
    resolved_spaces: &mut HashMap<PathBuf, ResolvedSpace>,
    graph: &Graph,
) -> Result<(), anyhow::Error> {
    resolve_space(&dependency.path, visited, resolved_spaces, graph)?;
    let resolved_space = resolved_spaces.get(&dependency.path).unwrap();

    let mut from_to_merge = resolved_space.variables.clone();

    for from_env in &resolved_space.environments {
        if let Some(mapped_envs) = mapping.as_ref().and_then(|m| m.get(from_env)) {
            for to_env in mapped_envs {
                if !environments.contains(to_env) {
                    return Err(anyhow::anyhow!(
                        "The target environment '{}' is not defined. Available environments are: {:?}",
                        to_env,
                        environments
                    ));
                }

                if let Some(ref mut value) = from_to_merge {
                    move_key(value, from_env, to_env)?;
                } else {
                    return Err(anyhow::anyhow!(
                        "No value present to move from '{}' to '{}'",
                        from_env,
                        to_env
                    ));
                }
            }
        }
    }

    if let Some(from_to_merge) = from_to_merge {
        if let Some(ref mut value) = variables {
            merge_values_in_place(value, from_to_merge)?;
        } else {
            *variables = Some(from_to_merge);
        }
    }

    Ok(())
}

fn move_key(value: &mut Value, from_key: &str, to_key: &str) -> Result<(), anyhow::Error> {
    let obj = match value.as_object_mut() {
        Some(obj) => obj,
        None => return Err(anyhow::anyhow!("Expected an object, got {:?}", value)),
    };

    if from_key == to_key {
        return Ok(());
    }

    if let Some(from_value) = obj.remove(from_key) {
        match obj.entry(to_key.to_string()) {
            Entry::Occupied(mut entry) => {
                merge_values_in_place(entry.get_mut(), from_value)?;
            }
            Entry::Vacant(entry) => {
                entry.insert(from_value);
            }
        }
    }

    Ok(())
}

fn merge_values_in_place(first: &mut Value, second: Value) -> Result<(), anyhow::Error> {
    match (first, second) {
        (Value::Object(map1), Value::Object(map2)) => {
            for (key, value) in map2 {
                match map1.entry(key) {
                    Entry::Occupied(mut entry) => {
                        merge_values_in_place(entry.get_mut(), value)?;
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(value);
                    }
                }
            }
            Ok(())
        }
        _ => Err(anyhow::anyhow!("Cannot merge non-object values")),
    }
}
