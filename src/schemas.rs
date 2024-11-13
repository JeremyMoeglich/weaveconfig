use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Deserialize, Debug, Clone, PartialEq)]
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
    /// A mapping from the environments in this space to the environments in the parent space.
    pub space_to_parent_mapping: Option<HashMap<String, HashSet<String>>>,
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

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum GenerateSchema {
    /// Toggle full generation on or off.
    ShouldGenerate(bool),
    /// Customize the generated files. This always includes the config.json, bindings can be toggled individually.
    Generate(GenerateObjectSchema),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct GenerateObjectSchema {
    /// Toggle the typescript bindings on or off.
    pub typescript: bool,
}
