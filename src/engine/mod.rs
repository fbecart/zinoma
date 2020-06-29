mod build_state;
mod builder;
pub mod incremental;
mod service;
mod watcher;

use crate::domain::{Target, TargetId};
use anyhow::{Context, Result};
use async_std::prelude::*;
use async_std::sync::{self, Receiver, Sender};
use async_std::task;
use build_state::TargetBuildStates;
use builder::build_target;
use futures::{future, FutureExt};
use incremental::IncrementalRunResult;
use service::ServicesRunner;
use std::collections::HashMap;
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

    pub async fn watch(self, mut termination_events: Receiver<()>) -> Result<()> {
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

        let mut target_build_states = TargetBuildStates::new(&self.targets);

        loop {
            for target_id in target_build_states.get_ready_to_build_targets() {
                let target = self.targets[&target_id].clone();
                let termination_events = termination_events.clone();
                let build_report_sender = build_report_sender.clone();
                let build_thread = task::spawn(async move {
                    build_target_incrementally(&target, &termination_events, &build_report_sender)
                        .await
                });
                target_build_states.set_build_started(&target_id, build_thread);
            }

            futures::select! {
                _ = termination_events.next().fuse() => {
                    target_build_states.join_all_build_threads().await;
                    services_runner.terminate_all_services().await;
                    return Ok(());
                },
                target_id = target_invalidated_events.next().fuse() => {
                    target_build_states.set_build_invalidated(&target_id.unwrap());
                },
                build_report = build_report_events.next().fuse() => {
                    let BuildReport { target_id, result } = build_report.unwrap();
                    let target = &self.targets[&target_id];

                    target_build_states.set_build_finished(&target_id, &result).await;

                    if let IncrementalRunResult::Run(Err(e)) = result {
                        log::warn!("{} - {}", target, e);
                    } else if let Target::Service(target) = target {
                        services_runner.restart_service(target).await?;
                    }
                },
            }
        }
    }

    pub async fn build(
        self,
        termination_sender: Sender<()>,
        mut termination_events: Receiver<()>,
    ) -> Result<()> {
        let mut services_runner = ServicesRunner::new();
        let (build_report_sender, mut build_report_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        let mut target_build_states = TargetBuildStates::new(&self.targets);

        while !target_build_states.all_are_built() {
            for target_id in target_build_states.get_ready_to_build_targets() {
                let target = self.targets[&target_id].clone();
                let termination_events = termination_events.clone();
                let build_report_sender = build_report_sender.clone();
                let build_thread = task::spawn(async move {
                    build_target_incrementally(&target, &termination_events, &build_report_sender)
                        .await
                });
                target_build_states.set_build_started(&target_id, build_thread);
            }

            futures::select! {
                _ = termination_events.next().fuse() => {
                    target_build_states.join_all_build_threads().await;
                    services_runner.terminate_all_services().await;
                    return Ok(());
                }
                build_report = build_report_events.next().fuse() => {
                    let BuildReport { target_id, result } = build_report.unwrap();

                    log::debug!("{} - Build report received", target_id);
                    target_build_states.set_build_finished(&target_id, &result).await;

                    if let IncrementalRunResult::Run(Err(e)) = result {
                        termination_sender.send(()).await;
                        target_build_states.join_all_build_threads().await;
                        services_runner.terminate_all_services().await;
                        return Err(e);
                    }

                    let target = &self.targets[&target_id];
                    if let Target::Service(target) = target {
                        services_runner.start_service(target).await?;
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

async fn build_target_incrementally(
    target: &Target,
    termination_events: &Receiver<()>,
    build_report_sender: &Sender<BuildReport>,
) {
    let result = incremental::run(&target, || async {
        if let Target::Build(target) = target {
            build_target(&target, termination_events.clone()).await?;
        }

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
