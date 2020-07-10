use super::{TargetActorHelper, TargetActorInputMessage};
use crate::domain::BuildTarget;
use crate::engine::{builder, incremental};
use anyhow::Context;
use async_std::prelude::*;
use builder::BuildCompletionReport;
use futures::{future::Fuse, pin_mut, FutureExt};
use incremental::IncrementalRunResult;
pub struct BuildTargetActor {
    target: BuildTarget,
    helper: TargetActorHelper,
}

impl BuildTargetActor {
    pub fn new(target: BuildTarget, target_actor_helper: TargetActorHelper) -> Self {
        Self {
            target,
            helper: target_actor_helper,
        }
    }

    pub async fn run(mut self) {
        let ongoing_build = Fuse::terminated();
        pin_mut!(ongoing_build);

        loop {
            if self.helper.is_ready_to_execute() {
                self.helper.set_execution_started();

                ongoing_build.set(
                    incremental::run(
                        &self.target.metadata,
                        &self.target.input,
                        Some(&self.target.output),
                        builder::build_target(&self.target, self.helper.termination_events.clone()),
                    )
                    .fuse(),
                );
            }

            futures::select! {
                _ = self.helper.termination_events.next().fuse() => break,
                _ = self.helper.target_invalidated_events.next().fuse() => {
                    self.helper.notify_target_invalidated().await
                }
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        TargetActorInputMessage::TargetAvailable(target_id) => {
                            self.helper.unavailable_dependencies.remove(&target_id);
                        },
                        TargetActorInputMessage::TargetInvalidated(target_id) => {
                            if self.helper.dependencies.contains(&target_id) {
                                self.helper.unavailable_dependencies.insert(target_id);
                                self.helper.notify_target_invalidated().await
                            }
                        }
                    }
                }
                build_result = ongoing_build => {
                    // TODO Why unwrap?
                    let build_result = build_result
                        .with_context(|| format!("{} - Failed to evaluate target input/output", self.target))
                        .unwrap();
                    match build_result {
                        IncrementalRunResult::Run(Err(e)) => self.helper.notify_execution_failed(e).await,
                        IncrementalRunResult::Skipped => {
                            log::info!("{} - Build skipped (Not Modified)", self.target);
                            self.helper.notify_execution_succeeded().await
                        }
                        IncrementalRunResult::Run(Ok(BuildCompletionReport::Completed)) => {
                            // TODO Why spreading logs between here and builder?
                            self.helper.notify_execution_succeeded().await
                        }
                        IncrementalRunResult::Run(Ok(BuildCompletionReport::Aborted)) => break,
                    }
                },
            }
        }
    }
}
