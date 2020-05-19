mod build_state;
mod builder;
pub mod incremental;
mod process;
mod service;
mod watcher;

use crate::domain::{Target, TargetId};
use anyhow::{Context, Result};
use build_state::TargetBuildStates;
use builder::build_target;
use crossbeam::channel::{unbounded, Receiver, Sender};
use incremental::IncrementalRunResult;
use service::ServicesRunner;
use watcher::TargetWatcher;

pub struct Engine {
    targets: Vec<Target>,
}

impl Engine {
    pub fn new(targets: Vec<Target>) -> Self {
        Self { targets }
    }

    pub fn watch(self, termination_events: Receiver<()>) -> Result<()> {
        let (target_invalidated_sender, target_invalidated_events) = unbounded();
        let _target_watchers = self
            .targets
            .iter()
            .map(|target| TargetWatcher::new(target, target_invalidated_sender.clone()))
            .collect::<Result<Vec<_>>>()
            .with_context(|| "Failed setting up filesystem watchers");

        let mut services_runner = ServicesRunner::new(&self.targets);
        let (build_report_sender, build_report_events) = unbounded();

        crossbeam::scope(|scope| -> Result<()> {
            let mut target_build_states = TargetBuildStates::new(&self.targets);

            loop {
                for &target_id in &target_build_states.get_ready_to_build_targets() {
                    let target = &self.targets[target_id];
                    let termination_events = termination_events.clone();
                    let build_report_sender = build_report_sender.clone();
                    let build_thread = scope.spawn(move |_| {
                        build_target_incrementally(
                            target,
                            &termination_events,
                            &build_report_sender,
                        )
                    });
                    target_build_states.set_build_started(target_id, build_thread);
                }

                crossbeam_channel::select! {
                    recv(target_invalidated_events) -> target_id => {
                        target_build_states.set_build_invalidated(target_id?);
                    }
                    recv(build_report_events) -> build_report => {
                        let BuildReport { target_id, result } = build_report?;
                        let target = &self.targets[target_id];

                        target_build_states.set_build_finished(target_id, &result);

                        if let IncrementalRunResult::Run(Err(e)) = result {
                            log::warn!("{} - {}", target.name, e);
                        } else {
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
        .map_err(|_| anyhow::anyhow!("Unknown crossbeam parallelism failure (thread panicked)"))?
    }

    pub fn build(
        self,
        termination_sender: Sender<()>,
        termination_events: Receiver<()>,
    ) -> Result<()> {
        let mut services_runner = ServicesRunner::new(&self.targets);
        let (build_report_sender, build_report_events) = unbounded();

        crossbeam::scope(|scope| {
            let mut target_build_states = TargetBuildStates::new(&self.targets);

            while !target_build_states.all_are_built() {
                for &target_id in &target_build_states.get_ready_to_build_targets() {
                    let target = &self.targets[target_id];
                    let termination_events = termination_events.clone();
                    let build_report_sender = build_report_sender.clone();
                    let build_thread = scope.spawn(move |_| {
                        build_target_incrementally(
                            target,
                            &termination_events,
                            &build_report_sender,
                        )
                    });
                    target_build_states.set_build_started(target_id, build_thread);
                }

                crossbeam_channel::select! {
                    recv(build_report_events) -> build_report => {
                        let BuildReport { target_id, result } = build_report?;

                        target_build_states.set_build_finished(target_id, &result);

                        if let IncrementalRunResult::Run(Err(e)) = result {
                            termination_sender.send(()).with_context(|| "Sender error")?;
                            target_build_states.join_all_build_threads();
                            services_runner.terminate_all_services();
                            return Err(e);
                        }

                        let target = &self.targets[target_id];
                        services_runner.start_service(target)?;
                    }
                    recv(termination_events) -> _ => {
                        target_build_states.join_all_build_threads();
                        services_runner.terminate_all_services();
                        return Ok(());
                    }
                }
            }

            if services_runner.has_running_services() {
                termination_events
                    .recv()
                    .with_context(|| "Failed to listen to termination event")?;
                target_build_states.join_all_build_threads();
                services_runner.terminate_all_services();
            }

            Ok(())
        })
        .map_err(|_| anyhow::anyhow!("Unknown crossbeam parallelism failure (thread panicked)"))?
    }
}

fn build_target_incrementally(
    target: &Target,
    termination_events: &Receiver<()>,
    build_report_sender: &Sender<BuildReport>,
) {
    let result = incremental::run(&target, || {
        build_target(&target, termination_events.clone())
    })
    .with_context(|| format!("{} - Build failed", target.name))
    .unwrap();

    if let IncrementalRunResult::Skipped = result {
        log::info!("{} - Build skipped (Not Modified)", target.name);
    }

    build_report_sender
        .send(BuildReport::new(target.id, result))
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
