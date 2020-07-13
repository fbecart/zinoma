use super::{ActorId, ActorInputMessage, ExecutionKind, TargetActorOutputMessage};
use crate::domain::{TargetId, TargetMetadata};
use crate::engine::watcher::TargetInvalidatedMessage;
use crate::TerminationMessage;
use anyhow::Error;
use async_std::sync::{Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

pub struct TargetActorHelper {
    pub target_id: TargetId,
    pub termination_events: Receiver<TerminationMessage>,
    pub target_invalidated_events: Receiver<TargetInvalidatedMessage>,
    pub target_actor_input_receiver: Receiver<ActorInputMessage>,
    pub target_actor_output_sender: Sender<TargetActorOutputMessage>,
    pub to_execute: bool,
    pub executed: bool,
    pub dependencies: Vec<TargetId>,
    pub unavailable_dependencies: HashMap<ExecutionKind, HashSet<TargetId>>,
    pub requesters: HashMap<ExecutionKind, HashSet<ActorId>>,
}

impl TargetActorHelper {
    pub fn new(
        target_metadata: &TargetMetadata,
        termination_events: Receiver<TerminationMessage>,
        target_invalidated_events: Receiver<TargetInvalidatedMessage>,
        target_actor_input_receiver: Receiver<ActorInputMessage>,
        target_actor_output_sender: Sender<TargetActorOutputMessage>,
    ) -> Self {
        let dependencies = target_metadata.dependencies.clone();

        let mut unavailable_dependencies = HashMap::new();
        let dependencies_set = HashSet::from_iter(dependencies.iter().cloned());
        unavailable_dependencies.insert(ExecutionKind::Build, dependencies_set.clone());
        unavailable_dependencies.insert(ExecutionKind::Service, dependencies_set);

        let mut requesters = HashMap::new();
        requesters.insert(ExecutionKind::Build, HashSet::new());
        requesters.insert(ExecutionKind::Service, HashSet::new());

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
            requesters,
        }
    }

    pub async fn notify_invalidated(&mut self, kind: ExecutionKind) {
        if !self.to_execute {
            self.to_execute = true;
            self.executed = false;

            let target_id = self.target_id.clone();
            let msg = ActorInputMessage::Invalidated { kind, target_id };
            self.send_to_requesters(kind, msg).await
        }
    }

    pub fn set_execution_started(&mut self) {
        self.to_execute = false;
        self.executed = false;
    }

    pub async fn notify_execution_failed(&mut self, e: Error) {
        self.executed = false;
        let msg = TargetActorOutputMessage::TargetExecutionError(self.target_id.clone(), e);
        self.target_actor_output_sender.send(msg).await;
    }

    pub async fn send_to_actor(&self, dest: ActorId, msg: ActorInputMessage) {
        self.target_actor_output_sender
            .send(TargetActorOutputMessage::MessageActor { dest, msg })
            .await
    }

    pub async fn send_to_dependencies(&self, msg: ActorInputMessage) {
        for dependency in &self.dependencies {
            self.send_to_actor(ActorId::Target(dependency.clone()), msg.clone())
                .await
        }
    }

    pub async fn send_to_requesters(&self, kind: ExecutionKind, msg: ActorInputMessage) {
        for requester in &self.requesters[&kind] {
            self.send_to_actor(requester.clone(), msg.clone()).await
        }
    }
}
