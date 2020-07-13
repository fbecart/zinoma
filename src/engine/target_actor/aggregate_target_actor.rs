use super::{ActorId, ActorInputMessage, TargetActorHelper};
use crate::domain::AggregateTarget;
use async_std::prelude::*;
use futures::FutureExt;
pub struct AggregateTargetActor {
    _target: AggregateTarget,
    helper: TargetActorHelper,
}

impl AggregateTargetActor {
    pub fn new(target: AggregateTarget, helper: TargetActorHelper) -> Self {
        Self {
            _target: target,
            helper,
        }
    }

    pub async fn run(mut self) {
        loop {
            futures::select! {
                _ = self.helper.termination_events.next().fuse() => break,
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        ActorInputMessage::BuildOk(target_id) => {
                            let removed = self.helper.unavailable_dependency_builds.remove(&target_id);

                            if removed && self.helper.unavailable_dependency_builds.is_empty() {
                                let msg = ActorInputMessage::BuildOk(self.helper.target_id.clone());
                                self.helper.send_to_build_requesters(msg).await
                            }
                        },
                        ActorInputMessage::ServiceOk(target_id) => {
                            let removed = self.helper.unavailable_dependency_services.remove(&target_id);

                            if removed && self.helper.unavailable_dependency_services.is_empty() {
                                let msg = ActorInputMessage::ServiceOk(self.helper.target_id.clone());
                                self.helper.send_to_service_requesters(msg).await
                            }
                        },
                        ActorInputMessage::BuildInvalidated(target_id) => {
                            // TODO Remove if statement (+ similar cases)
                            if self.helper.dependencies.contains(&target_id) {
                                let inserted = self.helper.unavailable_dependency_builds.insert(target_id.clone());

                                if inserted && self.helper.unavailable_dependency_builds.len() == 1 {
                                    let msg = ActorInputMessage::BuildInvalidated(self.helper.target_id.clone());
                                    self.helper.send_to_build_requesters(msg).await
                                }
                            }
                        }
                        ActorInputMessage::ServiceInvalidated(target_id) => {
                            if self.helper.dependencies.contains(&target_id) {
                                let inserted = self.helper.unavailable_dependency_services.insert(target_id);

                                if inserted && self.helper.unavailable_dependency_services.len() == 1 {
                                    let msg = ActorInputMessage::ServiceInvalidated(self.helper.target_id.clone());
                                    self.helper.send_to_service_requesters(msg).await
                                }
                            }
                        }
                        ActorInputMessage::BuildRequested { requester } => {
                            let inserted = self.helper.build_requesters.insert(requester.clone());

                            if inserted {
                                let is_first_insertion = self.helper.build_requesters.len() == 1;
                                if is_first_insertion {
                                    let msg = ActorInputMessage::BuildRequested {
                                        requester: ActorId::Target(self.helper.target_id.clone()),
                                    };
                                    self.helper.send_to_dependencies(msg).await
                                }

                                if self.helper.unavailable_dependency_builds.is_empty() {
                                    let msg = ActorInputMessage::BuildOk(self.helper.target_id.clone());
                                    self.helper.send_to_actor(requester, msg).await
                                }
                            }
                        }
                        ActorInputMessage::ServiceRequested { requester } => {
                            let inserted = self.helper.service_requesters.insert(requester.clone());

                            if inserted {
                                let is_first_insertion = self.helper.service_requesters.len() == 1;
                                if is_first_insertion {
                                    let msg = ActorInputMessage::ServiceRequested {
                                        requester: ActorId::Target(self.helper.target_id.clone()),
                                    };
                                    self.helper.send_to_dependencies(msg).await;
                                }

                                if self.helper.unavailable_dependency_services.is_empty() {
                                    let msg = ActorInputMessage::ServiceOk(self.helper.target_id.clone());
                                    self.helper.send_to_actor(requester, msg).await
                                }
                            }
                        }
                        ActorInputMessage::BuildUnrequested { requester } => {
                            let removed = self.helper.build_requesters.remove(&requester);

                            if removed && self.helper.build_requesters.is_empty() {
                                let msg = ActorInputMessage::BuildUnrequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                };
                                self.helper.send_to_dependencies(msg).await
                            }
                        }
                        ActorInputMessage::ServiceUnrequested { requester } => {
                            let removed = self.helper.service_requesters.remove(&requester);

                            if removed && self.helper.service_requesters.is_empty() {
                                let msg = ActorInputMessage::ServiceUnrequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                };
                                self.helper.send_to_dependencies(msg).await
                            }
                        }
                    }
                }
            }
        }
    }
}
