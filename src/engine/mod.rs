pub mod builder;
pub mod incremental;
mod run_state;
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
use incremental::IncrementalRunResult;
use run_state::TargetRunStates;
use service::ServicesRunner;
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
                    if let Err(_) = target_invalidated_sender.try_send(TargetInvalidatedMessage) {
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

    pub async fn watch_old(
        self,
        mut termination_events: Receiver<TerminationMessage>,
    ) -> Result<()> {
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

        let mut services_runner = ServicesRunner::new();
        let (build_report_sender, mut build_report_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        let mut target_run_states = TargetRunStates::new(&self.targets);

        loop {
            for target_id in target_run_states.list_ready_to_run_targets() {
                let target = self.targets[&target_id].clone();

                match target.clone() {
                    Target::Build(build_target) => {
                        let build_report_sender = build_report_sender.clone();
                        let (build_cancellation_sender, build_cancellation_events) =
                            sync::channel(1);
                        let build_thread = task::spawn(async move {
                            let result = incremental::run(&target, || async {
                                builder::build_target(
                                    &build_target,
                                    build_cancellation_events.clone(),
                                )
                                .await?;

                                Ok(())
                            })
                            .await
                            .with_context(|| format!("{} - Build failed", target))
                            .unwrap();

                            if let IncrementalRunResult::Skipped = result {
                                log::info!("{} - Build skipped (Not Modified)", target);
                            }

                            build_report_sender
                                .send(BuildReport::new(target.id().clone(), result))
                                .await;
                        });
                        target_run_states.set_build_started(
                            &target_id,
                            build_thread,
                            build_cancellation_sender,
                        );
                    }
                    Target::Service(service_target) => {
                        target_run_states.set_run_started(&target_id);
                        build_report_sender
                            .send(BuildReport::new(
                                target.id().clone(),
                                IncrementalRunResult::Run(Ok(())),
                            ))
                            .await;
                        services_runner.restart_service(&service_target).await?;
                    }
                    Target::Aggregate(_) => {
                        target_run_states.set_run_started(&target_id);
                        build_report_sender
                            .send(BuildReport::new(
                                target.id().clone(),
                                IncrementalRunResult::Run(Ok(())),
                            ))
                            .await;
                    }
                }
            }

            futures::select! {
                _ = termination_events.next().fuse() => {
                    target_run_states.cancel_all_builds().await;
                    services_runner.terminate_all_services().await;
                    return Ok(());
                },
                target_id = target_invalidated_events.next().fuse() => {
                    target_run_states.set_invalidated(&target_id.unwrap());
                },
                build_report = build_report_events.next().fuse() => {
                    let BuildReport { target_id, result } = build_report.unwrap();

                    target_run_states.set_finished(&target_id, &result).await;

                    if let IncrementalRunResult::Run(Err(e)) = result {
                        log::warn!("{} - {}", target_id, e);
                    }
                },
            }
        }
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

    pub async fn build_old(
        self,
        mut termination_events: Receiver<TerminationMessage>,
    ) -> Result<()> {
        let mut services_runner = ServicesRunner::new();
        let (build_report_sender, mut build_report_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        let mut target_run_states = TargetRunStates::new(&self.targets);

        while !target_run_states.all_are_built() {
            for target_id in target_run_states.list_ready_to_run_targets() {
                let target = self.targets[&target_id].clone();

                match target.clone() {
                    Target::Build(build_target) => {
                        let build_report_sender = build_report_sender.clone();
                        let (build_cancellation_sender, build_cancellation_events) =
                            sync::channel(1);
                        let build_thread = task::spawn(async move {
                            let result = incremental::run(&target, || async {
                                builder::build_target(
                                    &build_target,
                                    build_cancellation_events.clone(),
                                )
                                .await?;

                                Ok(())
                            })
                            .await
                            .with_context(|| format!("{} - Build failed", target))
                            .unwrap();

                            if let IncrementalRunResult::Skipped = result {
                                log::info!("{} - Build skipped (Not Modified)", target);
                            }

                            build_report_sender
                                .send(BuildReport::new(target.id().clone(), result))
                                .await;
                        });
                        target_run_states.set_build_started(
                            &target_id,
                            build_thread,
                            build_cancellation_sender,
                        );
                    }
                    Target::Service(service_target) => {
                        target_run_states.set_run_started(&target_id);
                        build_report_sender
                            .send(BuildReport::new(
                                target.id().clone(),
                                IncrementalRunResult::Run(Ok(())),
                            ))
                            .await;
                        services_runner.start_service(&service_target).await?;
                    }
                    Target::Aggregate(_) => {
                        target_run_states.set_run_started(&target_id);
                        build_report_sender
                            .send(BuildReport::new(
                                target.id().clone(),
                                IncrementalRunResult::Run(Ok(())),
                            ))
                            .await;
                    }
                }
            }

            futures::select! {
                _ = termination_events.next().fuse() => {
                    target_run_states.cancel_all_builds().await;
                    services_runner.terminate_all_services().await;
                    return Ok(());
                }
                build_report = build_report_events.next().fuse() => {
                    let BuildReport { target_id, result } = build_report.unwrap();

                    target_run_states.set_finished(&target_id, &result).await;

                    if let IncrementalRunResult::Run(Err(e)) = result {
                        target_run_states.cancel_all_builds().await;
                        services_runner.terminate_all_services().await;
                        return Err(e);
                    }
                }
            }
        }

        let necessary_services =
            service::get_service_graph_targets(&self.targets, &self.root_target_ids);
        let unnecessary_services = services_runner
            .list_running_services()
            .iter()
            .filter(|target_id| !necessary_services.contains(target_id))
            .cloned()
            .collect::<Vec<_>>();

        if !unnecessary_services.is_empty() {
            log::debug!("Terminating unnecessary services");
            services_runner
                .terminate_services(&unnecessary_services)
                .await;
        }

        if !necessary_services.is_empty() {
            // Wait for termination event to terminate all services
            termination_events
                .recv()
                .await
                .with_context(|| "Failed to listen to termination event".to_string())?;
            log::debug!("Terminating all services");
            services_runner.terminate_all_services().await;
        }

        Ok(())
    }
}

pub struct BuildReport {
    pub target_id: TargetId,
    pub result: IncrementalRunResult<Result<()>>,
}

impl BuildReport {
    pub fn new(target_id: TargetId, result: IncrementalRunResult<Result<()>>) -> Self {
        Self { target_id, result }
    }
}

pub enum BuildCancellationMessage {
    CancelBuild,
}
