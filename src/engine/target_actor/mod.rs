mod aggregate_target_actor;
mod build_target_actor;
mod service_target_actor;
mod target_actor_helper;

use super::watcher::{TargetInvalidatedMessage, TargetWatcher};
use crate::domain::{Target, TargetId};
use crate::TerminationMessage;
use aggregate_target_actor::AggregateTargetActor;
use anyhow::{Error, Result};
use async_std::sync::{self, Sender};
use async_std::task::{self, JoinHandle};
use build_target_actor::BuildTargetActor;
use service_target_actor::ServiceTargetActor;
use target_actor_helper::TargetActorHelper;

#[derive(Debug, Clone)]
pub enum ActorInputMessage {
    /// Indicates the execution of the build scripts behind this target are requested.
    ///
    /// This message should only be sent to direct dependencies.
    BuildRequested { requester: ActorId },
    /// Indicates the execution of the services behind this target are requested.
    ///
    /// This message should only be sent to direct dependencies.
    ServiceRequested { requester: ActorId },
    /// Indicates the execution of the build scripts behind this target are no more requested by the provided requester.
    ///
    /// This message should only be sent to direct dependencies.
    BuildUnrequested { requester: ActorId },
    /// Indicates the execution of the services behind this target are no more requested by the provided requester.
    ///
    /// This message should only be sent to direct dependencies.
    ServiceUnrequested { requester: ActorId },
    /// Indicates the execution of the build scripts behind the provided target are OK.
    ///
    /// Here, OK means one of the following:
    /// - There is no build script behind this target;
    /// - All build scripts have been executed or skipped, and therefore, their output resources are available.
    ///
    /// This message should only be sent to build requesters.
    BuildOk { target_id: TargetId },
    /// Indicates the execution of the services behind the provided target are OK.
    ///
    /// Here, OK means one of the following:
    /// - There is no service behind this target;
    /// - All services have been started and are currently running.
    ///
    /// This message should only be sent to service requesters.
    ServiceOk {
        target_id: TargetId,
        has_service: bool,
    },
    /// Indicates the build scripts behind the target are not OK anymore.
    ///
    /// The requester should invalidate the previously sent [`BuildOk`].
    ///
    /// [`BuildOk`]: #variant.BuildOk
    BuildInvalidated { target_id: TargetId },
    /// Indicates the services behind the target are not OK anymore.
    ///
    /// The requester should invalidate the previously sent [`ServiceOk`].
    ///
    /// [`ServiceOk`]: #variant.ServiceOk
    ServiceInvalidated { target_id: TargetId },
}

pub enum TargetActorOutputMessage {
    TargetExecutionError(TargetId, Error),
    MessageActor {
        dest: ActorId,
        msg: ActorInputMessage,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActorId {
    Root,
    Target(TargetId),
}

pub fn launch_target_actor(
    target: Target,
    target_watcher_option: TargetWatcherOption,
    target_actor_output_sender: Sender<TargetActorOutputMessage>,
) -> Result<(JoinHandle<()>, TargetActorHandleSet)> {
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

    Ok((
        join_handle,
        TargetActorHandleSet {
            termination_sender,
            target_actor_input_sender,
            _target_invalidated_sender: target_invalidated_sender,
            _watcher: watcher,
        },
    ))
}

#[derive(Copy, Clone)]
pub enum TargetWatcherOption {
    Enabled,
    Disabled,
}

pub struct TargetActorHandleSet {
    pub termination_sender: Sender<TerminationMessage>,
    pub target_actor_input_sender: Sender<ActorInputMessage>,
    _target_invalidated_sender: Sender<TargetInvalidatedMessage>,
    _watcher: Option<TargetWatcher>,
}
