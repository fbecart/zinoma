use super::{TargetActorHelper, TargetActorInputMessage};
use crate::domain::BuildTarget;
use crate::engine::{builder, incremental};
use anyhow::Context;
use async_std::{prelude::*, sync};
use builder::{BuildCancellationMessage, BuildCompletionReport};
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
        let ongoing_build_fuse = Fuse::terminated();
        pin_mut!(ongoing_build_fuse);
        let mut ongoing_build_cancellation_sender = None;

        loop {
            if self.helper.is_ready_to_execute() {
                let (build_cancellation_sender, build_cancellation_events) = sync::channel(1);
                ongoing_build_cancellation_sender = Some(build_cancellation_sender);
                let build_future = builder::build_target(&self.target, build_cancellation_events);
                ongoing_build_fuse.set(
                    incremental::run(
                        &self.target.metadata,
                        &self.target.input,
                        Some(&self.target.output),
                        build_future,
                    )
                    .fuse(),
                );

                self.helper.set_execution_started();
            }

            futures::select! {
                _ = self.helper.termination_events.next().fuse() => {
                    if let Some(ongoing_build_cancellation_sender) = &mut ongoing_build_cancellation_sender {
                        if ongoing_build_cancellation_sender.try_send(BuildCancellationMessage).is_err() {
                            log::trace!("{} - Build already cancelled. Skipping.", self.target);
                        }
                    } else {
                        break;
                    }
                },
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
                build_result = ongoing_build_fuse => {
                    ongoing_build_cancellation_sender = None;
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
