use super::{TargetActorInputMessage, TargetActorOutputMessage};
use crate::domain::{TargetId, TargetMetadata};
use crate::engine::watcher::TargetInvalidatedMessage;
use crate::TerminationMessage;
use anyhow::Error;
use async_std::sync::{Receiver, Sender};
use std::collections::HashSet;
use std::iter::FromIterator;

pub struct TargetActorHelper {
    target_id: TargetId,
    pub termination_events: Receiver<TerminationMessage>,
    pub target_invalidated_events: Receiver<TargetInvalidatedMessage>,
    pub target_actor_input_receiver: Receiver<TargetActorInputMessage>,
    target_actor_output_sender: Sender<TargetActorOutputMessage>,
    to_execute: bool,
    executed: bool,
    pub dependencies: HashSet<TargetId>,
    pub unavailable_dependencies: HashSet<TargetId>,
}

impl TargetActorHelper {
    pub fn new(
        target_metadata: &TargetMetadata,
        termination_events: Receiver<TerminationMessage>,
        target_invalidated_events: Receiver<TargetInvalidatedMessage>,
        target_actor_input_receiver: Receiver<TargetActorInputMessage>,
        target_actor_output_sender: Sender<TargetActorOutputMessage>,
    ) -> Self {
        let dependencies = HashSet::from_iter(target_metadata.dependencies.iter().cloned());
        let unavailable_dependencies = dependencies.clone();

        Self {
            target_id: target_metadata.id.clone(),
            termination_events,
            target_invalidated_events,
            target_actor_input_receiver,
            target_actor_output_sender,
            to_execute: true,
            executed: false,
            dependencies,
            unavailable_dependencies,
        }
    }

    pub async fn notify_target_invalidated(&mut self) {
        if !self.to_execute {
            self.to_execute = true;
            self.executed = false;

            let msg = TargetActorOutputMessage::TargetInvalidated(self.target_id.clone());
            self.target_actor_output_sender.send(msg).await;
        }
    }

    async fn send_target_available(&self) {
        let msg = TargetActorOutputMessage::TargetAvailable(self.target_id.clone());
        self.target_actor_output_sender.send(msg).await;
    }

    async fn send_target_execution_error(&self, e: Error) {
        let msg = TargetActorOutputMessage::TargetExecutionError(self.target_id.clone(), e);
        self.target_actor_output_sender.send(msg).await;
    }

    pub fn is_ready_to_execute(&self) -> bool {
        self.to_execute && self.unavailable_dependencies.is_empty()
    }

    pub fn set_execution_started(&mut self) {
        self.to_execute = false;
        self.executed = false;
    }

    pub async fn notify_execution_succeeded(&mut self) {
        self.executed = !self.to_execute;

        if self.executed {
            self.send_target_available().await;
        }
    }

    pub async fn notify_execution_failed(&mut self, e: Error) {
        self.executed = false;
        self.send_target_execution_error(e).await;
    }
}
