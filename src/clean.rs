use crate::domain::Target;
use anyhow::{Context, Result};
use async_std::path::Path;

pub async fn clean_target_output_paths(target: &Target) -> Result<()> {
    if let Some(output) = target.output() {
        for output_path in &output.paths {
            clean_path(output_path).await?;
        }
    }

    Ok(())
}

async fn clean_path(path: &Path) -> Result<()> {
    if path.exists().await {
        if path.is_file().await {
            std::fs::remove_file(&path)
                .with_context(|| format!("Failed to remove file {}", path.display()))?;
        } else if path.is_dir().await {
            std::fs::remove_dir_all(&path)
                .with_context(|| format!("Failed to remove directory {}", path.display()))?;
        } else {
            log::warn!("Failed to remove {}", path.display())
        }
    }

    Ok(())
}
