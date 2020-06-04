mod resources_state;
pub mod storage;

use crate::domain::Target;
use anyhow::{Context, Result};
use rayon::prelude::*;
use resources_state::ResourcesState;
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
        match TargetEnvState::current(target) {
            Ok(Some(env_state)) => storage::save_env_state(&target, &env_state)?,
            Ok(None) => {}
            Err(e) => log::error!(
                "{} - Failed to compute state of inputs and outputs: {}",
                target,
                e
            ),
        }
    }

    Ok(IncrementalRunResult::Run(result))
}

fn env_state_has_not_changed_since_last_successful_execution(target: &Target) -> Result<bool> {
    let saved_state = storage::read_saved_target_env_state(target)
        .with_context(|| format!("Failed to read saved env state for {}", target))?;

    Ok(saved_state
        .map(|saved_state| saved_state.eq_current_state(target))
        .unwrap_or(false))
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct TargetEnvState {
    input: ResourcesState,
    output: ResourcesState,
}

impl TargetEnvState {
    pub fn current(target: &Target) -> Result<Option<Self>> {
        if target.input.is_empty() {
            Ok(None)
        } else {
            let project_dir = &target.project.dir;
            Ok(Some(TargetEnvState {
                input: ResourcesState::current(&target.input, project_dir)?,
                output: ResourcesState::current(&target.output, project_dir)?,
            }))
        }
    }

    pub fn eq_current_state(&self, target: &Target) -> bool {
        let project_dir = &target.project.dir;

        [(&self.input, &target.input), (&self.output, &target.output)]
            .par_iter()
            .all(|(env_state, env_probes)| {
                match env_state.eq_current_state(env_probes, project_dir) {
                    Ok(res) => res,
                    Err(e) => {
                        log::error!("Failed to run {} incrementally: {}", target, e);
                        false
                    }
                }
            })
    }
}
