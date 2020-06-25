mod resources_state;
pub mod storage;

use crate::domain::Target;
use anyhow::{Context, Result};
use futures::Future;
use resources_state::ResourcesState;
use serde::{Deserialize, Serialize};

#[derive(PartialEq)]
pub enum IncrementalRunResult<T> {
    Skipped,
    Run(T),
}

pub async fn run<T, F>(
    target: &Target,
    function: impl Fn() -> F,
) -> Result<IncrementalRunResult<F::Output>>
where
    F: Future<Output = Result<T>>,
{
    if env_state_has_not_changed_since_last_successful_execution(target)? {
        return Ok(IncrementalRunResult::Skipped);
    }

    storage::delete_saved_env_state(&target)?;

    let result = function().await;

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
    output: Option<ResourcesState>,
}

impl TargetEnvState {
    pub fn current(target: &Target) -> Result<Option<Self>> {
        match target.input() {
            Some(target_input) if !target_input.is_empty() => {
                let input = ResourcesState::current(target_input)?;
                let output = target
                    .output()
                    .map(|target_output| ResourcesState::current(target_output))
                    .transpose()?;

                Ok(Some(TargetEnvState { input, output }))
            }
            _ => Ok(None),
        }
    }

    pub fn eq_current_state(&self, target: &Target) -> bool {
        // TODO Here was rayon
        [
            (Some(&self.input), &target.input()),
            (self.output.as_ref(), &target.output()),
        ]
        .iter()
        .all(|(env_state, resources)| {
            resources.map_or(true, |resources| {
                env_state.as_ref().map_or(false, |env_state| {
                    env_state.eq_current_state(resources).unwrap_or_else(|e| {
                        log::error!("Failed to run {} incrementally: {}", target, e);
                        false
                    })
                })
            })
        })
    }
}
