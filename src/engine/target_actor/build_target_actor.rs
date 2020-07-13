use super::{ActorId, ActorInputMessage, ExecutionKind, TargetActorHelper};
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
                && !self.helper.requesters[&ExecutionKind::Build].is_empty()
                && self.helper.unavailable_dependencies[&ExecutionKind::Build].is_empty()
                && self.helper.unavailable_dependencies[&ExecutionKind::Service].is_empty()
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
                    self.helper.notify_invalidated(ExecutionKind::Build).await
                }
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        ActorInputMessage::Ok { kind: ExecutionKind::Build, target_id, .. } => {
                            self.helper.unavailable_dependencies.get_mut(&ExecutionKind::Build).unwrap().remove(&target_id);
                        },
                        ActorInputMessage::Ok { kind: ExecutionKind::Service, target_id, .. } => {
                            self.helper.unavailable_dependencies.get_mut(&ExecutionKind::Service).unwrap().remove(&target_id);
                        },
                        ActorInputMessage::Invalidated { kind: ExecutionKind::Build, target_id } => {
                            self.helper.unavailable_dependencies.get_mut(&ExecutionKind::Build).unwrap().insert(target_id);
                            self.helper.notify_invalidated(ExecutionKind::Build).await
                        }
                        ActorInputMessage::Invalidated { kind: ExecutionKind::Service, target_id } => {
                            self.helper.unavailable_dependencies.get_mut(&ExecutionKind::Service).unwrap().insert(target_id);

                            // TODO If ongoing build, cancel?
                        }
                        ActorInputMessage::Requested { kind: ExecutionKind::Build, requester } => {
                            let inserted = self.helper.requesters.get_mut(&ExecutionKind::Build).unwrap().insert(requester);

                            if inserted && self.helper.requesters[&ExecutionKind::Build].len() == 1 {
                                // TODO Eventually, only request deps build (request services when build not skipped)
                                self.helper.send_to_dependencies(ActorInputMessage::Requested {
                                    kind: ExecutionKind::Build,
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                                self.helper.send_to_dependencies(ActorInputMessage::Requested {
                                    kind: ExecutionKind::Service,
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
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
                        ActorInputMessage::Unrequested { kind: ExecutionKind::Build, requester } => {
                            let removed = self.helper.requesters.get_mut(&ExecutionKind::Build).unwrap().remove(&requester);

                            if removed && self.helper.requesters[&ExecutionKind::Build].is_empty() {
                                self.helper.send_to_dependencies(ActorInputMessage::Unrequested {
                                    kind: ExecutionKind::Build,
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                                self.helper.send_to_dependencies(ActorInputMessage::Unrequested {
                                    kind: ExecutionKind::Service,
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                            }
                        }
                        ActorInputMessage::Unrequested { kind: ExecutionKind::Service, requester } => {}
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
                            Self::on_success(&mut self.helper).await;
                        }
                        IncrementalRunResult::Run(Ok(BuildCompletionReport::Completed)) => {
                            // TODO Why spreading logs between here and builder?
                            Self::on_success(&mut self.helper).await;

                            // TODO Eventually, unrequest dependency services
                        }
                        IncrementalRunResult::Run(Ok(BuildCompletionReport::Aborted)) => break,
                    }
                },
            }
        }
    }

    async fn on_success(helper: &mut TargetActorHelper) {
        helper.executed = !helper.to_execute;

        if helper.executed {
            let target_id = helper.target_id.clone();
            let msg = ActorInputMessage::Ok {
                kind: ExecutionKind::Build,
                target_id,
                actual: true,
            };
            helper.send_to_requesters(ExecutionKind::Build, msg).await
        }
    }
}
