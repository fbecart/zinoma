mod resources_state;
pub mod storage;

use crate::async_utils::both;
use crate::domain::{Resources, TargetMetadata};
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
    target: &TargetMetadata,
    target_input: &Resources,
    target_output: Option<&Resources>,
    function: impl Fn() -> F,
) -> Result<IncrementalRunResult<F::Output>>
where
    F: Future<Output = Result<T>>,
{
    if env_state_has_not_changed_since_last_successful_execution(
        target,
        target_input,
        target_output,
    )
    .await?
    {
        return Ok(IncrementalRunResult::Skipped);
    }

    storage::delete_saved_env_state(target).await?;

    let result = function().await;

    if result.is_ok() {
        match TargetEnvState::current(target_input, target_output).await {
            Ok(Some(env_state)) => storage::save_env_state(target, env_state).await?,
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
    target: &TargetMetadata,
    target_input: &Resources,
    target_output: Option<&Resources>,
) -> Result<bool> {
    let saved_state = storage::read_saved_target_env_state(target)
        .await
        .with_context(|| format!("Failed to read saved env state for {}", target))?;

    if let Some(saved_state) = saved_state {
        Ok(saved_state
            .eq_current_state(target_input, target_output)
            .await)
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
    pub async fn current(
        target_input: &Resources,
        target_output: Option<&Resources>,
    ) -> Result<Option<Self>> {
        if target_input.is_empty() {
            Ok(None)
        } else {
            let input = ResourcesState::current(target_input).await?;
            let output = match target_output {
                Some(target_output) => Some(ResourcesState::current(target_output).await?),
                None => None,
            };

            Ok(Some(TargetEnvState { input, output }))
        }
    }

    pub async fn eq_current_state(
        &self,
        target_input: &Resources,
        target_output: Option<&Resources>,
    ) -> bool {
        async fn eq(env_state: Option<&ResourcesState>, resources: Option<&Resources>) -> bool {
            if let Some(resources) = resources {
                if let Some(env_state) = env_state {
                    env_state.eq_current_state(resources).await
                } else {
                    false
                }
            } else {
                true
            }
        };

        both(
            eq(Some(&self.input), Some(target_input)),
            eq(self.output.as_ref(), target_output),
        )
        .await
    }
}
