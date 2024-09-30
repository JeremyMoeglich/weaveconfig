use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::graph::{Dependency, Space};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SpaceSchema {
    dependencies: Option<Vec<DependencySchema>>,
    mapping: Option<Vec<MappingSchema>>,
    environments: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct MappingSchema {
    from: String,
    to: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum DependencySchema {
    Path(PathBuf),
    DependencyObject(DependencyObjectSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct DependencyObjectSchema {
    path: PathBuf,
    template: Option<String>,
    keys: Option<Vec<String>>,
}

impl SpaceSchema {
    pub fn into_space(self) -> Result<Space, std::io::Error> {
        Ok(Space {
            dependencies: self
                .dependencies
                .unwrap_or_default()
                .into_iter()
                .map(|d| d.into_dependency())
                .collect::<Result<Vec<Dependency>, std::io::Error>>()?,
            mapping: self.mapping.map(|m| {
                let mut map = HashMap::new();
                for mapping in m {
                    map.entry(mapping.from).or_insert(vec![]).push(mapping.to);
                }
                map
            }),
            environments: self.environments.into_iter().collect(),
            variables: None,
        })
    }
}

impl DependencySchema {
    fn into_dependency(self) -> Result<Dependency, std::io::Error> {
        let (path, template, keys) = match self {
            DependencySchema::Path(path) => (path, None, None),
            DependencySchema::DependencyObject(dependency_object) => (
                dependency_object.path,
                dependency_object.template,
                dependency_object.keys,
            ),
        };

        // check if path exists
        let path = std::fs::canonicalize(path)?;

        Ok(Dependency {
            path,
            template,
            keys,
        })
    }
}
