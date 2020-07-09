mod aggregate_target_actor;
mod build_target_actor;
mod service_target_actor;

use super::watcher::{TargetInvalidatedMessage, TargetWatcher};
use crate::domain::{Target, TargetId};
use crate::TerminationMessage;
use anyhow::{Error, Result};
use async_std::sync::{self, Sender};
use async_std::task;
use async_std::task::JoinHandle;

pub enum TargetActorInputMessage {
    TargetAvailable(TargetId),
    TargetInvalidated(TargetId),
}

pub enum TargetActorOutputMessage {
    TargetExecutionError(TargetId, Error),
    TargetAvailable(TargetId),
    TargetInvalidated(TargetId),
}

enum TargetExecutionResult {
    InterruptedByTermination,
    Success,
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

    let join_handle = match target {
        Target::Build(_) => {
            let target_actor = build_target_actor::TargetActor::new(
                target,
                termination_events,
                target_invalidated_events,
                target_actor_input_receiver,
                target_actor_output_sender,
            );
            task::spawn(target_actor.run())
        }
        Target::Service(_) => {
            let target_actor = service_target_actor::TargetActor::new(
                target,
                termination_events,
                target_invalidated_events,
                target_actor_input_receiver,
                target_actor_output_sender,
            );
            task::spawn(target_actor.run())
        }
        Target::Aggregate(_) => {
            let target_actor = aggregate_target_actor::TargetActor::new(
                target,
                termination_events,
                target_invalidated_events,
                target_actor_input_receiver,
                target_actor_output_sender,
            );
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
