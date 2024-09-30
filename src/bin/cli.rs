use std::path::{Path, PathBuf};

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use clap::{arg, command, Arg};

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
    let path = path.canonicalize().unwrap();
    let root = locate_root(&path).unwrap();
    println!("Root: {}", root.display());

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
