use crate::domain::Target;
use anyhow::{Context, Result};
use async_std::fs;
use async_std::path::Path;

pub async fn clean_target_output_paths(target: &Target) -> Result<()> {
    if let Some(output) = target.output() {
        for resource in &output.files {
            if resource.extensions.is_some() {
                let resource_files =
                    crate::fs::list_files_in_paths(&resource.paths, &resource.extensions).await;
                for file in resource_files {
                    fs::remove_file(&file)
                        .await
                        .with_context(|| format!("Failed to remove file {}", file.display()))?;
                }
            } else {
                for output_path in &resource.paths {
                    clean_path(output_path).await?;
                }
            }
        }
    }

    Ok(())
}

async fn clean_path(path: &Path) -> Result<()> {
    if path.exists().await {
        if path.is_file().await {
            fs::remove_file(&path)
                .await
                .with_context(|| format!("Failed to remove file {}", path.display()))?;
        } else if path.is_dir().await {
            fs::remove_dir_all(&path)
                .await
                .with_context(|| format!("Failed to remove directory {}", path.display()))?;
        } else {
            log::warn!("Failed to remove {}", path.display())
        }
    }

    Ok(())
}
