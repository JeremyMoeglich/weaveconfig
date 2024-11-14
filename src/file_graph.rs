use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use futures::{stream::FuturesOrdered, StreamExt};

use crate::{merging::merge_map_consume, parse_jsonc::parse_jsonc, schemas::SpaceInfo};

#[derive(Debug, Clone, PartialEq)]
pub struct Directory {
    pub directories: Vec<Directory>,
    pub path: PathBuf,
    pub parent_directory: Option<PathBuf>,
    pub space: Option<SpaceNode>,
    pub rest_to_copy: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpaceNode {
    pub info: SpaceInfo,
    pub variables: Option<serde_json::Map<String, serde_json::Value>>,
}

const FORENV_PREFIX: &str = "_forenv";

/// Creates a graph of the weaveconfig configuration.
/// The root of the graph is typically the `weaveconfig` directory within the project root.
pub async fn traverse_directory(
    weaveconfig_config_root: &Path,
) -> Result<Directory, anyhow::Error> {
    let path = weaveconfig_config_root
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize path: {:?}", weaveconfig_config_root))?;

    let mut root_directory = Directory {
        directories: Vec::new(),
        path,
        parent_directory: None,
        space: None,
        rest_to_copy: Vec::new(),
    };

    locate_directories(&mut root_directory).await?;

    Ok(root_directory)
}

async fn locate_directories(directory: &mut Directory) -> Result<(), anyhow::Error> {
    let mut entries = tokio::fs::read_dir(&directory.path)
        .await
        .with_context(|| format!("Failed to read directory: {:?}", directory.path))?;

    let mut futures = FuturesOrdered::new();
    let mut variables: Option<serde_json::Map<String, serde_json::Value>> = None;
    let mut validation_schema: Option<serde_json::Value> = None;

    while let Some(entry) = entries
        .next_entry()
        .await
        .with_context(|| format!("Failed to read entry in directory: {:?}", directory.path))?
    {
        let metadata = entry
            .metadata()
            .await
            .with_context(|| format!("Failed to get metadata for entry: {:?}", entry.path()))?;
        let entry_path = entry.path();

        if metadata.is_dir() {
            let parent_path = directory.path.clone();
            futures.push_back(Box::pin(async move {
                let mut sub_directory = Directory {
                    directories: Vec::new(),
                    path: entry_path.clone(),
                    parent_directory: Some(parent_path.clone()),
                    space: None,
                    rest_to_copy: Vec::new(),
                };

                if let Err(e) = locate_directories(&mut sub_directory).await {
                    return Err(e.context(format!(
                        "Failed to locate subdirectories in {:?}",
                        entry_path
                    )));
                }
                Ok(sub_directory)
            }));
        } else {
            let file_type = process_file(entry_path.clone())
                .await
                .with_context(|| format!("Failed to process file: {:?}", entry_path))?;
            match file_type {
                FileType::Space(space) => {
                    if directory.space.is_some() {
                        return Err(anyhow!(
                            "Directory {:?} has multiple space configurations. Only one '_space.json' file is allowed per directory.",
                            directory.path
                        ));
                    }
                    directory.space = Some(SpaceNode {
                        info: space,
                        variables: None,
                    });
                }
                FileType::Schema(schema) => {
                    validation_schema = Some(schema);
                }
                FileType::Variables(value) => match (&mut variables, value) {
                    (None, value) => variables = Some(value),
                    (Some(main_map), map) => {
                        merge_map_consume(main_map, map)?;
                    }
                },
                FileType::Rest(path) => {
                    directory.rest_to_copy.push(path);
                }
            }
        }
    }

    match (&mut directory.space, variables, validation_schema) {
        (Some(space), Some(variables), schema) => {
            if let Some(schema) = schema {
                validate_space_schema(space, &variables, schema)?;
            }

            space.variables = Some(variables);
        }
        (Some(_), None, Some(_)) => {
            return Err(anyhow!(
                "Directory {:?} contains a schema but no variables, for example '_env.json'.",
                directory.path
            ));
        }
        (None, Some(_), _) => {
            return Err(anyhow!(
                "Directory {:?} contains variables but no '_space.json' configuration file.",
                directory.path
            ));
        }
        (None, None, Some(_)) => {
            return Err(anyhow!(
                "Directory {:?} contains a schema but no '_space.json' configuration file.",
                directory.path
            ));
        }
        _ => {}
    }

    while let Some(sub_directory) = futures.next().await {
        match sub_directory {
            Ok(sub_directory) => directory.directories.push(sub_directory),
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

enum FileType {
    Space(SpaceInfo),
    Variables(serde_json::Map<String, serde_json::Value>),
    Schema(serde_json::Value),
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
                let content = read_file_to_string(&file_path)
                    .await
                    .with_context(|| format!("Failed to read space configuration file: {:?}", file_path))?;
                let space_schema: SpaceInfo = parse_jsonc(&content).with_context(|| {
                    format!(
                        "Failed to parse JSON in space configuration file: {:?}",
                        file_path
                    )
                })?;
                Ok(FileType::Space(space_schema))
            }
            ["_env", ext] => {
                validate_json_extension(ext, file_name)?;
                let content = read_file_to_string(&file_path)
                    .await
                    .with_context(|| format!("Failed to read variables file: {:?}", file_path))?;
                let map: serde_json::Map<String, serde_json::Value> = parse_jsonc(&content)
                    .with_context(|| format!("Failed to parse JSON variables in file: {:?}", file_path))?;
                Ok(FileType::Variables(map))
            }
            [prefix, "env", ext] if prefix.starts_with('_') => {
                validate_json_extension(ext, file_name)?;
                let content = read_file_to_string(&file_path)
                    .await
                    .with_context(|| format!("Failed to read prefixed variables file: {:?}", file_path))?;
                let variables: serde_json::Map<String, serde_json::Value> =
                    parse_jsonc(&content).with_context(|| {
                        format!(
                            "Failed to parse JSON variables in prefixed file: {:?}",
                            file_path
                        )
                    })?;
                // Remove the leading '_' from prefix
                let prefix = prefix.trim_start_matches('_').to_string();
                let mut map = serde_json::Map::new();
                map.insert(prefix, serde_json::Value::Object(variables));
                Ok(FileType::Variables(map))
            }
            ["_schema", ext] => {
                validate_json_extension(ext, file_name)?;
                let content = read_file_to_string(&file_path)
                    .await
                    .with_context(|| format!("Failed to read schema file: {:?}", file_path))?;
                let schema: serde_json::Value = parse_jsonc(&content)
                    .with_context(|| format!("Failed to parse JSON schema in file: {:?}", file_path))?;
                Ok(FileType::Schema(schema))
            }
            segments if segments.first() == Some(&FORENV_PREFIX) => {
                Ok(FileType::Rest(file_path))
            }
            _ => Err(anyhow!(
                "Invalid file name format: '{}'. Expected '_space.json', '_env.json', '_<prefix>_env.json', '_schema.json' or '_forenv.<rest>'.",
                file_name
            )),
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
            "Invalid file extension for '{}'. Expected '.json' or '.jsonc', got '.{}'.",
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

fn validate_space_schema(
    space: &SpaceNode,
    variables: &serde_json::Map<String, serde_json::Value>,
    schema: serde_json::Value,
) -> Result<(), anyhow::Error> {
    let validator = jsonschema::Validator::new(&schema).with_context(|| {
        format!(
            "Failed to create validator for space schema: {:?} for space: {:?}",
            schema, space.info.name
        )
    })?;

    if space
        .info
        .environments
        .as_ref()
        .unwrap_or(&HashSet::new())
        .is_empty()
    {
        // Validate on the top level
        let object = serde_json::Value::Object(variables.clone());
        if let Err(e) = validator.validate(&object) {
            return Err(anyhow!(
                "Failed to validate variables against space schema: {}",
                e
            ));
        }
    } else {
        // Validate on the environment level
        for environment in space.info.environments.as_ref().unwrap() {
            let object = variables
                .get(environment)
                .ok_or_else(|| {
                    anyhow!(
                        "Environment {} not found in variables, but required due to the presence of a schema.",
                        environment
                    )
                })?;
            if let Err(e) = validator.validate(object) {
                return Err(anyhow!(
                    "Failed to validate variables of environment {} against space schema: {}",
                    environment,
                    e
                ));
            }
        }
    }

    Ok(())
}
