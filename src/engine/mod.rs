mod builder;
pub mod incremental;
mod target_actor;
mod target_actors;
mod watcher;

use crate::domain::TargetId;
use crate::TerminationMessage;
use anyhow::{Context, Result};
use async_std::channel::Receiver;
use async_std::prelude::*;
use futures::FutureExt;
use std::collections::HashSet;
use target_actor::{ActorId, ActorInputMessage, ExecutionKind, TargetActorOutputMessage};
pub use target_actors::TargetActors;

pub async fn run(
    root_target_ids: Vec<TargetId>,
    watch_option: WatchOption,
    mut target_actors: &mut TargetActors,
    termination_events: Receiver<TerminationMessage>,
    target_actor_output_events: Receiver<TargetActorOutputMessage>,
) -> Result<()> {
    for target_id in &root_target_ids {
        target_actors.request_target(target_id).await?;
    }

    match watch_option {
        WatchOption::Enabled => {
            watch(
                &mut target_actors,
                termination_events,
                target_actor_output_events,
            )
            .await
        }
        WatchOption::Disabled => {
            execute_once(
                &root_target_ids,
                &mut target_actors,
                termination_events,
                target_actor_output_events,
            )
            .await
        }
    }
}

async fn watch(
    target_actors: &mut TargetActors,
    mut termination_events: Receiver<TerminationMessage>,
    mut target_actor_output_events: Receiver<TargetActorOutputMessage>,
) -> Result<()> {
    loop {
        futures::select! {
            _ = termination_events.next().fuse() => break,
            target_actor_output = target_actor_output_events.next().fuse() => {
                match target_actor_output.unwrap() {
                    TargetActorOutputMessage::TargetExecutionError(target_id, e) => {
                        log::warn!("{} - {}", target_id, e);
                    },
                    TargetActorOutputMessage::MessageActor { dest, msg } => {
                        if let ActorId::Target(target_id) = dest {
                            target_actors.send(&target_id, msg).await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn execute_once(
    root_target_ids: &[TargetId],
    target_actors: &mut TargetActors,
    mut termination_events: Receiver<TerminationMessage>,
    mut target_actor_output_events: Receiver<TargetActorOutputMessage>,
) -> Result<()> {
    let unavailable_root_targets = root_target_ids.iter().cloned().collect::<HashSet<_>>();
    let mut unavailable_root_builds = unavailable_root_targets.clone();
    let mut unavailable_root_services = unavailable_root_targets;
    let mut service_root_targets = HashSet::new();
    let mut termination_event_received = false;

    while !(termination_event_received
        || unavailable_root_services.is_empty() && unavailable_root_builds.is_empty())
    {
        futures::select! {
            _ = termination_events.next().fuse() => termination_event_received = true,
            target_actor_output = target_actor_output_events.next().fuse() => {
                match target_actor_output.unwrap() {
                    TargetActorOutputMessage::TargetExecutionError(target_id, e) => {
                        return Err(e.context(format!("An issue occurred with target {}", target_id)));
                    },
                    TargetActorOutputMessage::MessageActor { dest, msg } => match dest {
                        ActorId::Target(target_id) => {
                            target_actors.send(&target_id, msg).await?;
                        }
                        ActorId::Root => match msg {
                            ActorInputMessage::Ok { kind: ExecutionKind::Build, target_id, .. } => {
                                unavailable_root_builds.remove(&target_id);
                            },
                            ActorInputMessage::Ok { kind: ExecutionKind::Service, target_id, actual } => {
                                unavailable_root_services.remove(&target_id);

                                if actual {
                                    service_root_targets.insert(target_id);
                                }
                            },
                            _ => {},
                        }
                    }
                }
            }
        }
    }

    if !termination_event_received && !service_root_targets.is_empty() {
        // Wait for termination event
        termination_events
            .recv()
            .await
            .with_context(|| "Failed to listen to termination event".to_string())?;
    }

    Ok(())
}
#[derive(Copy, Clone)]
pub enum WatchOption {
    Enabled,
    Disabled,
}

impl From<bool> for WatchOption {
    fn from(value: bool) -> Self {
        if value {
            WatchOption::Enabled
        } else {
            WatchOption::Disabled
        }
    }
}
