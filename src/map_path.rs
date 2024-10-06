use std::path::{Path, PathBuf};

use anyhow::Context;

pub fn map_path(weaveconfig_root: &Path, path: &Path) -> Result<PathBuf, anyhow::Error> {
    // Canonicalize both the root and the path
    let canonical_root = weaveconfig_root
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize root path: {}", weaveconfig_root.display()))?;
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?;

    // Get the parent of the canonicalized root to remove the last segment
    let trimmed_root = canonical_root.parent().context("Root has no parent")?;

    // Strip the prefix (canonicalized root) from the canonicalized path
    let relative_path = canonical_path
        .strip_prefix(&canonical_root)
        .with_context(|| {
            format!(
                "Path {} is not within root {}",
                canonical_path.display(),
                canonical_root.display()
            )
        })?;

    // Construct the new path by appending the relative path to trimmed_root
    Ok(trimmed_root.join(relative_path))
}
