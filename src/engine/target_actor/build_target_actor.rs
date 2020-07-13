use super::{ActorId, ActorInputMessage, TargetActorHelper};
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
            if self.helper.to_execute
                && ongoing_build_cancellation_sender.is_none()
                && !self.helper.build_requesters.is_empty()
                && self.helper.unavailable_dependency_builds.is_empty()
                && self.helper.unavailable_dependency_services.is_empty()
            {
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
                    self.helper.notify_build_invalidated().await
                }
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        ActorInputMessage::BuildOk(target_id) => {
                            self.helper.unavailable_dependency_builds.remove(&target_id);
                        },
                        ActorInputMessage::ServiceOk { target_id, .. } => {
                            self.helper.unavailable_dependency_services.remove(&target_id);
                        },
                        ActorInputMessage::BuildInvalidated(target_id) => {
                            if self.helper.dependencies.contains(&target_id) {
                                self.helper.unavailable_dependency_builds.insert(target_id);
                                self.helper.notify_build_invalidated().await
                            }
                        }
                        ActorInputMessage::ServiceInvalidated(target_id) => {
                            if self.helper.dependencies.contains(&target_id) {
                                self.helper.unavailable_dependency_services.insert(target_id);

                                // TODO If ongoing build, cancel?
                            }
                        }
                        ActorInputMessage::BuildRequested { requester } => {
                            let inserted = self.helper.build_requesters.insert(requester);

                            if inserted && self.helper.build_requesters.len() == 1 {
                                // TODO Eventually, only request deps build (request services when build not skipped)
                                self.helper.send_to_dependencies(ActorInputMessage::BuildRequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                                self.helper.send_to_dependencies(ActorInputMessage::ServiceRequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                            }
                        }
                        ActorInputMessage::ServiceRequested { requester } => {
                            let msg = ActorInputMessage::ServiceOk {
                                target_id: self.helper.target_id.clone(),
                                has_service: false,
                            };
                            self.helper.send_to_actor(requester, msg).await
                        }
                        ActorInputMessage::BuildUnrequested { requester } => {
                            let removed = self.helper.build_requesters.remove(&requester);

                            if removed && self.helper.build_requesters.is_empty() {
                                // TODO Interrupt ongoing build?

                                self.helper.send_to_dependencies(ActorInputMessage::BuildUnrequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                                self.helper.send_to_dependencies(ActorInputMessage::ServiceUnrequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                            }
                        }
                        ActorInputMessage::ServiceUnrequested { requester } => {}
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

                            // TODO Remove duplication (1)
                            self.helper.executed = !self.helper.to_execute;

                            if self.helper.executed {
                                let msg = ActorInputMessage::BuildOk(self.helper.target_id.clone());
                                self.helper.send_to_build_requesters(msg).await;
                            }
                        }
                        IncrementalRunResult::Run(Ok(BuildCompletionReport::Completed)) => {
                            // TODO Why spreading logs between here and builder?

                            // TODO Remove duplication (1)
                            self.helper.executed = !self.helper.to_execute;

                            if self.helper.executed {
                                let msg = ActorInputMessage::BuildOk(self.helper.target_id.clone());
                                self.helper.send_to_build_requesters(msg).await;
                            }

                            // TODO Unrequest dependency services
                        }
                        IncrementalRunResult::Run(Ok(BuildCompletionReport::Aborted)) => break,
                    }
                },
            }
        }
    }
}
