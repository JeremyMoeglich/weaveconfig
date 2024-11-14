use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;

use crate::{
    ancestor_mapping::{AncestorMapping, RootMappingError},
    file_graph::Directory,
    schemas::GenerateSchema,
};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    // spaces are resolved individually, so these map to their parent, not the root.
    // the root mapping is resolved later based on the parent mapping.
    pub parent_mapping: AncestorMapping,
    pub environments: HashSet<String>,
    pub variables: Option<serde_json::Map<String, serde_json::Value>>,
    pub files_to_copy: CopyTree,
    pub parent_space: Option<String>,
    pub generate: GenerateSpace,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopyTree {
    pub to_copy: Vec<ToCopy>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToCopy {
    File(PathBuf),
    Directory { path: PathBuf, subtree: CopyTree },
}

impl ToCopy {
    pub fn last_segment(&self) -> Result<&str, anyhow::Error> {
        let path = match self {
            ToCopy::File(path) => path,
            ToCopy::Directory { path, .. } => path,
        };
        let file_name = path.file_name().context("File has no name")?;
        let file_name = file_name
            .to_str()
            .context("File name is not valid unicode")?;
        Ok(file_name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenerateSpace {
    pub generate: bool,
    pub typescript: bool,
}

pub type SpaceGraph = HashMap<String, Space>;

pub fn create_space_graph(root_directory: Directory) -> Result<SpaceGraph, anyhow::Error> {
    let mut space_graph = HashMap::new();

    add_to_spaces_graph(root_directory, &mut space_graph, None)
        .with_context(|| "Failed to add to spaces graph")?;

    Ok(space_graph)
}

fn add_to_spaces_graph(
    mut dir: Directory,
    space_graph: &mut SpaceGraph,
    closest_parent_space: Option<String>,
) -> Result<(), RootMappingError> {
    let space_name = dir
        .space
        .as_ref()
        .map(|s| s.info.name.to_string())
        .or_else(|| closest_parent_space.clone());
    if let Some(space) = dir.space.take() {
        let mut mapping = match space.info.space_to_parent_mapping {
            Some(m) => AncestorMapping::from_space_to_ancestors(m)?,
            None => AncestorMapping::new(),
        };
        let environments = space.info.environments.unwrap_or_default();
        for environment in &environments {
            if !mapping.contains_space(environment) {
                mapping
                    .add_mapping(environment.clone(), environment.clone())
                    .expect(&format!(
                        "Failed to add mapping for environment: {}",
                        environment
                    ));
            }
        }

        let space = Space {
            name: space.info.name,
            path: dir.path.clone(),
            dependencies: space.info.dependencies.unwrap_or_default(),
            parent_mapping: mapping,
            environments,
            variables: space.variables,
            files_to_copy: resolve_files_to_copy(&dir),
            parent_space: closest_parent_space,
            generate: {
                match space.info.generate {
                    Some(GenerateSchema::Generate(generate)) => GenerateSpace {
                        generate: true,
                        typescript: generate.typescript,
                    },
                    Some(GenerateSchema::ShouldGenerate(generate)) => GenerateSpace {
                        generate,
                        typescript: true,
                    },
                    None => GenerateSpace {
                        generate: true,
                        typescript: true,
                    },
                }
            },
        };
        space_graph.insert(space.name.clone(), space);
    }

    for entry in dir.directories {
        add_to_spaces_graph(entry, space_graph, space_name.clone())?;
    }
    Ok(())
}

fn resolve_files_to_copy(dir: &Directory) -> CopyTree {
    let mut files = vec![];
    for file in &dir.rest_to_copy {
        files.push(ToCopy::File(file.clone()));
    }

    for entry in &dir.directories {
        if entry.space.is_none() {
            files.push(ToCopy::Directory {
                path: entry.path.clone(),
                subtree: resolve_files_to_copy(entry),
            });
        }
    }

    CopyTree { to_copy: files }
}
