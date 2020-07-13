use super::{ActorId, ActorInputMessage, ExecutionKind, TargetActorHelper};
use crate::domain::AggregateTarget;
use async_std::prelude::*;
use futures::FutureExt;
use std::collections::HashSet;
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
        let mut build_dependencies = HashSet::new();
        let mut service_dependencies = HashSet::new();

        loop {
            futures::select! {
                _ = self.helper.termination_events.next().fuse() => break,
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        ActorInputMessage::Ok { kind: ExecutionKind::Build, target_id, actual } => {
                            let removed = self.helper.unavailable_dependency_builds.remove(&target_id);

                            if actual {
                                build_dependencies.insert(target_id);
                            }

                            if removed && self.helper.unavailable_dependency_builds.is_empty() {
                                let msg = ActorInputMessage::Ok {
                                    kind: ExecutionKind::Build,
                                    target_id: self.helper.target_id.clone(),
                                    actual: !build_dependencies.is_empty(),
                                };
                                self.helper.send_to_build_requesters(msg).await
                            }
                        },
                        ActorInputMessage::Ok { kind: ExecutionKind::Service, target_id, actual } => {
                            let removed = self.helper.unavailable_dependency_services.remove(&target_id);

                            if actual {
                                service_dependencies.insert(target_id);
                            }

                            if removed && self.helper.unavailable_dependency_services.is_empty() {
                                let msg = ActorInputMessage::Ok {
                                    kind: ExecutionKind::Service,
                                    target_id: self.helper.target_id.clone(),
                                    actual: !service_dependencies.is_empty(),
                                };
                                self.helper.send_to_service_requesters(msg).await
                            }
                        },
                        ActorInputMessage::Invalidated { kind: ExecutionKind::Build, target_id } => {
                            let inserted = self.helper.unavailable_dependency_builds.insert(target_id.clone());

                            if inserted && self.helper.unavailable_dependency_builds.len() == 1 {
                                let msg = ActorInputMessage::Invalidated { kind: ExecutionKind::Build, target_id: self.helper.target_id.clone() };
                                self.helper.send_to_build_requesters(msg).await
                            }
                        }
                        ActorInputMessage::Invalidated { kind: ExecutionKind::Service, target_id } => {
                            let inserted = self.helper.unavailable_dependency_services.insert(target_id);

                            if inserted && self.helper.unavailable_dependency_services.len() == 1 {
                                let msg = ActorInputMessage::Invalidated { kind: ExecutionKind::Service, target_id: self.helper.target_id.clone() };
                                self.helper.send_to_service_requesters(msg).await
                            }
                        }
                        ActorInputMessage::Requested { kind: ExecutionKind::Build, requester } => {
                            let inserted = self.helper.build_requesters.insert(requester.clone());

                            if inserted {
                                let is_first_insertion = self.helper.build_requesters.len() == 1;
                                if is_first_insertion {
                                    let msg = ActorInputMessage::Requested {
                                        kind: ExecutionKind::Build,
                                        requester: ActorId::Target(self.helper.target_id.clone()),
                                    };
                                    self.helper.send_to_dependencies(msg).await
                                }

                                if self.helper.unavailable_dependency_builds.is_empty() {
                                    let msg = ActorInputMessage::Ok {
                                        kind: ExecutionKind::Build,
                                        target_id: self.helper.target_id.clone(),
                                        actual: !build_dependencies.is_empty(),
                                    };
                                    self.helper.send_to_actor(requester, msg).await
                                }
                            }
                        }
                        ActorInputMessage::Requested { kind: ExecutionKind::Service, requester } => {
                            let inserted = self.helper.service_requesters.insert(requester.clone());

                            if inserted {
                                let is_first_insertion = self.helper.service_requesters.len() == 1;
                                if is_first_insertion {
                                    let msg = ActorInputMessage::Requested {
                                        kind: ExecutionKind::Service,
                                        requester: ActorId::Target(self.helper.target_id.clone()),
                                    };
                                    self.helper.send_to_dependencies(msg).await;
                                }

                                if self.helper.unavailable_dependency_services.is_empty() {
                                    let msg = ActorInputMessage::Ok {
                                        kind: ExecutionKind::Service,
                                        target_id: self.helper.target_id.clone(),
                                        actual: !service_dependencies.is_empty(),
                                    };
                                    self.helper.send_to_actor(requester, msg).await
                                }
                            }
                        }
                        ActorInputMessage::Unrequested { kind: ExecutionKind::Build, requester } => {
                            let removed = self.helper.build_requesters.remove(&requester);

                            if removed && self.helper.build_requesters.is_empty() {
                                let msg = ActorInputMessage::Unrequested {
                                    kind: ExecutionKind::Build,
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                };
                                self.helper.send_to_dependencies(msg).await
                            }
                        }
                        ActorInputMessage::Unrequested { kind: ExecutionKind::Service, requester } => {
                            let removed = self.helper.service_requesters.remove(&requester);

                            if removed && self.helper.service_requesters.is_empty() {
                                let msg = ActorInputMessage::Unrequested {
                                    kind: ExecutionKind::Service,
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
