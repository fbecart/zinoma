mod aggregate_target_actor;
mod build_target_actor;
mod service_target_actor;

use super::watcher::{TargetInvalidatedMessage, TargetWatcher};
use crate::domain::{Target, TargetId, TargetMetadata};
use crate::TerminationMessage;
use aggregate_target_actor::AggregateTargetActor;
use anyhow::{Error, Result};
use async_std::sync::{self, Receiver, Sender};
use async_std::task::{self, JoinHandle};
use build_target_actor::BuildTargetActor;
use service_target_actor::ServiceTargetActor;
use std::collections::HashSet;
use std::iter::FromIterator;

pub enum TargetActorInputMessage {
    TargetAvailable(TargetId),
    TargetInvalidated(TargetId),
}

pub enum TargetActorOutputMessage {
    TargetExecutionError(TargetId, Error),
    TargetAvailable(TargetId),
    TargetInvalidated(TargetId),
}

pub fn launch_target_actor(
    target: Target,
    target_watcher_option: &TargetWatcherOption,
    target_actor_output_sender: Sender<TargetActorOutputMessage>,
) -> Result<TargetActorHandleSet> {
    let (termination_sender, termination_events) = sync::channel(1);
    let (target_invalidated_sender, target_invalidated_events) = sync::channel(1);
    let (target_actor_input_sender, target_actor_input_receiver) =
        sync::channel(crate::DEFAULT_CHANNEL_CAP);

    let watcher = match target_watcher_option {
        TargetWatcherOption::Enabled => TargetWatcher::new(
            target.id().clone(),
            target.input().cloned(),
            target_invalidated_sender.clone(),
        )?,
        TargetWatcherOption::Disabled => None,
    };

    let target_actor_helper = TargetActorHelper::new(
        target.metadata(),
        termination_events,
        target_invalidated_events,
        target_actor_input_receiver,
        target_actor_output_sender,
    );

    let join_handle = match target {
        Target::Build(build_target) => {
            let target_actor = BuildTargetActor::new(build_target, target_actor_helper);
            task::spawn(target_actor.run())
        }
        Target::Service(service_target) => {
            let target_actor = ServiceTargetActor::new(service_target, target_actor_helper);
            task::spawn(target_actor.run())
        }
        Target::Aggregate(aggregate_target) => {
            let target_actor = AggregateTargetActor::new(aggregate_target, target_actor_helper);
            task::spawn(target_actor.run())
        }
    };

    Ok(TargetActorHandleSet {
        join_handle,
        termination_sender,
        target_actor_input_sender,
        _target_invalidated_sender: target_invalidated_sender,
        _watcher: watcher,
    })
}

pub enum TargetWatcherOption {
    Enabled,
    Disabled,
}

pub struct TargetActorHandleSet {
    pub join_handle: JoinHandle<()>,
    pub termination_sender: Sender<TerminationMessage>,
    pub target_actor_input_sender: Sender<TargetActorInputMessage>,
    _target_invalidated_sender: Sender<TargetInvalidatedMessage>,
    _watcher: Option<TargetWatcher>,
}

pub struct TargetActorHelper {
    target_id: TargetId,
    termination_events: Receiver<TerminationMessage>,
    target_invalidated_events: Receiver<TargetInvalidatedMessage>,
    target_actor_input_receiver: Receiver<TargetActorInputMessage>,
    target_actor_output_sender: Sender<TargetActorOutputMessage>,
    to_execute: bool,
    executed: bool,
    dependencies: HashSet<TargetId>,
    unavailable_dependencies: HashSet<TargetId>,
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
