mod build_state;
mod builder;
pub mod incremental;
mod service;
mod watcher;

use crate::domain::{Target, TargetId};
use anyhow::{anyhow, Context, Result};
use build_state::TargetBuildStates;
use builder::build_target;
use crossbeam::channel::{unbounded, Receiver, Sender};
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

    pub fn watch(self, termination_events: Receiver<()>) -> Result<()> {
        let (target_invalidated_sender, target_invalidated_events) = unbounded();
        let _target_watchers = self
            .targets
            .iter()
            .map(|(target_id, target)| {
                TargetWatcher::new(target, target_invalidated_sender.clone())
                    .map(|watcher| (target_id, watcher))
            })
            .collect::<Result<HashMap<_, _>>>()
            .with_context(|| "Failed setting up filesystem watchers")?;

        let mut services_runner = ServicesRunner::new();
        let (build_report_sender, build_report_events) = unbounded();

        crossbeam::scope(|scope| -> Result<()> {
            let mut target_build_states = TargetBuildStates::new(&self.targets);

            loop {
                for target_id in target_build_states.get_ready_to_build_targets() {
                    let target = &self.targets[&target_id];
                    let termination_events = termination_events.clone();
                    let build_report_sender = build_report_sender.clone();
                    let build_thread = scope.spawn(move |_| {
                        build_target_incrementally(
                            target,
                            &termination_events,
                            &build_report_sender,
                        )
                    });
                    target_build_states.set_build_started(&target_id, build_thread);
                }

                crossbeam_channel::select! {
                    recv(target_invalidated_events) -> target_id => {
                        target_build_states.set_build_invalidated(&target_id?);
                    }
                    recv(build_report_events) -> build_report => {
                        let BuildReport { target_id, result } = build_report?;
                        let target = &self.targets[&target_id];

                        target_build_states.set_build_finished(&target_id, &result);

                        if let IncrementalRunResult::Run(Err(e)) = result {
                            log::warn!("{} - {}", target, e);
                        } else if let Target::Service(target) = target {
                            services_runner.restart_service(target)?;
                        }
                    }
                    recv(termination_events) -> _ => {
                        target_build_states.join_all_build_threads();
                        services_runner.terminate_all_services();
                        return Ok(());
                    }
                }
            }
        })
        .map_err(|_| anyhow!("Unknown crossbeam parallelism failure (thread panicked)"))?
    }

    pub fn build(
        self,
        termination_sender: Sender<()>,
        termination_events: Receiver<()>,
    ) -> Result<()> {
        let mut services_runner = ServicesRunner::new();
        let (build_report_sender, build_report_events) = unbounded();

        crossbeam::scope(|scope| {
            let mut target_build_states = TargetBuildStates::new(&self.targets);

            while !target_build_states.all_are_built() {
                for target_id in target_build_states.get_ready_to_build_targets() {
                    let target = &self.targets[&target_id];
                    let termination_events = termination_events.clone();
                    let build_report_sender = build_report_sender.clone();
                    let build_thread = scope.spawn(move |_| {
                        build_target_incrementally(
                            target,
                            &termination_events,
                            &build_report_sender,
                        )
                    });
                    target_build_states.set_build_started(&target_id, build_thread);
                }

                crossbeam_channel::select! {
                    recv(build_report_events) -> build_report => {
                        let BuildReport { target_id, result } = build_report?;

                        target_build_states.set_build_finished(&target_id, &result);

                        if let IncrementalRunResult::Run(Err(e)) = result {
                            termination_sender.send(()).with_context(|| "Sender error")?;
                            target_build_states.join_all_build_threads();
                            services_runner.terminate_all_services();
                            return Err(e);
                        }

                        let target = &self.targets[&target_id];
                        if let Target::Service(target) = target {
                            services_runner.start_service(target)?;
                        }
                    }
                    recv(termination_events) -> _ => {
                        target_build_states.join_all_build_threads();
                        services_runner.terminate_all_services();
                        return Ok(());
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
                services_runner.terminate_services(&unnecessary_services);
            }

            if !necessary_services.is_empty() {
                // Wait for termination event to terminate all services
                termination_events
                    .recv()
                    .with_context(|| "Failed to listen to termination event")?;
                log::debug!("Terminating all services");
                services_runner.terminate_all_services();
            }

            Ok(())
        })
        .map_err(|_| anyhow!("Unknown crossbeam parallelism failure (thread panicked)"))?
    }
}

fn build_target_incrementally(
    target: &Target,
    termination_events: &Receiver<()>,
    build_report_sender: &Sender<BuildReport>,
) {
    let result = incremental::run(&target, || {
        if let Target::Build(target) = target {
            build_target(&target, termination_events.clone())?;
        }

        Ok(())
    })
    .with_context(|| format!("{} - Build failed", target))
    .unwrap();

    if let IncrementalRunResult::Skipped = result {
        log::info!("{} - Build skipped (Not Modified)", target);
    }

    build_report_sender
        .send(BuildReport::new(target.id().clone(), result))
        .with_context(|| "Sender error")
        .unwrap();
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
