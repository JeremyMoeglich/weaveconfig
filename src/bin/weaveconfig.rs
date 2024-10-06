use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use anyhow::Context;
use clap::{arg, command};
use weaveconfig::generate_weaveconfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let matches = command!()
        .version("0.1.0")
        .author("Jeremy Moeglich <jeremy@moeglich.dev>")
        .about("A CLI to generate a weaveconfig configuration")
        .arg(arg!([path] "Path to the directory to generate the configuration for").required(false))
        .get_matches();

    let current_dir = String::from(".");
    let path = matches.get_one::<String>("path").unwrap_or(&current_dir);

    let path = Path::new(path);
    let path = path
        .canonicalize()
        .with_context(|| format!("The path {:?} does not exist", path))?;
    let root = locate_root(&path)
        .with_context(|| format!("Any of the parent directories must contain a 'weaveconfig' directory. None of {:?} and its parents do.", path.display()))?;
    let weaveconfig_config_root = root.join("weaveconfig").canonicalize()?;
    generate_weaveconfig(&weaveconfig_config_root).await?;
    Ok(())
}

fn locate_root(path: &Path) -> Option<PathBuf> {
    // The root is the first directory that contains a "weaveconfig" directory
    let root = path.ancestors().find(|dir| is_root(dir))?;
    Some(root.to_path_buf())
}

fn is_root(path: &Path) -> bool {
    path.join("weaveconfig").exists()
}
