use super::{ActorId, ActorInputMessage, ExecutionKind, TargetActorHelper};
use crate::domain::AggregateTarget;
use async_std::prelude::*;
use futures::FutureExt;
use std::collections::{HashMap, HashSet};
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
        let mut dependencies = HashMap::<ExecutionKind, _>::new();
        dependencies.insert(ExecutionKind::Build, HashSet::new());
        dependencies.insert(ExecutionKind::Service, HashSet::new());

        loop {
            futures::select! {
                _ = self.helper.termination_events.next().fuse() => break,
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        ActorInputMessage::Ok { kind: ExecutionKind::Build, target_id, actual } => {
                            let removed = self.helper.unavailable_dependencies.get_mut(&ExecutionKind::Build).unwrap().remove(&target_id);

                            if actual {
                                dependencies.get_mut(&ExecutionKind::Build).unwrap().insert(target_id);
                            }

                            if removed && self.helper.unavailable_dependencies[&ExecutionKind::Build].is_empty() {
                                let msg = ActorInputMessage::Ok {
                                    kind: ExecutionKind::Build,
                                    target_id: self.helper.target_id.clone(),
                                    actual: !dependencies[&ExecutionKind::Build].is_empty(),
                                };
                                self.helper.send_to_requesters(ExecutionKind::Build, msg).await
                            }
                        },
                        ActorInputMessage::Ok { kind: ExecutionKind::Service, target_id, actual } => {
                            let removed = self.helper.unavailable_dependencies.get_mut(&ExecutionKind::Service).unwrap().remove(&target_id);

                            if actual {
                                dependencies.get_mut(&ExecutionKind::Service).unwrap().insert(target_id);
                            }

                            if removed && self.helper.unavailable_dependencies[&ExecutionKind::Service].is_empty() {
                                let msg = ActorInputMessage::Ok {
                                    kind: ExecutionKind::Service,
                                    target_id: self.helper.target_id.clone(),
                                    actual: !dependencies[&ExecutionKind::Service].is_empty(),
                                };
                                self.helper.send_to_requesters(ExecutionKind::Service, msg).await
                            }
                        },
                        ActorInputMessage::Invalidated { kind: ExecutionKind::Build, target_id } => {
                            let inserted = self.helper.unavailable_dependencies.get_mut(&ExecutionKind::Build).unwrap().insert(target_id.clone());

                            if inserted && self.helper.unavailable_dependencies[&ExecutionKind::Build].len() == 1 {
                                let msg = ActorInputMessage::Invalidated { kind: ExecutionKind::Build, target_id: self.helper.target_id.clone() };
                                self.helper.send_to_requesters(ExecutionKind::Build, msg).await
                            }
                        }
                        ActorInputMessage::Invalidated { kind: ExecutionKind::Service, target_id } => {
                            let inserted = self.helper.unavailable_dependencies.get_mut(&ExecutionKind::Service).unwrap().insert(target_id);

                            if inserted && self.helper.unavailable_dependencies[&ExecutionKind::Service].len() == 1 {
                                let msg = ActorInputMessage::Invalidated { kind: ExecutionKind::Service, target_id: self.helper.target_id.clone() };
                                self.helper.send_to_requesters(ExecutionKind::Service, msg).await
                            }
                        }
                        ActorInputMessage::Requested { kind: ExecutionKind::Build, requester } => {
                            let inserted = self.helper.requesters.get_mut(&ExecutionKind::Build).unwrap().insert(requester.clone());

                            if inserted {
                                let is_first_insertion = self.helper.requesters[&ExecutionKind::Build].len() == 1;
                                if is_first_insertion {
                                    let msg = ActorInputMessage::Requested {
                                        kind: ExecutionKind::Build,
                                        requester: ActorId::Target(self.helper.target_id.clone()),
                                    };
                                    self.helper.send_to_dependencies(msg).await
                                }

                                if self.helper.unavailable_dependencies[&ExecutionKind::Build].is_empty() {
                                    let msg = ActorInputMessage::Ok {
                                        kind: ExecutionKind::Build,
                                        target_id: self.helper.target_id.clone(),
                                        actual: !dependencies[&ExecutionKind::Build].is_empty(),
                                    };
                                    self.helper.send_to_actor(requester, msg).await
                                }
                            }
                        }
                        ActorInputMessage::Requested { kind: ExecutionKind::Service, requester } => {
                            let inserted = self.helper.requesters.get_mut(&ExecutionKind::Service).unwrap().insert(requester.clone());

                            if inserted {
                                let is_first_insertion = self.helper.requesters[&ExecutionKind::Service].len() == 1;
                                if is_first_insertion {
                                    let msg = ActorInputMessage::Requested {
                                        kind: ExecutionKind::Service,
                                        requester: ActorId::Target(self.helper.target_id.clone()),
                                    };
                                    self.helper.send_to_dependencies(msg).await;
                                }

                                if self.helper.unavailable_dependencies[&ExecutionKind::Service].is_empty() {
                                    let msg = ActorInputMessage::Ok {
                                        kind: ExecutionKind::Service,
                                        target_id: self.helper.target_id.clone(),
                                        actual: !dependencies[&ExecutionKind::Service].is_empty(),
                                    };
                                    self.helper.send_to_actor(requester, msg).await
                                }
                            }
                        }
                        ActorInputMessage::Unrequested { kind: ExecutionKind::Build, requester } => {
                            let removed = self.helper.requesters.get_mut(&ExecutionKind::Build).unwrap().remove(&requester);

                            if removed && self.helper.requesters[&ExecutionKind::Build].is_empty() {
                                let msg = ActorInputMessage::Unrequested {
                                    kind: ExecutionKind::Build,
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                };
                                self.helper.send_to_dependencies(msg).await
                            }
                        }
                        ActorInputMessage::Unrequested { kind: ExecutionKind::Service, requester } => {
                            let removed = self.helper.requesters.get_mut(&ExecutionKind::Service).unwrap().remove(&requester);

                            if removed && self.helper.requesters[&ExecutionKind::Service].is_empty() {
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
