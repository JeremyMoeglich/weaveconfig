use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// The _space.jsonc file.
/// A space describes a folder and its configuration.
/// Each space can have multiple environments, each with their own values for the variables in the space.
pub struct SpaceSchema {
    /// The name of the space. This is used to identify the space in the graph.
    /// Dependencies reference spaces by their name.
    /// It must be unique within the graph.
    pub name: String,
    /// A list of dependencies that this space imports.
    /// Each element must be a name of another space.
    /// If not present, the space will not import any dependencies.
    pub dependencies: Option<Vec<String>>,
    /// A mapping between the environments from dependencies to the environments in this space.
    /// If not present the environments from the dependencies will be used as is.
    /// If this field is present, the environments field must be set.
    pub mapping: Option<Vec<MappingSchema>>,
    /// A list of environments that this space supports.
    /// An environment describes a particular configuration of the space
    /// for example, prod, dev, staging, etc.
    /// If not present, the space will have a single unnamed environment with just the global variables.
    pub environments: Option<HashSet<String>>,
    /// weaveconfig can generate a /gen folder in the folder this space maps to.
    /// This folder contains the config.json itself, as well as the typescript bindings to that config.
    /// This is enabled by default, and can be disabled by setting this to false.
    pub generate: Option<GenerateSchema>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum GenerateSchema {
    /// Toggle full generation on or off.
    ShouldGenerate(bool),
    /// Customize the generated files. This always includes the config.json, bindings can be toggled individually.
    Generate(GenerateObjectSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GenerateObjectSchema {
    /// Toggle the typescript bindings on or off.
    pub typescript: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// A mapping between the environments from dependencies to the environments in this space.
pub struct MappingSchema {
    /// The name of the environment in the dependency.
    pub from: String,
    /// The name of the environment in this space.
    pub to: String,
}
