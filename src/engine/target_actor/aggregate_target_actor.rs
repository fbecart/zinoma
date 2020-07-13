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
                        ActorInputMessage::Ok { kind, target_id, actual } => {
                            let removed = self.helper.unavailable_dependencies.get_mut(&kind).unwrap().remove(&target_id);

                            if actual {
                                dependencies.get_mut(&kind).unwrap().insert(target_id);
                            }

                            if removed && self.helper.unavailable_dependencies[&kind].is_empty() {
                                let msg = ActorInputMessage::Ok {
                                    kind,
                                    target_id: self.helper.target_id.clone(),
                                    actual: !dependencies[&kind].is_empty(),
                                };
                                self.helper.send_to_requesters(kind, msg).await
                            }
                        },
                        ActorInputMessage::Invalidated { kind, target_id } => {
                            let inserted = self.helper.unavailable_dependencies.get_mut(&kind).unwrap().insert(target_id.clone());

                            if inserted && self.helper.unavailable_dependencies[&kind].len() == 1 {
                                let msg = ActorInputMessage::Invalidated { kind, target_id: self.helper.target_id.clone() };
                                self.helper.send_to_requesters(kind, msg).await
                            }
                        }
                        ActorInputMessage::Requested { kind, requester } => {
                            let inserted = self.helper.requesters.get_mut(&kind).unwrap().insert(requester.clone());

                            if inserted {
                                let is_first_insertion = self.helper.requesters[&kind].len() == 1;
                                if is_first_insertion {
                                    let msg = ActorInputMessage::Requested {
                                        kind,
                                        requester: ActorId::Target(self.helper.target_id.clone()),
                                    };
                                    self.helper.send_to_dependencies(msg).await
                                }

                                if self.helper.unavailable_dependencies[&kind].is_empty() {
                                    let msg = ActorInputMessage::Ok {
                                        kind,
                                        target_id: self.helper.target_id.clone(),
                                        actual: !dependencies[&kind].is_empty(),
                                    };
                                    self.helper.send_to_actor(requester, msg).await
                                }
                            }
                        }
                        ActorInputMessage::Unrequested { kind, requester } => {
                            let removed = self.helper.requesters.get_mut(&kind).unwrap().remove(&requester);

                            if removed && self.helper.requesters[&kind].is_empty() {
                                let msg = ActorInputMessage::Unrequested {
                                    kind,
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
