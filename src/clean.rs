use crate::domain::Target;
use anyhow::{Context, Result};
use std::path::Path;

pub fn clean_target_output_paths(target: &Target) -> Result<()> {
    for output_path in &target.outputs.paths {
        clean_path(output_path)?;
    }

    Ok(())
}

fn clean_path(path: &Path) -> Result<()> {
    if path.exists() {
        if path.is_file() {
            std::fs::remove_file(&path)
                .with_context(|| format!("Failed to remove file {}", path.display()))?;
        } else if path.is_dir() {
            std::fs::remove_dir_all(&path)
                .with_context(|| format!("Failed to remove directory {}", path.display()))?;
        } else {
            log::warn!("Failed to remove {}", path.display())
        }
    }

    Ok(())
}
