use std::{collections::HashMap, path::PathBuf};

use crate::file_graph::Directory;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub mapping: Option<HashMap<String, Vec<String>>>,
    pub environments: HashSet<String>,
    pub variables: Option<serde_json::Map<String, serde_json::Value>>,
    pub files_to_copy: Vec<PathBuf>,
    pub parent_space: Option<String>,
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
        let mut files_to_copy = vec![];
        resolve_files_to_copy(&dir, &mut files_to_copy);
        let mut space = Space {
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
            files_to_copy: vec![],
            parent_space: closest_parent_space,
        };
        resolve_files_to_copy(&dir, &mut space.files_to_copy);
        space_graph.insert(space.name.clone(), space);
    }

    for entry in dir.directories {
        add_to_spaces_graph(entry, space_graph, space_name.clone());
    }
}

fn resolve_files_to_copy(dir: &Directory, files: &mut Vec<PathBuf>) {
    for file in &dir.rest_to_copy {
        files.push(file.clone());
    }

    for entry in &dir.directories {
        if entry.space.is_none() {
            resolve_files_to_copy(&entry, files)
        }
    }
}