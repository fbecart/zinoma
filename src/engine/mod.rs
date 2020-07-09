pub mod builder;
pub mod incremental;
mod service;
mod target_actor;
mod watcher;

use crate::domain::{Target, TargetId};
use crate::TerminationMessage;
use anyhow::{Context, Result};
use async_std::prelude::*;
use async_std::sync::{self, Receiver};
use futures::FutureExt;
use std::collections::{HashMap, HashSet};
use target_actor::{
    TargetActorHandleSet, TargetActorInputMessage, TargetActorOutputMessage, TargetWatcherOption,
};

pub struct Engine {
    targets: HashMap<TargetId, Target>,
    root_target_ids: Vec<TargetId>,
}

impl Engine {
    pub fn new(targets: HashMap<TargetId, Target>, root_target_ids: Vec<TargetId>) -> Self {
        Self {
            targets,
            root_target_ids,
        }
    }

    pub async fn watch(self, mut termination_events: Receiver<TerminationMessage>) -> Result<()> {
        let (target_actor_handles, mut target_actor_output_events) =
            Self::launch_target_actors(&self.targets, TargetWatcherOption::Enabled)?;

        loop {
            futures::select! {
                _ = termination_events.next().fuse() => break,
                target_actor_output = target_actor_output_events.next().fuse() => {
                    match target_actor_output.unwrap() {
                        TargetActorOutputMessage::TargetAvailable(target_id) => {
                            for handles in target_actor_handles.values() {
                                handles.target_actor_input_sender.send(TargetActorInputMessage::TargetAvailable(target_id.clone())).await;
                            }
                        }
                        TargetActorOutputMessage::TargetExecutionError(target_id, e) => {
                            log::warn!("{} - {}", target_id, e); // FIXME Better logs at least?
                        },
                        TargetActorOutputMessage::TargetInvalidated(target_id) => {
                            for handles in target_actor_handles.values() {
                                handles.target_actor_input_sender.send(TargetActorInputMessage::TargetInvalidated(target_id.clone())).await;
                            }
                        }
                    }
                }
            }
        }

        Self::terminate_target_actors(target_actor_handles).await;

        Ok(())
    }

    pub async fn build(self, mut termination_events: Receiver<TerminationMessage>) -> Result<()> {
        let (target_actor_handles, mut target_actor_output_events) =
            Self::launch_target_actors(&self.targets, TargetWatcherOption::Disabled)?;

        let mut unavailable_root_targets =
            self.root_target_ids.iter().cloned().collect::<HashSet<_>>();
        let mut terminating = false;

        while !unavailable_root_targets.is_empty() && !terminating {
            futures::select! {
                _ = termination_events.next().fuse() => {
                    terminating = true
                },
                target_actor_output = target_actor_output_events.next().fuse() => {
                    match target_actor_output.unwrap() {
                        TargetActorOutputMessage::TargetAvailable(target_id) => {
                            unavailable_root_targets.remove(&target_id);
                            for handles in target_actor_handles.values() {
                                handles.target_actor_input_sender.send(TargetActorInputMessage::TargetAvailable(target_id.clone())).await;
                            }
                        }
                        TargetActorOutputMessage::TargetExecutionError(target_id, e) => {
                            // TODO Log here? Or already done?
                            terminating = true
                        },
                        TargetActorOutputMessage::TargetInvalidated(_) => unreachable!("Watcher is disabled in build mode"),
                    }
                }
            }
        }

        if !terminating {
            let necessary_services =
                service::get_service_graph_targets(&self.targets, &self.root_target_ids);

            if !necessary_services.is_empty() {
                for (_, handles) in target_actor_handles
                    .iter()
                    .filter(|(target_id, _)| !necessary_services.contains(target_id))
                {
                    handles.termination_sender.send(TerminationMessage).await;
                }

                // Wait for termination event
                termination_events
                    .recv()
                    .await
                    .with_context(|| "Failed to listen to termination event".to_string())?;
            }
        }

        Self::terminate_target_actors(target_actor_handles).await;

        Ok(())
    }

    fn launch_target_actors(
        targets: &HashMap<TargetId, Target>,
        target_watcher_option: TargetWatcherOption,
    ) -> Result<(
        HashMap<TargetId, TargetActorHandleSet>,
        Receiver<TargetActorOutputMessage>,
    )> {
        let (target_actor_output_sender, target_actor_output_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        // TODO Instead, consume targets
        let mut target_actor_handles = HashMap::with_capacity(targets.len());
        for (target_id, target) in targets {
            target_actor_handles.insert(
                target_id.clone(),
                target_actor::launch_target_actor(
                    target.clone(), // TODO Remove clone
                    &target_watcher_option,
                    target_actor_output_sender.clone(),
                )?,
            );
        }

        Ok((target_actor_handles, target_actor_output_events))
    }

    async fn terminate_target_actors(
        target_actor_handles: HashMap<TargetId, TargetActorHandleSet>,
    ) {
        log::debug!("Terminating all services");
        for handles in target_actor_handles.values() {
            handles.termination_sender.send(TerminationMessage).await;
        }
        for (_target_id, handles) in target_actor_handles.into_iter() {
            handles.join_handle.await;
        }
    }
}
