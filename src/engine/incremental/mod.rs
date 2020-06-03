mod env_state;
pub mod storage;

use crate::domain::Target;
use anyhow::{Context, Result};
use env_state::EnvState;
use serde::{Deserialize, Serialize};

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

    storage::delete_saved_env_state(&target)?;

    let result = function();

    if result.is_ok() {
        if let Some(env_state) = TargetEnvState::current(target)? {
            storage::save_env_state(&target, &env_state)?;
        }
    }

    Ok(IncrementalRunResult::Run(result))
}

fn env_state_has_not_changed_since_last_successful_execution(target: &Target) -> Result<bool> {
    let saved_state = storage::read_saved_target_env_state(target)
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

#[derive(Serialize, Deserialize, PartialEq)]
pub struct TargetEnvState {
    inputs: EnvState,
    outputs: EnvState,
}

impl TargetEnvState {
    pub fn current(target: &Target) -> Result<Option<Self>> {
        if target.inputs.is_empty() {
            Ok(None)
        } else {
            let project_dir = &target.project.dir;
            Ok(Some(TargetEnvState {
                inputs: EnvState::current(&target.inputs, project_dir)?,
                outputs: EnvState::current(&target.outputs, project_dir)?,
            }))
        }
    }

    pub fn eq_current_state(&self, target: &Target) -> Result<bool> {
        let project_dir = &target.project.dir;
        Ok(self.inputs.eq_current_state(&target.inputs, &project_dir)?
            && self
                .outputs
                .eq_current_state(&target.outputs, &project_dir)?)
    }
}
