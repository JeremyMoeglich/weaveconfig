use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;

use crate::{file_graph::Directory, schemas::GenerateSchema};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub mapping: Option<HashMap<String, Vec<String>>>,
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

pub fn create_space_graph(root_directory: Directory) -> SpaceGraph {
    let mut space_graph = HashMap::new();

    add_to_spaces_graph(root_directory, &mut space_graph, None);

    space_graph
}

fn add_to_spaces_graph(
    mut dir: Directory,
    space_graph: &mut SpaceGraph,
    closest_parent_space: Option<String>,
) {
    let space_name = dir
        .space
        .as_ref()
        .map(|s| s.schema.name.to_string())
        .or_else(|| closest_parent_space.clone());
    if let Some(space) = dir.space.take() {
        let space = Space {
            name: space.schema.name,
            path: dir.path.clone(),
            dependencies: space.schema.dependencies.unwrap_or_default(),
            mapping: space.schema.mapping.map(|m| {
                let mut map = HashMap::new();
                for mapping in m {
                    map.entry(mapping.from).or_insert(vec![]).push(mapping.to);
                }
                map
            }),
            environments: space.schema.environments.unwrap_or_default(),
            variables: space.variables,
            files_to_copy: resolve_files_to_copy(&dir),
            parent_space: closest_parent_space,
            generate: {
                match space.schema.generate {
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
        add_to_spaces_graph(entry, space_graph, space_name.clone());
    }
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
