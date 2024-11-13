use crate::{
    ancestor_mapping::AncestorMapping,
    merging::merge_map_consume,
    space_graph::{CopyTree, GenerateSpace, SpaceGraph},
};
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSpace {
    pub variables: Option<Map<String, Value>>,
    pub root_mapping: AncestorMapping,
    pub environments: HashSet<String>,
    pub path: PathBuf,
    pub files_to_copy: CopyTree,
    pub generate: GenerateSpace,
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

// The root mapping is the mapping from the ENV variable to this space's environments.
// Other mappings such as dependency mappings may be omitted.

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

    let mut root_mapping = space.parent_mapping.clone();
    if let Some(parent_space) = &space.parent_space {
        let parent_space = resolve_parent(
            parent_space,
            &space.parent_mapping,
            &mut variables,
            visited,
            resolved_spaces,
            space_graph,
        )
        .with_context(|| format!("Failed to resolve parent for path: {:?}", name))?;

        // Turn the parents root_mapping and this space's parent_mapping into a root_mapping for this space
        let mut new_root_mapping = AncestorMapping::new();

        // For each ancestor in the parent's root mapping
        for (ancestor, parent_space_env) in parent_space.root_mapping.list_ancestor_to_space() {
            // Look up what this space's environments are for the parent's space environment
            if let Some(space_envs) = space.parent_mapping.get_space(parent_space_env) {
                // Add mapping from ancestor to this space's environment
                new_root_mapping.add_mapping(ancestor.clone(), space_envs.clone())?;
            }
        }

        root_mapping = new_root_mapping;
    }

    for dependency in &space.dependencies {
        resolve_dependency(
            dependency,
            &root_mapping,
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

    if let Some(variables) = &mut variables {
        // insert empty object for each environment if not present
        for env in &space.environments {
            variables
                .entry(env.clone())
                .or_insert_with(|| Value::Object(Map::new()));
        }
    }

    resolved_spaces.insert(
        name.to_string(),
        ResolvedSpace {
            variables,
            environments: space.environments.clone(),
            path: space.path.clone(),
            files_to_copy: space.files_to_copy.clone(),
            generate: space.generate.clone(),
            root_mapping,
        },
    );

    Ok(())
}

fn resolve_parent<'a>(
    parent_name: &str,
    parent_mapping: &AncestorMapping,
    this_variables: &mut Option<Map<String, Value>>,
    visited: &mut HashSet<String>,
    resolved_spaces: &'a mut HashMap<String, ResolvedSpace>,
    space_graph: &SpaceGraph,
) -> Result<&'a ResolvedSpace> {
    resolve_space(parent_name, visited, resolved_spaces, space_graph)
        .with_context(|| format!("Failed to resolve dependency path: {:?}", parent_name))?;

    let resolved_space = resolved_spaces
        .get(parent_name)
        .with_context(|| format!("Resolved space not found for path: {:?}", parent_name))?;

    let mut to_merge = resolved_space.variables.clone();

    for dependency_env in &resolved_space.environments {
        let space_env = parent_mapping.get_space(dependency_env);
        if let Some(space_env) = space_env {
            if let Some(ref mut value) = to_merge {
                if let Some(moved_value) = value.remove(dependency_env) {
                    value.insert(space_env.clone(), moved_value.clone());
                }
            }
        }
    }

    if let Some(to_merge) = to_merge {
        if let Some(ref mut value) = this_variables {
            let value_clone = value.clone();
            let to_merge_clone = to_merge.clone();
            merge_map_consume(value, to_merge).with_context(|| {
                format!(
                    "Failed to merge variables for dependency: {:?}, {:?}, {:?}",
                    parent_name, value_clone, to_merge_clone
                )
            })?;
        } else {
            *this_variables = Some(to_merge);
        }
    }

    Ok(&resolved_space)
}

fn resolve_dependency<'a>(
    dependency_name: &str,
    root_mapping: &AncestorMapping,
    this_variables: &mut Option<Map<String, Value>>,
    visited: &mut HashSet<String>,
    resolved_spaces: &'a mut HashMap<String, ResolvedSpace>,
    space_graph: &SpaceGraph,
) -> Result<&'a ResolvedSpace> {
    resolve_space(dependency_name, visited, resolved_spaces, space_graph)
        .with_context(|| format!("Failed to resolve dependency path: {:?}", dependency_name))?;

    let resolved_space = resolved_spaces
        .get(dependency_name)
        .with_context(|| format!("Resolved space not found for path: {:?}", dependency_name))?;

    let mut to_merge = resolved_space.variables.clone();

    if let Some(to_merge) = to_merge.as_mut() {
        for dependency_env in &resolved_space.environments {
            let rooted_dependency_envs = resolved_space.root_mapping.get_ancestors(dependency_env);
            if let Some(moved_value) = to_merge.remove(dependency_env) {
                for rooted_dependency_env in rooted_dependency_envs {
                    let space_env = root_mapping.get_space(rooted_dependency_env);
                    if let Some(space_env) = space_env {
                        to_merge.insert(space_env.clone(), moved_value.clone());
                    }
                }
            }
        }
    }

    if let Some(to_merge) = to_merge {
        if let Some(ref mut value) = this_variables {
            let value_clone = value.clone();
            let to_merge_clone = to_merge.clone();
            merge_map_consume(value, to_merge).with_context(|| {
                format!(
                    "Failed to merge variables for dependency: {:?}, {:?}, {:?}",
                    dependency_name, value_clone, to_merge_clone
                )
            })?;
        } else {
            *this_variables = Some(to_merge);
        }
    }

    Ok(&resolved_space)
}
