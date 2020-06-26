mod resources_state;
pub mod storage;

use crate::domain::{Resources, Target, TargetId};
use anyhow::{Context, Result};
use futures::{future, Future};
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
    if env_state_has_not_changed_since_last_successful_execution(target).await? {
        return Ok(IncrementalRunResult::Skipped);
    }

    storage::delete_saved_env_state(&target).await?;

    let result = function().await;

    if result.is_ok() {
        match TargetEnvState::current(target).await {
            Ok(Some(env_state)) => storage::save_env_state(&target, env_state).await?,
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

async fn env_state_has_not_changed_since_last_successful_execution(
    target: &Target,
) -> Result<bool> {
    let saved_state = storage::read_saved_target_env_state(target)
        .await
        .with_context(|| format!("Failed to read saved env state for {}", target))?;

    if let Some(saved_state) = saved_state {
        Ok(saved_state.eq_current_state(target).await)
    } else {
        Ok(false)
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct TargetEnvState {
    input: ResourcesState,
    output: Option<ResourcesState>,
}

impl TargetEnvState {
    pub async fn current(target: &Target) -> Result<Option<Self>> {
        match target.input() {
            Some(target_input) if !target_input.is_empty() => {
                let input = ResourcesState::current(target_input).await?;
                let output = match target.output() {
                    Some(target_output) => Some(ResourcesState::current(target_output).await?),
                    None => None,
                };

                Ok(Some(TargetEnvState { input, output }))
            }
            _ => Ok(None),
        }
    }

    pub async fn eq_current_state(&self, target: &Target) -> bool {
        // TODO Resolve at the first negative result
        async fn eq(
            env_state: &Option<&ResourcesState>,
            resources: &Option<&Resources>,
            target_id: &TargetId,
        ) -> bool {
            if let Some(resources) = resources {
                if let Some(env_state) = env_state {
                    env_state
                        .eq_current_state(resources)
                        .await
                        .unwrap_or_else(|e| {
                            log::error!("Failed to run {} incrementally: {}", target_id, e);
                            false
                        })
                } else {
                    false
                }
            } else {
                true
            }
        };

        let (input, output) = future::join(
            eq(&Some(&self.input), &target.input(), target.id()),
            eq(&self.output.as_ref(), &target.output(), target.id()),
        )
        .await;

        input && output
    }
}
