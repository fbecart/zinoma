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
use async_std::task;
use futures::{future, FutureExt};
use std::collections::{HashMap, HashSet};
use target_actor::{
    TargetActor, TargetActorInputMessage, TargetExecutionReportMessage, TargetInvalidatedMessage,
};
use watcher::TargetWatcher;

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
        let (target_invalidated_sender, mut target_invalidated_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);
        let _target_watchers =
            future::try_join_all(self.targets.iter().map(|(target_id, target)| {
                let target_invalidated_sender = target_invalidated_sender.clone();
                async move {
                    TargetWatcher::new(
                        target.id().clone(),
                        target.input().cloned(),
                        target_invalidated_sender,
                    )
                    .await
                    .map(|watcher| (target_id, watcher))
                }
            }))
            .await
            .with_context(|| "Failed setting up filesystem watchers")?
            .into_iter()
            .collect::<HashMap<_, _>>();

        // TODO Remove duplication
        let (target_execution_report_sender, mut target_execution_report_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        // TODO Instead, consume targets
        let mut target_actor_handles = self
            .targets
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
                    (
                        handle,
                        termination_sender,
                        target_invalidated_sender,
                        sender,
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        loop {
            futures::select! {
                _ = termination_events.next().fuse() => break,
                target_id = target_invalidated_events.next().fuse() => {
                    let target_id = target_id.unwrap();
                    let (_, _, target_invalidated_sender, _) = &target_actor_handles[&target_id];
                    if target_invalidated_sender.try_send(TargetInvalidatedMessage).is_err() {
                        log::trace!("{} - Target already invalidated. Discarding message.", target_id);
                    }
                },
                target_execution_report = target_execution_report_events.next().fuse() => {
                    match target_execution_report.unwrap() {
                        TargetExecutionReportMessage::TargetOutputAvailable(target_id) => {
                            for (_, _, _, sender) in target_actor_handles.values() {
                                sender.send(TargetActorInputMessage::TargetOutputAvailable(target_id.clone())).await;
                            }
                        }
                        TargetExecutionReportMessage::TargetExecutionError(target_id, e) => {
                            log::warn!("{} - {}", target_id, e);
                        },
                    }
                }
            }
        }

        for (_, termination_sender, _, _) in target_actor_handles.values() {
            termination_sender.send(TerminationMessage).await;
        }
        for (join_handle, _, _, _) in target_actor_handles.values_mut() {
            join_handle.await;
        }

        Ok(())
    }

    pub async fn build(self, mut termination_events: Receiver<TerminationMessage>) -> Result<()> {
        let mut unavailable_root_targets =
            self.root_target_ids.iter().cloned().collect::<HashSet<_>>();

        let (target_execution_report_sender, mut target_execution_report_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        // TODO Instead, consume targets
        let mut target_actor_handles = self
            .targets
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
                    (
                        handle,
                        termination_sender,
                        sender,
                        target_invalidated_sender,
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

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
                            for (_, _, sender, _) in target_actor_handles.values() {
                                sender.send(TargetActorInputMessage::TargetOutputAvailable(target_id.clone())).await;
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
                for (_, (_, termination_sender, _, _)) in target_actor_handles
                    .iter()
                    .filter(|(target_id, _)| !necessary_services.contains(target_id))
                {
                    termination_sender.send(TerminationMessage).await;
                }

                // Wait for termination event to terminate all services
                termination_events
                    .recv()
                    .await
                    .with_context(|| "Failed to listen to termination event".to_string())?;
            }
        }

        log::debug!("Terminating all services");
        for (_, termination_sender, _, _) in target_actor_handles.values() {
            termination_sender.send(TerminationMessage).await;
        }
        for (join_handle, _, _, _) in target_actor_handles.values_mut() {
            join_handle.await;
        }

        Ok(())
    }
}

pub struct BuildCancellationMessage;
