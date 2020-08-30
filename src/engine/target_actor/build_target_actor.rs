use super::{ActorInputMessage, ExecutionKind, TargetActorHelper};
use crate::domain::BuildTarget;
use crate::engine::{builder, incremental};
use async_std::{prelude::*, sync};
use builder::BuildCancellationMessage;
use futures::future::Fuse;
use futures::{pin_mut, FutureExt};
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
        let mut termination_event_received = false;
        let ongoing_build_fuse = Fuse::terminated();
        pin_mut!(ongoing_build_fuse);
        let mut ongoing_build_cancellation_sender = None;

        loop {
            if self.helper.should_execute(ExecutionKind::Build)
                && ongoing_build_cancellation_sender.is_none()
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
                    termination_event_received = true;
                    if let Some(ongoing_build_cancellation_sender) = &mut ongoing_build_cancellation_sender {
                        if ongoing_build_cancellation_sender.try_send(BuildCancellationMessage).is_err() {
                            log::trace!("{} - Build already cancelled. Skipping.", self.target);
                        }
                    } else {
                        break;
                    }
                },
                _ = self.helper.target_invalidated_events.next().fuse() => {
                    self.helper.notify_invalidated(ExecutionKind::Build).await
                }
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        ActorInputMessage::Ok { kind, target_id, .. } => {
                            self.helper.unavailable_dependencies.get_mut(&kind).unwrap().remove(&target_id);
                        },
                        ActorInputMessage::Invalidated { kind, target_id } => {
                            self.helper.unavailable_dependencies.get_mut(&kind).unwrap().insert(target_id);

                            if kind == ExecutionKind::Build {
                              self.helper.notify_invalidated(ExecutionKind::Build).await
                            } // TODO Else, if ongoing build, cancel?
                        }
                        ActorInputMessage::Requested { kind: ExecutionKind::Build, requester } => {
                            let inserted = self.helper.requesters.get_mut(&ExecutionKind::Build).unwrap().insert(requester);

                            if inserted && self.helper.requesters[&ExecutionKind::Build].len() == 1 {
                                // TODO Eventually, only request deps build (request services when build not skipped)
                                self.helper.request_dependencies(ExecutionKind::Build).await;
                                self.helper.request_dependencies(ExecutionKind::Service).await;
                            }
                        }
                        ActorInputMessage::Requested { kind: ExecutionKind::Service, requester } => {
                            let msg = ActorInputMessage::Ok {
                                kind: ExecutionKind::Service,
                                target_id: self.helper.target_id.clone(),
                                actual: false,
                            };
                            self.helper.send_to_actor(requester, msg).await
                        }
                        ActorInputMessage::Unrequested { kind, requester } => {
                            let was_last_requester = self.helper.handle_unrequested(kind, requester);

                            if was_last_requester && kind == ExecutionKind::Build {
                                self.helper.unrequest_dependencies(ExecutionKind::Build).await;
                                self.helper.unrequest_dependencies(ExecutionKind::Service).await;
                            }
                        }
                    }
                }
                build_result = ongoing_build_fuse => {
                    ongoing_build_cancellation_sender = None;

                    match build_result {
                        Err(e) => self.helper.notify_execution_failed(e).await,
                        Ok(IncrementalRunResult::Skipped) => {
                            log::info!("{} - Build skipped (Not Modified)", self.target);
                            self.helper.notify_success(ExecutionKind::Build).await;
                        }
                        Ok(IncrementalRunResult::Completed) => {
                            // TODO Why spreading logs between here and builder?
                            self.helper.notify_success(ExecutionKind::Build).await;

                            // TODO Eventually, unrequest dependency services
                        }
                        Ok(IncrementalRunResult::Cancelled) => {
                            // As termination_event_received == true, we will exit the loop
                        },
                    }

                    if termination_event_received {
                        break
                    }
                },
            }
        }
    }
}
