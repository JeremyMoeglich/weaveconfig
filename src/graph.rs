use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context};
use futures::{stream::FuturesOrdered, StreamExt};

use crate::schemas::SpaceSchema;

pub struct Directory {
    pub directories: Vec<Arc<Directory>>,
    pub path: PathBuf,
    pub parent: Option<PathBuf>,
    pub space: Option<Space>,
    pub rest_to_copy: Vec<PathBuf>,
}

pub struct Space {
    pub dependencies: Vec<Dependency>,
    pub mapping: Option<HashMap<String, Vec<String>>>,
    pub environments: HashSet<String>,
    pub variables: Option<serde_json::Value>,
}

pub struct Dependency {
    pub path: PathBuf,
    pub template: Option<String>,
    pub keys: Option<Vec<String>>,
}

pub type Graph = HashMap<PathBuf, Arc<Directory>>;

pub async fn create_graph(envoyr_config_root: PathBuf) -> Result<Graph, anyhow::Error> {
    let path = envoyr_config_root.canonicalize()?;

    let mut root_directory = Directory {
        directories: Vec::new(),
        path,
        parent: None,
        space: None,
        rest_to_copy: Vec::new(),
    };

    locate_directories(&mut root_directory).await?;

    let mut graph = HashMap::new();
    insert_into_graph(Arc::new(root_directory), &mut graph);

    Ok(graph)
}

fn insert_into_graph(directory: Arc<Directory>, graph: &mut Graph) {
    for sub_directory in directory.directories.iter() {
        insert_into_graph(sub_directory.clone(), graph);
    }
    graph.insert(directory.path.clone(), directory);
}

async fn locate_directories(directory: &mut Directory) -> Result<(), anyhow::Error> {
    let mut entries = tokio::fs::read_dir(&directory.path).await?;

    let mut futures = FuturesOrdered::new();
    let mut variables: Option<serde_json::Value> = None;

    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        let entry_path = entry.path();

        if metadata.is_dir() {
            let parent_path = directory.path.clone();
            futures.push_back(Box::pin(async move {
                let mut sub_directory = Directory {
                    directories: Vec::new(),
                    path: entry_path,
                    parent: Some(parent_path),
                    space: None,
                    rest_to_copy: Vec::new(),
                };

                if let Err(e) = locate_directories(&mut sub_directory).await {
                    return Err(e);
                }
                Ok(sub_directory)
            }));
        } else {
            let file_type = process_file(entry_path).await?;
            match file_type {
                FileType::Space(space) => {
                    if directory.space.is_some() {
                        return Err(anyhow!(
                            "Directory {:?} has multiple space configs",
                            directory.path
                        ));
                    }
                    directory.space = Some(space);
                }
                FileType::Variables(value) => match (&mut variables, value) {
                    (None, value) => variables = Some(value),
                    (Some(serde_json::Value::Object(main_map)), serde_json::Value::Object(map)) => {
                        main_map.extend(map);
                    }
                    (_, value) => {
                        return Err(anyhow!(
                            "Directory {:?} has conflicting variable configs, {:?} can not be merged with {:?}",
                            directory.path,
                            variables,
                            value
                        ));
                    }
                },
                FileType::Rest(path) => {
                    directory.rest_to_copy.push(path);
                }
            }
        }
    }

    match (&mut directory.space, variables) {
        (Some(space), Some(variables)) => {
            space.variables = Some(variables);
        }
        (None, Some(_)) => {
            return Err(anyhow!(
                "Directory {:?} has variables but no space config",
                directory.path
            ));
        }
        _ => {}
    }

    while let Some(sub_directory) = futures.next().await {
        match sub_directory {
            Ok(sub_directory) => directory.directories.push(Arc::new(sub_directory)),
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

enum FileType {
    Space(Space),
    Variables(serde_json::Value),
    Rest(PathBuf),
}

async fn process_file(file_path: PathBuf) -> Result<FileType, anyhow::Error> {
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Invalid file name for path: {:?}", file_path))?;

    if file_name.starts_with('_') {
        let segments: Vec<&str> = file_name.split('.').collect();
        match segments.as_slice() {
            ["_space", ext] => {
                validate_json_extension(ext, file_name)?;
                let content = read_file_to_string(&file_path).await?;
                let space_schema: SpaceSchema =
                    serde_json::from_str(&content).context("Failed to parse SpaceSchema")?;
                let space = space_schema.into_space()?;
                Ok(FileType::Space(space))
            }
            ["_env", ext] => {
                validate_json_extension(ext, file_name)?;
                let content = read_file_to_string(&file_path).await?;
                let map: serde_json::Value =
                    serde_json::from_str(&content).context("Failed to parse JSON variables")?;
                Ok(FileType::Variables(map))
            }
            [prefix, "env", ext] if prefix.starts_with('_') => {
                validate_json_extension(ext, file_name)?;
                let content = read_file_to_string(&file_path).await?;
                let variables: serde_json::Value =
                    serde_json::from_str(&content).context("Failed to parse JSON variables")?;
                // Remove the leading '_' from prefix
                let prefix = prefix.trim_start_matches('_').to_string();
                let mut map = serde_json::Map::new();
                map.insert(prefix, variables);
                let map = serde_json::Value::Object(map);
                Ok(FileType::Variables(map))
            }
            _ => Err(anyhow!("Invalid file name format: {}", file_name)),
        }
    } else {
        Ok(FileType::Rest(file_path))
    }
}

/// Validates that the extension is either "json" or "jsonc".
fn validate_json_extension(ext: &str, file_name: &str) -> Result<(), anyhow::Error> {
    match ext {
        "json" | "jsonc" => Ok(()),
        _ => Err(anyhow!(
            "Invalid file extension for {}. Expected .json or .jsonc, got .{}",
            file_name,
            ext
        )),
    }
}

/// Reads the entire contents of a file asynchronously as a String.
async fn read_file_to_string(path: &Path) -> Result<String, anyhow::Error> {
    tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read file: {:?}", path))
}
