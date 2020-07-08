pub mod builder;
pub mod incremental;
mod service;
mod target_actor;
mod watcher;

use crate::domain::{Target, TargetId};
use crate::TerminationMessage;
use anyhow::{Context, Result};
use async_std::prelude::*;
use async_std::sync::{self, Receiver, Sender};
use async_std::task;
use futures::{future, FutureExt};
use std::collections::{HashMap, HashSet};
use target_actor::{TargetActor, TargetActorInputMessage, TargetExecutionReportMessage};
use task::JoinHandle;
use watcher::{TargetInvalidatedMessage, TargetWatcher};

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
        let (target_actor_handles, mut target_execution_report_events) =
            Self::launch_target_actors(&self.targets);

        let _target_watchers =
            future::try_join_all(self.targets.iter().map(|(target_id, target)| {
                let handles = &target_actor_handles[target_id];
                async move {
                    TargetWatcher::new(
                        target.id().clone(),
                        target.input().cloned(),
                        handles.target_invalidated_sender.clone(),
                    )
                    .await
                    .map(|watcher| (target_id, watcher))
                }
            }))
            .await
            .with_context(|| "Failed setting up filesystem watchers")?
            .into_iter()
            .collect::<HashMap<_, _>>();

        loop {
            futures::select! {
                _ = termination_events.next().fuse() => break,
                target_execution_report = target_execution_report_events.next().fuse() => {
                    match target_execution_report.unwrap() {
                        TargetExecutionReportMessage::TargetOutputAvailable(target_id) => {
                            for handles in target_actor_handles.values() {
                                handles.sender.send(TargetActorInputMessage::TargetOutputAvailable(target_id.clone())).await;
                            }
                        }
                        TargetExecutionReportMessage::TargetExecutionError(target_id, e) => {
                            log::warn!("{} - {}", target_id, e);
                        },
                    }
                }
            }
        }

        for handles in target_actor_handles.values() {
            handles.termination_sender.send(TerminationMessage).await;
        }
        for (_target_id, handles) in target_actor_handles.into_iter() {
            handles.join_handle.await;
        }

        Ok(())
    }

    pub async fn build(self, mut termination_events: Receiver<TerminationMessage>) -> Result<()> {
        let mut unavailable_root_targets =
            self.root_target_ids.iter().cloned().collect::<HashSet<_>>();

        let (target_actor_handles, mut target_execution_report_events) =
            Self::launch_target_actors(&self.targets);

        let mut terminating = false;

        while !unavailable_root_targets.is_empty() && !terminating {
            futures::select! {
                _ = termination_events.next().fuse() => {
                    terminating = true
                },
                target_execution_report = target_execution_report_events.next().fuse() => {
                    match target_execution_report.unwrap() {
                        TargetExecutionReportMessage::TargetOutputAvailable(target_id) => {
                            unavailable_root_targets.remove(&target_id);
                            for handles in target_actor_handles.values() {
                                handles.sender.send(TargetActorInputMessage::TargetOutputAvailable(target_id.clone())).await;
                            }
                        }
                        TargetExecutionReportMessage::TargetExecutionError(target_id, e) => {
                            // TODO Log here? Or already done?
                            terminating = true
                        },
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

                // Wait for termination event to terminate all services
                termination_events
                    .recv()
                    .await
                    .with_context(|| "Failed to listen to termination event".to_string())?;
            }
        }

        log::debug!("Terminating all services");
        for handles in target_actor_handles.values() {
            handles.termination_sender.send(TerminationMessage).await;
        }
        for (_target_id, handles) in target_actor_handles.into_iter() {
            handles.join_handle.await;
        }

        Ok(())
    }

    fn launch_target_actors(
        targets: &HashMap<TargetId, Target>,
    ) -> (
        HashMap<TargetId, TargetActorHandleSet>,
        Receiver<TargetExecutionReportMessage>,
    ) {
        let (target_execution_report_sender, target_execution_report_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        // TODO Instead, consume targets
        let target_actor_handles = targets
            .iter()
            .map(|(target_id, target)| {
                let (termination_sender, termination_events) = sync::channel(1);
                let (target_invalidated_sender, target_invalidated_events) = sync::channel(1);
                let (sender, receiver) = sync::channel(crate::DEFAULT_CHANNEL_CAP);
                let target_actor = TargetActor::new(
                    target.clone(),
                    termination_events,
                    target_invalidated_events,
                    receiver,
                    target_execution_report_sender.clone(),
                );
                let handle = task::spawn(target_actor.run());
                (
                    target_id.clone(),
                    TargetActorHandleSet::new(
                        handle,
                        termination_sender,
                        target_invalidated_sender,
                        sender,
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        (target_actor_handles, target_execution_report_events)
    }
}

pub struct BuildCancellationMessage;

pub struct TargetActorHandleSet {
    join_handle: JoinHandle<()>,
    termination_sender: Sender<TerminationMessage>,
    target_invalidated_sender: Sender<TargetInvalidatedMessage>,
    // TODO Rename
    sender: Sender<TargetActorInputMessage>,
}

impl TargetActorHandleSet {
    pub fn new(
        join_handle: JoinHandle<()>,
        termination_sender: Sender<TerminationMessage>,
        target_invalidated_sender: Sender<TargetInvalidatedMessage>,
        sender: Sender<TargetActorInputMessage>,
    ) -> Self {
        Self {
            join_handle,
            termination_sender,
            target_invalidated_sender,
            sender,
        }
    }
}
