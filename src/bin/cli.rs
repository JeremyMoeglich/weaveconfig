use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use anyhow::Context;
use clap::{arg, command};
use envoyr::{graph::create_graph, resolve_spaces::resolve_spaces, write_env_file::write_env_file};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let matches = command!()
        .version("0.1.0")
        .author("Jeremy Moeglich <jeremy@moeglich.dev>")
        .about("A simple CLI to generate Envoyr configuration")
        .arg(arg!([path] "Path to the directory to generate the configuration for").required(false))
        .get_matches();

    let current_dir = String::from(".");
    let path = matches.get_one::<String>("path").unwrap_or(&current_dir);

    let path = Path::new(path);
    let path = path
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize path: {:?}", path))?;
    let root = locate_root(&path)
        .with_context(|| format!("Failed to locate root, starting from {:?}", path.display()))?;
    let envoyr_config_root = root.join("envoyr");
    let graph = create_graph(envoyr_config_root).await?;
    let spaces = resolve_spaces(&graph)?;
    for space in spaces.values() {
        write_env_file(space).await?;
    }
    Ok(())
}

fn locate_root(path: &Path) -> Option<PathBuf> {
    // The root is the first directory that contains a "envoyr" directory
    let root = path.ancestors().find(|dir| is_root(dir))?;
    Some(root.to_path_buf())
}

fn is_root(path: &Path) -> bool {
    path.join("envoyr").exists()
}
