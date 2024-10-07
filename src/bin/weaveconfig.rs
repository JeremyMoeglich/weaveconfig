use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use anyhow::Context;
use clap::{Parser, Subcommand};
use weaveconfig::generate_weaveconfig;

#[derive(Parser)]
#[command(
    name = "weaveconfig-cli",
    version = "0.1.2",
    author = "Jeremy Moeglich <jeremy@moeglich.dev>",
    about = "A CLI to manage weaveconfig configurations"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes a new weaveconfig in the specified directory
    Init {
        /// Directory to initialize the weaveconfig
        #[arg(default_value = ".")]
        dir: String,
    },
    /// Generates the weaveconfig configuration
    Generate {
        /// Path to the directory to generate the configuration for
        #[arg(default_value = ".")]
        path: String,
    },
    /// Generates the weaveconfig configuration
    Gen {
        /// Path to the directory to generate the configuration for
        #[arg(default_value = ".")]
        path: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { dir } => {
            // Handle `init` command
            let init_path = Path::new(&dir);
            let init_path = init_path
                .canonicalize()
                .with_context(|| format!("The path {:?} does not exist", init_path))?;

            println!("Initializing weaveconfig in directory: {:?}", init_path);
            tokio::fs::create_dir(init_path.join("weaveconfig")).await?;
        }
        Commands::Generate { path } | Commands::Gen { path } => {
            // Handle `generate` command
            let path = Path::new(&path);
            generate_config(path).await?;
        }
    }

    Ok(())
}

async fn generate_config(path: &Path) -> Result<(), anyhow::Error> {
    let path = path
        .canonicalize()
        .with_context(|| format!("The path {:?} does not exist", path))?;
    let root = locate_root(&path).with_context(|| {
                format!(
                    "Any of the parent directories must contain a 'weaveconfig' directory. None of {:?} and its parents do.",
                    path.display()
                )
            })?;
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
