use std::path::Path;

use anyhow::Result;
use apply_resolved::apply_resolved;
use file_graph::traverse_directory;
use resolve_spaces::resolve_spaces;
use space_graph::create_space_graph;

mod ancestor_mapping;
mod apply_resolved;
mod file_graph;
mod get_environment_value;
mod map_path;
mod merging;
mod parse_jsonc;
mod resolve_spaces;
mod schemas;
mod space_graph;
mod template_file;
mod ts_binding;
mod write_json_file;

pub async fn generate_weaveconfig(weaveconfig_config_root: &Path) -> Result<()> {
    let directory = traverse_directory(weaveconfig_config_root).await?;
    let space_graph = create_space_graph(directory)?;
    let resolved_spaces = resolve_spaces(space_graph)?;
    apply_resolved(resolved_spaces, weaveconfig_config_root).await
}
