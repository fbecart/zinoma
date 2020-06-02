mod cmd_outputs;
mod env_vars;
mod fs;

use crate::domain::{EnvProbes, Target};
use anyhow::{Context, Error, Result};
use cmd_outputs::EnvCmdOutputsState;
use env_vars::EnvVarsState;
use fs::EnvFsState;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::ErrorKind;
use std::path::{self, Path, PathBuf};

/// Name of the directory in which Žinoma stores checksums of the targets inputs and outputs.
const CHECKSUMS_DIR_NAME: &str = ".zinoma";

#[derive(PartialEq)]
pub enum IncrementalRunResult<T> {
    Skipped,
    Run(T),
}

pub fn run<T, F>(target: &Target, function: F) -> Result<IncrementalRunResult<Result<T>>>
where
    F: Fn() -> Result<T>,
{
    if env_state_has_not_changed_since_last_successful_execution(target)? {
        return Ok(IncrementalRunResult::Skipped);
    }

    delete_saved_env_state(&target)?;

    let result = function();

    if result.is_ok() {
        if let Some(env_state) = compute_target_env_state(target)? {
            save_env_state(&target, &env_state)?;
        }
    }

    Ok(IncrementalRunResult::Run(result))
}

pub fn is_in_checksums_dir(path: &Path) -> bool {
    path.components().any(|component| match component {
        path::Component::Normal(name) => name == CHECKSUMS_DIR_NAME,
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::is_in_checksums_dir;
    use std::path::Path;

    #[test]
    fn test_is_in_checksums_dir() {
        assert!(is_in_checksums_dir(Path::new(".zinoma/my/file.json")));
        assert!(is_in_checksums_dir(Path::new(
            "/my/project/.zinoma/my/file.json"
        )));
        assert!(!is_in_checksums_dir(Path::new("/my/file.json")));
    }
}

fn get_checksums_dir_path(project_dir: &Path) -> PathBuf {
    project_dir.join(CHECKSUMS_DIR_NAME)
}

fn get_checksums_file_path(target: &Target) -> PathBuf {
    get_checksums_dir_path(&target.project.dir).join(format!("{}.checksums", target.name))
}

fn env_state_has_not_changed_since_last_successful_execution(target: &Target) -> Result<bool> {
    let saved_state = read_saved_target_env_state(target)
        .with_context(|| format!("Failed to read saved env state for {}", target.name))?;

    match saved_state {
        Some(saved_state) => saved_state.eq_current_state(target).with_context(|| {
            format!(
                "Failed to compare saved env state with current env state for {}",
                target.name
            )
        }),
        _ => Ok(false),
    }
}

fn read_saved_target_env_state(target: &Target) -> Result<Option<TargetEnvState>> {
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
        std::fs::remove_file(&checksums_file).with_context(|| {
            format!(
                "Failed to delete checksums file {}",
                checksums_file.display()
            )
        })?;
    }
    Ok(())
}

fn save_env_state(target: &Target, checksums: &TargetEnvState) -> Result<()> {
    std::fs::create_dir(get_checksums_dir_path(&target.project.dir)).ok();

    let file_path = get_checksums_file_path(target);
    let file = File::create(&file_path)
        .with_context(|| format!("Failed to create checksums file {}", file_path.display()))?;
    bincode::serialize_into(file, checksums)
        .with_context(|| format!("Failed to serialize checksums for {}", target.name))
}

pub fn remove_checksums_dir(project_dir: PathBuf) -> Result<()> {
    let checksums_dir = get_checksums_dir_path(&project_dir);
    match std::fs::remove_dir_all(&checksums_dir) {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => {
            return Err(Error::new(e).context(format!(
                "Failed to remove checksums directory {}",
                checksums_dir.display()
            )));
        }
    }

    Ok(())
}

fn compute_target_env_state(target: &Target) -> Result<Option<TargetEnvState>> {
    if target.inputs.is_empty() {
        Ok(None)
    } else {
        let project_dir = &target.project.dir;
        Ok(Some(TargetEnvState {
            inputs: hash_env(&target.inputs, project_dir)?,
            outputs: hash_env(&target.outputs, project_dir)?,
        }))
    }
}

// TODO Hash or checksum? make up your mind
// Hash is function, checksum is result
fn hash_env(env_probes: &EnvProbes, project_dir: &Path) -> Result<EnvState> {
    Ok(EnvState {
        fs: EnvFsState::current(&env_probes.paths)?,
        cmd_stdouts: EnvCmdOutputsState::current(&env_probes.cmd_outputs, project_dir)?,
        vars: EnvVarsState::current(&env_probes.env_vars)?,
    })
}

#[derive(Serialize, Deserialize, PartialEq)]
struct TargetEnvState {
    inputs: EnvState,
    outputs: EnvState,
}

impl TargetEnvState {
    fn eq_current_state(&self, target: &Target) -> Result<bool> {
        let project_dir = &target.project.dir;
        Ok(self.inputs.eq_current_state(&target.inputs, &project_dir)?
            && self
                .outputs
                .eq_current_state(&target.outputs, &project_dir)?)
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
struct EnvState {
    fs: EnvFsState,
    cmd_stdouts: EnvCmdOutputsState,
    vars: EnvVarsState,
}

impl EnvState {
    fn eq_current_state(&self, env_probes: &EnvProbes, project_dir: &Path) -> Result<bool> {
        Ok((&self.fs).eq_current_state(&env_probes.paths)?
            && (&self.cmd_stdouts).eq_current_state(&env_probes.cmd_outputs, project_dir)?
            && (&self.vars).eq_current_state(&env_probes.env_vars)?)
    }
}

// TODO Run all computations in parallel?
