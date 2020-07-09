use super::{TargetActorInputMessage, TargetActorOutputMessage};
use crate::domain::{AggregateTarget, TargetId};
use crate::TerminationMessage;
use async_std::prelude::*;
use async_std::sync::{Receiver, Sender};
use futures::FutureExt;
use std::collections::HashSet;
use std::iter::FromIterator;
pub struct AggregateTargetActor {
    target_id: TargetId,
    termination_events: Receiver<TerminationMessage>,
    target_actor_input_receiver: Receiver<TargetActorInputMessage>,
    target_actor_output_sender: Sender<TargetActorOutputMessage>,
    to_execute: bool,
    dependencies: HashSet<TargetId>,
    unavailable_dependencies: HashSet<TargetId>,
}

impl AggregateTargetActor {
    pub fn new(
        target: AggregateTarget,
        termination_events: Receiver<TerminationMessage>,
        target_actor_input_receiver: Receiver<TargetActorInputMessage>,
        target_actor_output_sender: Sender<TargetActorOutputMessage>,
    ) -> Self {
        let dependencies = HashSet::from_iter(target.metadata.dependencies.iter().cloned());
        let unavailable_dependencies = dependencies.clone();

        Self {
            target_id: target.metadata.id,
            termination_events,
            target_actor_input_receiver,
            target_actor_output_sender,
            to_execute: true,
            dependencies,
            unavailable_dependencies,
        }
    }

    pub async fn run(mut self) {
        loop {
            if self.to_execute && self.unavailable_dependencies.is_empty() {
                self.to_execute = false;

                let target_id = self.target_id.clone();
                let msg = TargetActorOutputMessage::TargetAvailable(target_id);
                self.target_actor_output_sender.send(msg).await;
            }

            futures::select! {
                _ = self.termination_events.next().fuse() => break,
                message = self.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        TargetActorInputMessage::TargetAvailable(target_id) => {
                            self.unavailable_dependencies.remove(&target_id);
                        },
                        TargetActorInputMessage::TargetInvalidated(target_id) => {
                            if self.dependencies.contains(&target_id) {
                                self.unavailable_dependencies.insert(target_id);

                                if !self.to_execute {
                                    self.to_execute = true;

                                    let msg = TargetActorOutputMessage::TargetInvalidated(self.target_id.clone());
                                    self.target_actor_output_sender.send(msg).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
