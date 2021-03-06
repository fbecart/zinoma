mod resources_state;
pub mod storage;

use super::builder::BuildTerminationReport;
use crate::async_utils::both;
use crate::domain::{Resources, TargetMetadata};
use anyhow::Result;
use futures::Future;
use resources_state::ResourcesState;
use serde::{Deserialize, Serialize};

#[derive(PartialEq)]
pub enum IncrementalRunResult {
    Skipped,
    Completed,
    Cancelled,
}

pub async fn run<F>(
    target: &TargetMetadata,
    target_input: &Resources,
    target_output: Option<&Resources>,
    future: F,
) -> Result<IncrementalRunResult>
where
    F: Future<Output = Result<BuildTerminationReport>>,
{
    if env_state_has_not_changed_since_last_successful_execution(
        target,
        target_input,
        target_output,
    )
    .await
    {
        return Ok(IncrementalRunResult::Skipped);
    }

    storage::delete_saved_env_state(target).await?;

    let build_report = future.await?;

    match build_report {
        BuildTerminationReport::Cancelled => Ok(IncrementalRunResult::Cancelled),
        BuildTerminationReport::Completed => {
            match TargetEnvState::current(target_input, target_output).await {
                Ok(Some(env_state)) => {
                    if let Err(e) = storage::save_env_state(target, env_state).await {
                        log::warn!(
                            "{} - Failed to save state of inputs and outputs: {}",
                            target,
                            e
                        )
                    }
                }
                Ok(None) => {}
                Err(e) => log::warn!(
                    "{} - Failed to compute state of inputs and outputs: {}",
                    target,
                    e
                ),
            }

            Ok(IncrementalRunResult::Completed)
        }
    }
}

async fn env_state_has_not_changed_since_last_successful_execution(
    target: &TargetMetadata,
    target_input: &Resources,
    target_output: Option<&Resources>,
) -> bool {
    if let Some(saved_state) = storage::read_saved_target_env_state(target).await {
        saved_state
            .eq_current_state(target_input, target_output)
            .await
    } else {
        false
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
        }

        both(
            eq(Some(&self.input), Some(target_input)),
            eq(self.output.as_ref(), target_output),
        )
        .await
    }
}
