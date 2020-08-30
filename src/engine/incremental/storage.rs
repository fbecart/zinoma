use super::TargetEnvState;
use crate::domain::TargetMetadata;
use crate::work_dir;
use anyhow::{Context, Result};
use async_std::fs;
use async_std::{path::PathBuf, task};

/// File where the state of the target inputs and outputs are stored upon successful build.
fn get_checksums_file_path(target: &TargetMetadata) -> PathBuf {
    work_dir::get_work_dir_path(&target.project_dir).join(format!("{}.checksums", target))
}

pub async fn read_saved_target_env_state(target: &TargetMetadata) -> Option<TargetEnvState> {
    let file_path = get_checksums_file_path(target);
    if !&file_path.exists().await {
        return None;
    }

    let result = {
        let target_id = target.id.clone();
        let file_path = file_path.clone();

        task::spawn_blocking(move || {
            let file = std::fs::File::open(&file_path).with_context(|| {
                format!("Failed to open checksums file {}", &file_path.display())
            })?;
            bincode::deserialize_from(file)
                .with_context(|| format!("Failed to deserialize checksums for {}", target_id))
        })
        .await
    };

    if let Err(e) = &result {
        log::debug!(
            "{} - Dropping corrupted checksums file (Error: {})",
            target,
            e
        );
        if let Err(e) = delete_saved_env_state(&target).await {
            log::error!(
                "{} - Failed to drop corrupted checksum file: {} (Error: {})",
                target,
                file_path.display(),
                e
            )
        }
    }

    result.ok()
}

pub async fn delete_saved_env_state(target: &TargetMetadata) -> Result<()> {
    let checksums_file = get_checksums_file_path(target);
    if checksums_file.exists().await {
        fs::remove_file(&checksums_file).await.with_context(|| {
            format!(
                "Failed to delete checksums file {}",
                checksums_file.display()
            )
        })?;
    }

    Ok(())
}

pub async fn save_env_state(target: &TargetMetadata, env_state: TargetEnvState) -> Result<()> {
    fs::create_dir(work_dir::get_work_dir_path(&target.project_dir))
        .await
        .ok();

    let file_path = get_checksums_file_path(target);
    let target_id = target.id.clone();
    task::spawn_blocking(move || {
        let file = std::fs::File::create(&file_path)
            .with_context(|| format!("Failed to create checksums file {}", file_path.display()))?;
        bincode::serialize_into(file, &env_state)
            .with_context(|| format!("Failed to serialize checksums for {}", target_id))
    })
    .await
}
