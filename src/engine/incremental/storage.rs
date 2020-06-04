use super::TargetEnvState;
use crate::domain::Target;
use crate::work_dir;
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::path::PathBuf;

/// File where the state of the target inputs and outputs are stored upon successful build.
fn get_checksums_file_path(target: &Target) -> PathBuf {
    work_dir::get_work_dir_path(&target.project.dir).join(format!("{}.checksums", target.name))
}

pub fn read_saved_target_env_state(target: &Target) -> Result<Option<TargetEnvState>> {
    let file_path = get_checksums_file_path(target);
    if file_path.exists() {
        let file = File::open(&file_path)
            .with_context(|| format!("Failed to open checksums file {}", file_path.display()))?;
        match bincode::deserialize_from(file) {
            Ok(checksums) => Ok(Some(checksums)),
            Err(e) => {
                log::trace!(
                    "{} - Dropping corrupted checksums file (Error: {})",
                    target,
                    e
                );
                delete_saved_env_state(&target)?;
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

pub fn delete_saved_env_state(target: &Target) -> Result<()> {
    let checksums_file = get_checksums_file_path(target);
    if checksums_file.exists() {
        fs::remove_file(&checksums_file).with_context(|| {
            format!(
                "Failed to delete checksums file {}",
                checksums_file.display()
            )
        })?;
    }
    Ok(())
}

pub fn save_env_state(target: &Target, env_state: &TargetEnvState) -> Result<()> {
    fs::create_dir(work_dir::get_work_dir_path(&target.project.dir)).ok();

    let file_path = get_checksums_file_path(target);
    let file = File::create(&file_path)
        .with_context(|| format!("Failed to create checksums file {}", file_path.display()))?;
    bincode::serialize_into(file, env_state)
        .with_context(|| format!("Failed to serialize checksums for {}", target))
}
